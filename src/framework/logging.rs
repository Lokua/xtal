use env_logger::{Builder, Env};
use log::LevelFilter;
use std::io::Write;
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

#[macro_export]
macro_rules! warn_once {
   ($($arg:tt)+) => {{
       use lazy_static::lazy_static;
       use std::collections::HashSet;
       use std::sync::Mutex;

       lazy_static! {
           static ref SEEN: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
       }

       let message = format!($($arg)+);
       let mut set = SEEN.lock().unwrap();
       if set.insert(message.to_string()) {
           log::warn!("{}", message);
       }
   }}
}

#[macro_export]
macro_rules! debug_once {
   ($($arg:tt)+) => {{
       use lazy_static::lazy_static;
       use std::collections::HashSet;
       use std::sync::Mutex;

       lazy_static! {
           static ref SEEN: Mutex<HashSet<String>> = Mutex::new(HashSet::new());
       }

       let message = format!($($arg)+);
       let mut set = SEEN.lock().unwrap();
       if set.insert(message.to_string()) {
           log::debug!("{}", message);
       }
   }}
}

/// Helper to make panic errors stand out more in logs
#[macro_export]
macro_rules! loud_panic {
    ($($arg:tt)*) => {{
        error!("\n\n{}\n\n", format!($($arg)*));
        panic!("\n\n{}\n\n", format!($($arg)*));
    }};
}

/// Logs a debug message at most once within a specified time interval.
#[allow(unused_macros)]
#[macro_export]
macro_rules! debug_throttled {
    ($interval_ms:expr, $($arg:tt)*) => {
        {
            use std::time::{Duration, Instant};
            use std::sync::Mutex;
            use rustc_hash::FxHashMap;
            use log::debug;

            // Lazy initialization of throttle map
            lazy_static::lazy_static! {
                static ref DEBUG_THROTTLE: Mutex<FxHashMap<&'static str, Instant>> =
                    Mutex::new(FxHashMap::default());
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
