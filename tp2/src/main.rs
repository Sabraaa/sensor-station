use std::{
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use tp2::{
    config::Config,
    service::{Collector, ReadFailure, collect_reader},
    simulator,
};

#[derive(Parser)]
#[command(version, about = "TP2 environmental measurement collector")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Receive, validate and store measurements. Without --input, use USB serial.
    Collect {
        #[arg(long, default_value = "config.json")]
        config: PathBuf,
        /// Read a finite newline-delimited file, or '-' for standard input.
        #[arg(long)]
        input: Option<PathBuf>,
        /// Validate input without opening a database connection.
        #[arg(long)]
        validate_only: bool,
    },
    /// Simulate the current Nano firmware on standard output.
    Simulate {
        #[arg(long, default_value_t = 10)]
        count: usize,
        #[arg(long, default_value_t = 1000)]
        interval_ms: u64,
        /// Insert malformed, incomplete and implausible records between samples.
        #[arg(long)]
        include_invalid: bool,
    },
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Simulate {
            count,
            interval_ms,
            include_invalid,
        } => {
            simulator::send(
                &mut io::stdout().lock(),
                count,
                Duration::from_millis(interval_ms),
                include_invalid,
            )?;
        }
        Command::Collect {
            config,
            input,
            validate_only,
        } => {
            let config = Config::load(&config)?;
            let log_file = config.open_log()?;
            tracing_subscriber::fmt()
                .json()
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(log_file))
                .try_init()
                .map_err(|_| anyhow::anyhow!("could not initialize file logging"))?;
            let running = Arc::new(AtomicBool::new(true));
            let signal_flag = Arc::clone(&running);
            ctrlc::set_handler(move || signal_flag.store(false, Ordering::SeqCst))?;
            let mut collector = Collector::new(&config, validate_only)?;
            tracing::info!(validate_only, "collector started");
            eprintln!("Collector started; log: {}", config.log_file.display());

            if let Some(path) = input {
                let mut reader: Box<dyn Read> = if path.as_os_str() == "-" {
                    Box::new(io::stdin())
                } else {
                    Box::new(
                        File::open(&path)
                            .with_context(|| format!("cannot open input {}", path.display()))?,
                    )
                };
                collect_reader(&mut reader, &mut collector, &running, false)?;
            } else {
                while running.load(Ordering::SeqCst) {
                    match serialport::new(&config.serial.port, config.serial.baud_rate)
                        .data_bits(serialport::DataBits::Eight)
                        .parity(serialport::Parity::None)
                        .stop_bits(serialport::StopBits::One)
                        .flow_control(serialport::FlowControl::None)
                        .timeout(Duration::from_millis(250))
                        .open()
                    {
                        Ok(mut port) => {
                            tracing::info!(port = %config.serial.port, baud = config.serial.baud_rate, "serial port connected");
                            if let Err(error) =
                                collect_reader(&mut port, &mut collector, &running, true)
                            {
                                if error.downcast_ref::<ReadFailure>().is_none() {
                                    return Err(error.context(
                                        "collector failed while processing a measurement",
                                    ));
                                }
                                tracing::warn!(error = %error, "serial port disconnected");
                            }
                        }
                        Err(_) => {
                            tracing::warn!(port = %config.serial.port, "cannot open serial port; retrying")
                        }
                    }
                    // Short waits keep SIGINT/SIGTERM responsive during reconnects.
                    for _ in 0..config.serial.reconnect_seconds * 10 {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            tracing::info!(summary = %serde_json::to_string(&collector.stats)?, "collector stopped");
            writeln!(io::stdout(), "{}", serde_json::to_string(&collector.stats)?)?;
            if collector.stats.database_errors > 0 {
                bail!("some measurements could not be stored; inspect the log and failure file");
            }
        }
    }
    Ok(())
}
