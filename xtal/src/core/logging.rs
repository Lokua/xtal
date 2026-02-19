use env_logger::{Builder, Env};
use log::LevelFilter;
use std::io::Write;
use termcolor::{Color, ColorSpec, WriteColor};

pub use log::{debug, error, info, trace, warn};

pub fn init_logger() {
    let mut builder =
        Builder::from_env(Env::default().default_filter_or("xtal=info"));
    builder.filter_module("naga", LevelFilter::Warn);
    builder.filter_module("wgpu", LevelFilter::Warn);

    builder.format(|_buf, record| {
        let writer =
            termcolor::BufferWriter::stdout(termcolor::ColorChoice::Auto);
        let mut buffer = writer.buffer();
        let mut spec = ColorSpec::new();

        spec.set_fg(Some(match record.level() {
            log::Level::Trace => Color::Cyan,
            log::Level::Debug => Color::Blue,
            log::Level::Info => Color::Green,
            log::Level::Warn => Color::Yellow,
            log::Level::Error => Color::Red,
        }));

        buffer.set_color(&spec)?;
        let module_path = record.module_path().unwrap_or("<unknown>");
        write!(buffer, "[{}][{}]", record.level(), module_path)?;
        buffer.reset()?;
        writeln!(buffer, " {}", record.args())?;
        writer.print(&buffer)?;
        Ok(())
    });

    let _ = builder.try_init();
}
