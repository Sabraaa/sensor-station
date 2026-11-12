use std::{
    io::{self, Read},
    sync::atomic::AtomicBool,
    time::Duration,
};

use chrono::NaiveDateTime;
use serde_json::json;
use tp2::{
    config::{Config, Limits},
    framing::{Decoder, MAX_LINE_BYTES},
    measurement::Measurement,
    service::{Collector, collect_reader},
    simulator,
};

fn timestamp() -> NaiveDateTime {
    "2026-11-11T14:30:00".parse().unwrap()
}
fn parse(text: &str) -> anyhow::Result<Measurement> {
    Measurement::parse(text.as_bytes(), &Limits::default(), timestamp())
}
fn config(directory: &std::path::Path) -> Config {
    let mut config: Config = serde_json::from_value(json!({
        "hostname":"127.0.0.1", "db_name":"unused", "db_writeonly_user":"unused", "db_writeonly_password":"unused"
    })).unwrap();
    config.log_file = directory.join("test.log");
    config.failure_file = directory.join("failed.jsonl");
    config
}

#[test]
fn csv_mapping_matches_current_firmware_and_database() {
    let sample = parse("650;23.24;45.46;1021.14;312;0.0;1.2;3.4").unwrap();
    assert_eq!(sample.timestamp, timestamp());
    assert_eq!(sample.co2, Some(650.0));
    assert_eq!(sample.temperature, Some(23.2));
    assert_eq!(sample.humidity, Some(45.5));
    assert_eq!(sample.pressure, Some(1021.1));
    assert_eq!(sample.no2, Some(0.0));
    assert_eq!(sample.nh3, Some(1.2));
    assert_eq!(sample.co, Some(3.4));
    assert!(
        serde_json::to_value(sample)
            .unwrap()
            .get("gas_raw")
            .is_none()
    );
}

#[test]
fn original_five_column_firmware_and_explicit_null_are_supported() {
    let sample = parse("650;23.2;45.4;1021.1;312").unwrap();
    assert!(sample.no2.is_none() && sample.co.is_none() && sample.nh3.is_none());
    let sample = parse("null;null;null;null;null;null;null;null").unwrap();
    assert!(sample.co2.is_none() && sample.temperature.is_none());
}

#[test]
fn every_primary_boundary_is_inclusive_and_neighbors_are_rejected() {
    // Firmware order: CO2, temperature, humidity, pressure, ADC, NO2, NH3, CO.
    let limits = [
        (0, 400.0, 5000.0),
        (1, 0.0, 50.0),
        (2, 5.0, 95.0),
        (3, 900.0, 1100.0),
        (4, 1.0, 1022.0),
    ];
    for (index, min, max) in limits {
        for (value, accepted) in [
            (min, true),
            (max, true),
            (min - 1.0, false),
            (max + 1.0, false),
        ] {
            let mut fields = [650.0, 23.2, 45.4, 1021.1, 312.0, 0.0, 1.2, 3.4];
            fields[index] = value;
            let text = fields
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(parse(&text).is_ok(), accepted, "field {index}: {value}");
        }
    }
}

#[test]
fn invalid_fields_never_turn_into_valid_measurements() {
    for index in 0..8 {
        for invalid in ["NaN", "inf", "-inf", "1e999", "", "hello", "1,2", "-1e40"] {
            let mut fields = ["650", "23.2", "45.4", "1021.1", "312", "0.0", "1.2", "3.4"];
            fields[index] = invalid;
            assert!(
                parse(&fields.join(";")).is_err(),
                "field {index}: {invalid}"
            );
        }
    }
    for text in [
        "",
        "1;2;3;4",
        "650;23;45;1013;312;0",
        "650;23;45;1013;312;0;1",
        "650;23;45;1013;312;0;1;2;3",
        "650.5;23;45;1013;312",
        "650;23;45;1013;312.5",
        "650;23;45;1013;312;0;-1;1",
    ] {
        assert!(parse(text).is_err(), "accepted {text}");
    }
    assert!(Measurement::parse(&[0xff], &Limits::default(), timestamp()).is_err());
}

#[test]
fn configured_limits_are_applied_before_rounding() {
    let limits = Limits {
        temperature: [10.0, 30.0],
        ..Limits::default()
    };
    assert!(Measurement::parse(b"650;30.01;45;1013;312", &limits, timestamp()).is_err());
    assert!(Measurement::parse(b"650;30.0;45;1013;312", &limits, timestamp()).is_ok());
}

#[test]
fn framing_bounds_memory_discards_whole_long_lines_and_resynchronizes() {
    let mut decoder = Decoder::default();
    for _ in 0..MAX_LINE_BYTES * 100 {
        assert!(decoder.push(b'x').is_none());
    }
    let frame = decoder.push(b'\n').unwrap();
    assert_eq!(frame.bytes.len(), MAX_LINE_BYTES);
    assert!(frame.error.is_some());
    assert!(decoder.push(b'o').is_none());
    assert!(decoder.push(b'k').is_none());
    assert!(decoder.push(b'\r').is_none());
    let frame = decoder.push(b'\n').unwrap();
    assert_eq!(frame.bytes, b"ok");
    assert!(frame.error.is_none());
    assert!(decoder.finish().is_none());
    decoder.push(b'x');
    assert!(decoder.finish().unwrap().error.is_some());
    assert!(decoder.finish().is_none());
}

#[test]
fn test_sender_records_pass_through_the_collector_without_a_database() {
    let directory = tempfile::tempdir().unwrap();
    let mut collector = Collector::new(&config(directory.path()), true).unwrap();
    let mut input = Vec::new();
    simulator::send(&mut input, 10, Duration::ZERO, true).unwrap();
    collect_reader(
        &mut input.as_slice(),
        &mut collector,
        &AtomicBool::new(true),
        false,
    )
    .unwrap();
    assert_eq!(collector.stats.received, 40);
    assert_eq!(collector.stats.validated, 10);
    assert_eq!(collector.stats.rejected, 30);
    assert_eq!(collector.stats.stored, 0);
    assert_eq!(collector.stats.database_errors, 0);
    assert!(!directory.path().join("failed.jsonl").exists());
}

#[test]
fn serial_timeout_preserves_partial_line_and_disconnect_discards_tail() {
    struct Chunks(usize);
    impl Read for Chunks {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            self.0 += 1;
            let bytes: &[u8] = match self.0 {
                1 => b"650;23.2;",
                2 => return Err(io::ErrorKind::TimedOut.into()),
                3 => b"45.4;1021.1;312\r\n650;23",
                _ => return Err(io::ErrorKind::BrokenPipe.into()),
            };
            output[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let mut collector = Collector::new(&config(directory.path()), true).unwrap();
    assert!(collect_reader(&mut Chunks(0), &mut collector, &AtomicBool::new(true), true).is_err());
    assert_eq!(collector.stats.validated, 1);
    assert_eq!(collector.stats.rejected, 1);
}

#[test]
fn end_of_file_rejects_unterminated_measurements() {
    let directory = tempfile::tempdir().unwrap();
    let mut collector = Collector::new(&config(directory.path()), true).unwrap();
    let mut input = b"650;23.2;45.4;1021.1;312".as_slice();
    collect_reader(&mut input, &mut collector, &AtomicBool::new(true), false).unwrap();
    assert_eq!(collector.stats.validated, 0);
    assert_eq!(collector.stats.rejected, 1);
}

#[test]
fn existing_configuration_keys_defaults_alias_and_relative_paths_work() {
    let directory = tempfile::tempdir().unwrap();
    for host_key in ["hostname", "db_host"] {
        let mut value = json!({"db_name":"umwelt_db","db_writeonly_user":"writer","db_writeonly_password":"local", "db_deadonly_user":"legacy", "db_readonly_password":"ignored"});
        value[host_key] = json!("localhost");
        let path = directory.path().join("config.json");
        std::fs::write(&path, value.to_string()).unwrap();
        let config = Config::load(&path).unwrap();
        assert_eq!(config.serial.baud_rate, 9600);
        assert_eq!(config.serial.port, "/dev/ttyUSB0");
        assert_eq!(config.log_file, directory.path().join("var/tp2.log"));
    }
}

#[test]
fn configuration_errors_do_not_echo_secrets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, r#"{"hostname":"localhost","db_name":"x","db_writeonly_user":"x","db_writeonly_password":"secret","db_port":"VERY_SECRET"}"#).unwrap();
    let error = Config::load(&path).err().unwrap().to_string();
    assert!(!error.contains("SECRET") && !error.contains("secret"));
    let mut config = config(directory.path());
    config.limits.temperature = [50.0, 0.0];
    assert!(config.validate().is_err());
    config.limits.temperature = [0.0, 50.0];
    config.serial.reconnect_seconds = 0;
    assert!(config.validate().is_err());
}
