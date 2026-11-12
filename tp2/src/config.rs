use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

// The flat database keys match the existing, ignored repository config.json.
// Do not derive Debug: this structure contains a database password.
/// Shared repository configuration, with optional TP2 serial settings and limits.
///
/// Database credentials are read from the ignored `config.json` at runtime.
#[derive(Deserialize)]
pub struct Config {
    #[serde(alias = "db_host")]
    pub hostname: String,
    #[serde(default = "default_port")]
    pub db_port: u16,
    pub db_name: String,
    pub db_writeonly_user: String,
    pub db_writeonly_password: String,
    #[serde(default)]
    pub serial: SerialConfig,
    #[serde(default)]
    pub limits: Limits,
    #[serde(default = "default_log")]
    pub log_file: PathBuf,
    #[serde(default = "default_failure_file")]
    pub failure_file: PathBuf,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    pub reconnect_seconds: u64,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            port: "/dev/ttyUSB0".into(),
            baud_rate: 9600,
            reconnect_seconds: 2,
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub temperature: [f64; 2],
    pub humidity: [f64; 2],
    pub pressure: [f64; 2],
    pub co2: [f64; 2],
    pub gas_raw: [f64; 2],
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            temperature: [0.0, 50.0],
            humidity: [5.0, 95.0],
            pressure: [900.0, 1100.0],
            co2: [400.0, 5000.0],
            gas_raw: [1.0, 1022.0],
        }
    }
}

fn default_port() -> u16 {
    3306
}
fn default_log() -> PathBuf {
    "var/tp2.log".into()
}
fn default_failure_file() -> PathBuf {
    "var/failed-measurements.jsonl".into()
}

impl Config {
    /// Load configuration and resolve log paths relative to its directory.
    /// Parse errors omit configuration values to avoid exposing credentials.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("cannot read configuration {}", path.display()))?;
        // Serde errors may include an invalid scalar value. Never print it from a secret file.
        let mut config: Self = serde_json::from_slice(&bytes).map_err(|error| {
            anyhow::anyhow!(
                "invalid configuration at line {}, column {}",
                error.line(),
                error.column()
            )
        })?;
        config.validate()?;
        let base = path.parent().unwrap_or(Path::new("."));
        for output in [&mut config.log_file, &mut config.failure_file] {
            if output.is_relative() {
                *output = base.join(&*output);
            }
        }
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        for (name, value) in [
            ("hostname", &self.hostname),
            ("db_name", &self.db_name),
            ("db_writeonly_user", &self.db_writeonly_user),
        ] {
            if value.trim().is_empty() {
                bail!("configuration field {name} must not be empty");
            }
        }
        if self.db_port == 0
            || self.serial.baud_rate == 0
            || self.serial.port.is_empty()
            || !(1..=60).contains(&self.serial.reconnect_seconds)
        {
            bail!("invalid port, baud rate or reconnect_seconds in configuration");
        }
        if self.log_file.as_os_str().is_empty()
            || self.failure_file.as_os_str().is_empty()
            || self.log_file == self.failure_file
        {
            bail!("log_file and failure_file must be different, nonempty paths");
        }
        for (name, range) in [
            ("temperature", self.limits.temperature),
            ("humidity", self.limits.humidity),
            ("pressure", self.limits.pressure),
            ("co2", self.limits.co2),
            ("gas_raw", self.limits.gas_raw),
        ] {
            if !range
                .iter()
                .all(|n| n.is_finite() && n.abs() <= f32::MAX as f64)
                || range[0] > range[1]
            {
                bail!("invalid plausibility range for {name}");
            }
        }
        if self.limits.humidity[0] < 0.0
            || self.limits.humidity[1] > 100.0
            || self.limits.co2[0] < 0.0
            || self.limits.gas_raw[0] < 0.0
            || self.limits.gas_raw[1] > 1023.0
        {
            bail!("plausibility ranges exceed physical limits");
        }
        Ok(())
    }

    pub fn open_log(&self) -> Result<File> {
        open_append(&self.log_file)
    }
}

pub fn open_append(path: &Path) -> Result<File> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("cannot create directory {}", parent.display()))?;
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("cannot open {}", path.display()))
}
