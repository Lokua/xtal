use env_logger::{Builder, Env};
use log::LevelFilter;
use once_cell::sync::Lazy;
use std::{collections::HashSet, io::Write, sync::Mutex};
use termcolor::{Color, ColorSpec, WriteColor};

pub use log::{debug, error, info, trace, warn};

pub fn init_logger() {
    Builder::from_env(Env::default().default_filter_or("lattice=info"))
        .filter_module("nannou", LevelFilter::Warn)
        .format(|_buf, record| {
            let buffer_writer =
                termcolor::BufferWriter::stdout(termcolor::ColorChoice::Auto);
            let mut buffer = buffer_writer.buffer();
            let mut spec = ColorSpec::new();

            spec.set_fg(Some(match record.level() {
                log::Level::Trace => Color::Cyan,
                log::Level::Debug => Color::Blue,
                log::Level::Info => Color::Green,
                log::Level::Warn => Color::Yellow,
                log::Level::Error => Color::Red,
            }))
            .set_bold(true);

            buffer.set_color(&spec)?;

            let module_path = record.module_path().unwrap_or("<unknown>");

            write!(buffer, "[{}][{}]", record.level(), module_path)?;

            buffer.reset()?;
            writeln!(buffer, " {}", record.args())?;

            buffer_writer.print(&buffer)?;
            Ok(())
        })
        .init();
}

static WARNED_MESSAGES: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

pub fn warn_once(message: String) {
    let mut set = WARNED_MESSAGES.lock().unwrap();
    if set.insert(message.to_string()) {
        warn!("{}", message);
    }
}

#[allow(unused_macros)]
#[macro_export]
macro_rules! loud_panic {
    ($($arg:tt)*) => {{
        error!($($arg)*);
        panic!($($arg)*);
    }};
}

/// Logs a debug message at most once within a specified time interval.
///
/// # Parameters
/// - `$interval_ms`: The minimum time in milliseconds between log messages
///   for the same content.
/// - `$($arg:tt)*`: The debug message and its optional format arguments,
///   similar to the `log::debug!` macro.
///
/// This macro ensures that repeated debug messages are throttled, avoiding
/// log spam. It uses a global throttle map to track the last logged time
/// for each unique message.
///
/// # Examples
/// ```rust
/// debug_throttled!(1000, "This message appears at most once every second.");
/// debug_throttled!(2000, "Another throttled message: {}", 42);
/// ```
#[allow(unused_macros)]
#[macro_export]
macro_rules! debug_throttled {
    ($interval_ms:expr, $($arg:tt)*) => {
        {
            use std::collections::HashMap;
            use std::time::{Duration, Instant};
            use std::sync::Mutex;
            use log::debug;

            // Lazy initialization of throttle map
            lazy_static::lazy_static! {
                static ref DEBUG_THROTTLE: Mutex<HashMap<&'static str, Instant>> =
                    Mutex::new(HashMap::new());
            }

            // Throttle logic
            let key = stringify!($($arg)*);
            let interval = Duration::from_millis($interval_ms as u64);
            let mut throttle_map = DEBUG_THROTTLE.lock().unwrap();
            let now = Instant::now();

            if throttle_map.get(key).map_or(true, |&last_log_time| {
                now.duration_since(last_log_time) >= interval
            }) {
                throttle_map.insert(key, now);
                debug!($($arg)*);
            }
        }
    };
}
