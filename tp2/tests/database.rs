use std::{fs, process::Command};

use chrono::{Local, NaiveDateTime, Timelike};
use mysql::{Conn, Opts, Row, prelude::Queryable};
use serde_json::{Value, json};

/// Run against an explicitly supplied local test database, never config.json.
#[test]
#[ignore = "requires TP2_TEST_DATABASE_URL pointing to a local tp2_test database"]
fn csv_to_mariadb_stores_valid_rows_and_logs_rejections() {
    let url = std::env::var("TP2_TEST_DATABASE_URL").expect("set TP2_TEST_DATABASE_URL");
    let options = Opts::from_url(&url).expect("invalid test database URL");
    assert!(matches!(
        options.get_ip_or_hostname().as_ref(),
        "127.0.0.1" | "localhost"
    ));
    assert!(
        options
            .get_db_name()
            .is_some_and(|name| name.starts_with("tp2_test"))
    );

    let mut database = Conn::new(options.clone()).expect("cannot connect to test MariaDB");
    database
        .query_drop(include_str!("../sql/schema.sql"))
        .unwrap();
    let columns: Vec<Row> = database.query("SHOW COLUMNS FROM measurements").unwrap();
    let names: Vec<String> = columns
        .iter()
        .map(|row| row.get("Field").unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "id",
            "timestamp",
            "temperature",
            "humidity",
            "pressure",
            "co2",
            "no2",
            "co",
            "nh3"
        ]
    );
    let previous_id: u64 = database
        .query_first("SELECT COALESCE(MAX(id), 0) FROM measurements")
        .unwrap()
        .unwrap();

    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config.json");
    fs::write(
        &config,
        json!({
            "hostname": options.get_ip_or_hostname(),
            "db_port": options.get_tcp_port(),
            "db_name": options.get_db_name(),
            "db_writeonly_user": options.get_user(),
            "db_writeonly_password": options.get_pass(),
            "log_file": "collector.log",
            "failure_file": "failed.jsonl"
        })
        .to_string(),
    )
    .unwrap();

    let binary = env!("CARGO_BIN_EXE_tp2");
    let generated = Command::new(binary)
        .args([
            "simulate",
            "--count",
            "10",
            "--interval-ms",
            "0",
            "--include-invalid",
        ])
        .output()
        .unwrap();
    assert!(generated.status.success());
    let mut input = generated.stdout;
    input.extend_from_slice(b"null;null;null;null;null;null;null;null\n");
    let input_path = directory.path().join("input.csv");
    fs::write(&input_path, input).unwrap();
    let before = Local::now().naive_local().with_nanosecond(0).unwrap();

    let collected = Command::new(binary)
        .arg("collect")
        .arg("--config")
        .arg(&config)
        .arg("--input")
        .arg(&input_path)
        .output()
        .unwrap();
    assert!(
        collected.status.success(),
        "{}",
        String::from_utf8_lossy(&collected.stderr)
    );
    let summary: Value = serde_json::from_slice(&collected.stdout).unwrap();
    assert_eq!(
        summary,
        json!({"received":41,"validated":11,"stored":11,"rejected":30,"database_errors":0})
    );

    let rows: Vec<Row> = database
        .exec(
            "SELECT * FROM measurements WHERE id > ? ORDER BY id",
            (previous_id,),
        )
        .unwrap();
    assert_eq!(rows.len(), 11);
    for (name, expected) in [
        ("co2", 600.0),
        ("temperature", 20.0),
        ("humidity", 45.0),
        ("pressure", 1013.2),
        ("no2", 0.0),
        ("nh3", 1.2),
        ("co", 3.4),
    ] {
        let actual: f64 = rows[0].get(name).unwrap();
        assert!((actual - expected).abs() < 0.001, "incorrect {name}");
    }
    let timestamp: NaiveDateTime = rows[0].get("timestamp").unwrap();
    assert!(timestamp >= before && timestamp <= Local::now().naive_local());
    for name in [
        "temperature",
        "humidity",
        "pressure",
        "co2",
        "no2",
        "nh3",
        "co",
    ] {
        assert_eq!(rows[10].get::<Option<f64>, _>(name), Some(None));
    }
    let log = fs::read_to_string(directory.path().join("collector.log")).unwrap();
    let rejections = log
        .lines()
        .filter(|line| {
            let event: Value = serde_json::from_str(line).unwrap();
            event["fields"]["message"] == "measurement rejected"
        })
        .count();
    assert_eq!(rejections, 30);

    // A new collector process appends a row without changing earlier measurements.
    fs::write(&input_path, b"650;23.2;45.4;1021.1;312;0;1.2;3.4\n").unwrap();
    let restarted = Command::new(binary)
        .arg("collect")
        .arg("--config")
        .arg(&config)
        .arg("--input")
        .arg(&input_path)
        .output()
        .unwrap();
    assert!(restarted.status.success());
    let count: u64 = database
        .exec_first(
            "SELECT COUNT(*) FROM measurements WHERE id > ?",
            (previous_id,),
        )
        .unwrap()
        .unwrap();
    assert_eq!(count, 12);
}
