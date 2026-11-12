use anyhow::{Result, bail};
use chrono::NaiveDateTime;
use serde::Serialize;

use crate::config::Limits;

/// One database row, using the Pi's local reception time.
///
/// The ADC gas reading is checked during parsing but has no database column.
/// CO and NH3 are uncalibrated estimates supplied by the current firmware.
#[derive(Debug, Clone, Serialize)]
pub struct Measurement {
    pub timestamp: NaiveDateTime,
    pub temperature: Option<f64>,
    pub humidity: Option<f64>,
    pub pressure: Option<f64>,
    pub co2: Option<f64>,
    pub no2: Option<f64>,
    pub co: Option<f64>,
    pub nh3: Option<f64>,
}

impl Measurement {
    /// Parse and validate five or eight semicolon-separated CSV fields.
    ///
    /// Order: `co2;temperature;humidity;pressure;gas[;no2;nh3;co]`.
    /// Explicit `null` values become SQL NULL. Any malformed or out-of-range
    /// field rejects the entire row. Climate values are rounded to one decimal
    /// after validation. The caller must strip and check the line terminator.
    pub fn parse(line: &[u8], limits: &Limits, timestamp: NaiveDateTime) -> Result<Self> {
        let text = std::str::from_utf8(line)
            .map_err(|_| anyhow::anyhow!("input is not UTF-8"))?
            .trim();
        if text.is_empty() {
            bail!("empty measurement");
        }
        let values: Vec<_> = text.split(';').collect();
        if values.len() != 5 && values.len() != 8 {
            bail!("CSV requires exactly 5 or 8 fields");
        }
        let mut numbers = Vec::with_capacity(values.len());
        for value in values {
            numbers.push(if value.trim() == "null" {
                None
            } else {
                Some(
                    value
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("CSV field is not a number or null"))?,
                )
            });
        }
        let mut sample = Self {
            timestamp,
            co2: numbers[0],
            temperature: numbers[1],
            humidity: numbers[2],
            pressure: numbers[3],
            no2: numbers.get(5).copied().flatten(),
            nh3: numbers.get(6).copied().flatten(),
            co: numbers.get(7).copied().flatten(),
        };
        let gas = numbers[4];
        for (name, value, range, integer) in [
            ("temperature", sample.temperature, limits.temperature, false),
            ("humidity", sample.humidity, limits.humidity, false),
            ("pressure", sample.pressure, limits.pressure, false),
            ("co2", sample.co2, limits.co2, true),
            ("gas_raw", gas, limits.gas_raw, true),
            ("no2", sample.no2, [0.0, f32::MAX as f64], false),
            ("co", sample.co, [0.0, f32::MAX as f64], false),
            ("nh3", sample.nh3, [0.0, f32::MAX as f64], false),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value < range[0] || value > range[1] {
                    bail!("{name} is outside its plausibility range");
                }
                if integer && value.fract() != 0.0 {
                    bail!("{name} must be an integer");
                }
            }
        }
        // Keep the requested storage resolution for the climate measurements.
        for value in [
            &mut sample.temperature,
            &mut sample.humidity,
            &mut sample.pressure,
        ] {
            *value = value.map(|number| (number * 10.0).round() / 10.0);
        }
        // gas has deliberately been validated but is absent from the real DB schema.
        Ok(sample)
    }
}
