//! A logger that calls another application on each log event
//!
//! The target application that this library calls, is passed a JSON formatted parameter that displays the
//! information about the log call to the target application.
//!
//! Why would you do this?
//!
//! - There are quick a dirty things that you might want to do with log output
//! - You want your log output to be handled differently in different environments which you can configure
//! - You want to use call a webhook/webservice to notify another service (e.g. Pushover.net, discord, AWS Cloudwatch)

use std::process::Command;

use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

#[cfg(feature = "timestamps")]
use chrono::{DateTime, Local, Utc};
#[cfg(feature = "timestamps")]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "timestamps")]
#[derive(PartialEq)]
pub enum TimestampFormat {
    UtcEpochMs,
    UtcEpochUs,
    UtcTime,
    LocalTime,
}

/// Implements [`Log`] and some simple builder methods to configure.
pub struct CallLogger {
    /// The default logging level
    default_level: LevelFilter,

    /// The target call to make every time a logging event occurs
    call_target: String,

    #[cfg(feature = "timestamps")]
    timestamp: TimestampFormat,
}

impl CallLogger {
    pub fn new() -> CallLogger {
        CallLogger {
            default_level: LevelFilter::Trace,

            // default to calling echo which will output the log event to console
            call_target: "echo".to_string(),

            #[cfg(feature = "timestamps")]
            timestamp: TimestampFormat::UtcTime,
        }
    }

    #[must_use = "You must call init() before logging"]
    pub fn with_level(mut self, level: LevelFilter) -> CallLogger {
        self.default_level = level;
        self
    }

    #[must_use = "You must call init() before logging"]
    pub fn with_call_target(mut self, call_target: String) -> CallLogger {
        self.call_target = call_target;
        self
    }

    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_timestamp(mut self, timestamp: TimestampFormat) -> CallLogger {
        self.timestamp = timestamp;
        self
    }

    pub fn init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.default_level);
        log::set_boxed_logger(Box::new(self))?;
        Ok(())
    }
}

impl Default for CallLogger {
    /// See [this](struct.SimpleLogger.html#method.new)
    fn default() -> Self {
        CallLogger::new()
    }
}

impl Log for CallLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let timestamp = {
            #[cfg(feature = "timestamps")]
            match &self.timestamp {
                TimestampFormat::UtcEpochMs => format!(
                    "\"ts\": \"{}\", ",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Leap second or time went backwards")
                        .as_millis()
                ),
                TimestampFormat::UtcEpochUs => format!(
                    "\"ts\": \"{}\", ",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Leap second or time went backwards")
                        .as_micros()
                ),
                TimestampFormat::UtcTime => format!(
                    "\"ts\": \"{}\", ",
                    Into::<DateTime<Utc>>::into(SystemTime::now()).to_rfc3339()
                ),
                TimestampFormat::LocalTime => format!(
                    "\"ts\": \"{}\", ",
                    Into::<DateTime<Local>>::into(SystemTime::now()).to_rfc3339()
                ),
            }

            #[cfg(not(feature = "timestamps"))]
            ""
        };
        let level = format!("\"level\": \"{}\", ", record.level());
        let file = match record.file() {
            Some(file) => format!("\"file\": \"{}\", ", file),
            None => "".to_string(),
        };
        let line = match record.line() {
            Some(line) => format!("\"line\": \"{}\", ", line),
            None => "".to_string(),
        };
        let module_path = match record.module_path() {
            Some(module_path) => format!("\"module_path\": \"{}\", ", module_path),
            None => "".to_string(),
        };
        let msg = format!("\"msg\": \"{}\"", record.args());
        let json = format!("{{ {timestamp}{level}{file}{line}{module_path}{msg} }}");
        let call_rtn = Command::new(self.call_target.clone()).args([json]).spawn();
        match call_rtn {
            Ok(_child) => {
                println!(
                    "{} called successfully with pid {}",
                    self.call_target,
                    _child.id()
                );
            }
            Err(x) => {
                println!("call to {} failed {x}", self.call_target);
            }
        }
    }

    fn flush(&self) {}
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_log_default() {
        let logger = CallLogger::default();
        assert_eq!(logger.default_level, LevelFilter::Trace);
        assert_eq!(logger.call_target, "echo".to_string());
        let _ = logger.init();
        log::info!("test_log_default");
    }

    #[test]
    fn test_log_custom() {
        let logger = CallLogger::default()
            .with_level(LevelFilter::Info)
            .with_call_target("wc".to_string())
            .with_timestamp(TimestampFormat::UtcEpochUs);
        assert_eq!(logger.default_level, LevelFilter::Info);
        assert_eq!(logger.call_target, "wc".to_string());
        let _ = logger.with_call_target("echo".to_string()).init();
        log::error!("test_log_custom");
    }
}
