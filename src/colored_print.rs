use std::io::Write;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn print_green(word: &str) {
    let bufwrt = BufferWriter::stdout(ColorChoice::Always);
    let mut buffer = bufwrt.buffer();
    buffer
        .set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Green))
                .set_intense(true),
        )
        .unwrap();
    writeln!(&mut buffer, "{}", &word).unwrap();
    buffer.set_color(&ColorSpec::new()).unwrap();
    bufwrt.print(&buffer).unwrap();
}

pub fn print_yellow(word: &str) {
    let bufwrt = BufferWriter::stdout(ColorChoice::Always);

    let mut buffer = bufwrt.buffer();
    buffer
        .set_color(
            ColorSpec::new()
                .set_fg(Some(Color::Yellow))
                .set_intense(true),
        )
        .unwrap();
    writeln!(&mut buffer, "{}", &word).unwrap();
    buffer.set_color(&ColorSpec::new()).unwrap();
    bufwrt.print(&buffer).unwrap();
}
