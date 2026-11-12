use std::{
    io::{self, Write},
    time::Duration,
};

/// Send reproducible CSV samples in the current Nano's eight-field order.
/// With `include_invalid`, each valid sample is followed by three bad lines.
pub fn send(
    output: &mut impl Write,
    count: usize,
    interval: Duration,
    include_invalid: bool,
) -> io::Result<()> {
    for index in 0..count {
        let co2 = 600 + index % 1000;
        let temperature = 20.0 + (index % 100) as f64 / 10.0;
        writeln!(output, "{co2};{temperature:.1};45.0;1013.2;312;0.0;1.2;3.4")?;
        if include_invalid {
            writeln!(output, "not-a-measurement")?;
            writeln!(output, "600;22.0;45.0")?;
            writeln!(output, "600;99.0;45.0;1013.2;312;0.0;1.2;3.4")?;
        }
        output.flush()?;
        if index + 1 < count {
            std::thread::sleep(interval);
        }
    }
    Ok(())
}
