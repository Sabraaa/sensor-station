use std::time::{Duration, Instant};

use mysql::{Conn, OptsBuilder, params, prelude::Queryable};

use crate::{config::Config, measurement::Measurement};

// Only columns from the existing measurements table are used here.
const INSERT: &str = "INSERT INTO measurements (`timestamp`, temperature, humidity, pressure, co2, no2, co, nh3) VALUES (:timestamp, :temperature, :humidity, :pressure, :co2, :no2, :co, :nh3)";

pub struct Database {
    options: OptsBuilder,
    connection: Option<Conn>,
    retry_after: Option<Instant>,
}

impl Database {
    pub fn new(config: &Config) -> Self {
        let timeout = Some(Duration::from_secs(2));
        let options = OptsBuilder::new()
            .ip_or_hostname(Some(config.hostname.clone()))
            .tcp_port(config.db_port)
            .user(Some(config.db_writeonly_user.clone()))
            .pass(Some(config.db_writeonly_password.clone()))
            .db_name(Some(config.db_name.clone()))
            .prefer_socket(false)
            .tcp_connect_timeout(timeout)
            .read_timeout(timeout)
            .write_timeout(timeout);
        Self {
            options,
            connection: None,
            retry_after: None,
        }
    }

    /// Insert a sample using the writer account. No table changes or reads occur.
    /// Failed INSERTs are not retried because an acknowledgement may be lost
    /// after commit. A later sample reconnects after a five-second cooldown.
    pub fn insert(&mut self, sample: &Measurement) -> Result<(), String> {
        if self
            .retry_after
            .is_some_and(|deadline| Instant::now() < deadline)
        {
            return Err("database unavailable; waiting before reconnect".into());
        }
        if self.connection.is_none() {
            match Conn::new(self.options.clone()) {
                Ok(connection) => self.connection = Some(connection),
                Err(error) => {
                    self.disconnect();
                    return Err(safe_error(&error));
                }
            }
        }
        let result = self
            .connection
            .as_mut()
            .expect("connection initialized")
            .exec_drop(
                INSERT,
                params! {
                    "timestamp" => sample.timestamp,
                    "temperature" => sample.temperature,
                    "humidity" => sample.humidity,
                    "pressure" => sample.pressure,
                    "co2" => sample.co2,
                    "no2" => sample.no2,
                    "co" => sample.co,
                    "nh3" => sample.nh3,
                },
            );
        if let Err(error) = result {
            self.disconnect();
            // A lost acknowledgement may mean the INSERT committed. Do not blindly retry it.
            return Err(safe_error(&error));
        }
        Ok(())
    }

    fn disconnect(&mut self) {
        self.connection = None;
        self.retry_after = Some(Instant::now() + Duration::from_secs(5));
    }
}

fn safe_error(error: &mysql::Error) -> String {
    match error {
        mysql::Error::MySqlError(error) => {
            format!("database server error {} ({})", error.code, error.state)
        }
        _ => "database connection or protocol error".into(),
    }
}
