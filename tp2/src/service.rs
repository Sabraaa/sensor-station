use std::{
    fs::File,
    io::{self, Read, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use chrono::{Local, Timelike};
use serde::Serialize;

use crate::{
    config::{Config, Limits, open_append},
    database::Database,
    framing::{Decoder, Frame},
    measurement::Measurement,
};

#[derive(Default, Debug, Serialize)]
pub struct Stats {
    pub received: u64,
    pub validated: u64,
    pub stored: u64,
    pub rejected: u64,
    pub database_errors: u64,
}

#[derive(Debug)]
pub struct ReadFailure(pub io::Error);

impl std::fmt::Display for ReadFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for ReadFailure {}

pub struct Collector {
    limits: Limits,
    database: Option<Database>,
    failure_file: Option<File>,
    pub stats: Stats,
}

impl Collector {
    /// Create a collector. Validation-only mode opens no database or failure file.
    pub fn new(config: &Config, validate_only: bool) -> Result<Self> {
        Ok(Self {
            limits: config.limits.clone(),
            database: (!validate_only).then(|| Database::new(config)),
            failure_file: if validate_only {
                None
            } else {
                Some(open_append(&config.failure_file)?)
            },
            stats: Stats::default(),
        })
    }

    /// Validate one frame, then store it or log why it was rejected.
    /// Unconfirmed database writes are preserved in the local failure file.
    pub fn process(&mut self, frame: Frame) -> Result<()> {
        self.stats.received += 1;
        let timestamp = Local::now()
            .naive_local()
            .with_nanosecond(0)
            .expect("zero nanoseconds are valid");
        let result = match frame.error {
            Some(reason) => Err(anyhow::anyhow!(reason)),
            None => Measurement::parse(&frame.bytes, &self.limits, timestamp),
        };
        match result {
            Err(error) => {
                self.stats.rejected += 1;
                tracing::warn!(reason = %error, input = %String::from_utf8_lossy(&frame.bytes), "measurement rejected");
            }
            Ok(sample) => {
                self.stats.validated += 1;
                if let Some(database) = &mut self.database {
                    match database.insert(&sample) {
                        Ok(()) => self.stats.stored += 1,
                        Err(error) => {
                            self.stats.database_errors += 1;
                            // Preserve the full sample locally. It may already exist in MariaDB if
                            // only the acknowledgement was lost, so replay requires a manual check.
                            let file = self
                                .failure_file
                                .as_mut()
                                .expect("database mode has failure file");
                            serde_json::to_writer(&mut *file, &sample)?;
                            file.write_all(b"\n")?;
                            file.sync_data()?;
                            tracing::error!(reason = %error, "measurement storage unconfirmed; saved to failure file");
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Receive newline-terminated samples until EOF, shutdown or an I/O error.
/// Serial timeouts preserve partial lines; EOF or disconnect discards the tail.
pub fn collect_reader(
    reader: &mut dyn Read,
    collector: &mut Collector,
    running: &AtomicBool,
    serial: bool,
) -> Result<()> {
    let mut decoder = Decoder::default();
    let mut buffer = [0; 512];
    let result = loop {
        if !running.load(Ordering::SeqCst) {
            break Ok(());
        }
        match reader.read(&mut buffer) {
            Ok(0) if serial => {
                break Err(ReadFailure(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "serial device reached end of stream",
                ))
                .into());
            }
            Ok(0) => break Ok(()),
            Ok(count) => {
                for &byte in &buffer[..count] {
                    if let Some(frame) = decoder.push(byte) {
                        collector.process(frame)?;
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) && serial =>
            {
                continue;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => break Err(ReadFailure(error).into()),
        }
    };
    if let Some(frame) = decoder.finish() {
        collector.process(frame)?;
    }
    result
}
