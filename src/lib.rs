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

/// The format to use when outputting the timestamp of the log.  Timestamps are only part
/// of the log output if the `timestamps` feature is enabled for `call_logger`/
#[cfg(feature = "timestamps")]
#[derive(PartialEq, Debug)]
pub enum TimestampFormat {
    UtcEpochMs,
    UtcEpochUs,
    Utc,
    Local,
}

/// Implements [`Log`] and some simple builder methods to configure.
pub struct CallLogger {
    /// The default logging level
    default_level: LevelFilter,

    /// The target call to make every time a logging event occurs
    call_target: String,

    /// The format to be used to output the timestamp
    #[cfg(feature = "timestamps")]
    timestamp: TimestampFormat,
}

impl CallLogger {
    /// Creates a new `CallLogger`, use this along with the builder methods and then call `init` to
    /// set up the logger.  The default timestamp format is utc epoch (if the `timestamps` feature
    /// is enabled), and the default call app that is called is `echo`.
    pub fn new() -> CallLogger {
        CallLogger {
            default_level: LevelFilter::Trace,

            // default to calling echo which will output the log event to console
            call_target: "echo".to_string(),

            #[cfg(feature = "timestamps")]
            timestamp: TimestampFormat::Utc,
        }
    }

    /// The maximum log level that would be logged
    #[must_use = "You must call init() before logging"]
    pub fn with_level(mut self, level: LevelFilter) -> CallLogger {
        self.default_level = level;
        self
    }

    /// Sets the command line app or script that is called and passed the log details
    #[must_use = "You must call init() before logging"]
    pub fn with_call_target(mut self, call_target: String) -> CallLogger {
        self.call_target = call_target;
        self
    }

    /// Sets the timestamp to the number of milliseconds since the epoch
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_epoch_ms_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::UtcEpochMs;
        self
    }

    /// Sets the timestamp to the number of microseconds since the epoch
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_epoch_us_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::UtcEpochUs;
        self
    }

    /// Sets the timestamp to a the UTC timezone
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_utc_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::Utc;
        self
    }

    /// Sets the timestamp to a the local timezone
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_local_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::Local;
        self
    }

    /// This needs to be called after the builder has set up the logger
    pub fn init(self) -> Result<(), SetLoggerError> {
        log::set_max_level(self.default_level);
        log::set_boxed_logger(Box::new(self))?;
        Ok(())
    }
}

impl Default for CallLogger {
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
                TimestampFormat::Utc => format!(
                    "\"ts\": \"{}\", ",
                    Into::<DateTime<Utc>>::into(SystemTime::now()).to_rfc3339()
                ),
                TimestampFormat::Local => format!(
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
            Ok(_child) => {}
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
    fn test_level() {
        let logger = CallLogger::default().with_level(LevelFilter::Info);
        assert_eq!(logger.default_level, LevelFilter::Info);
    }

    #[test]
    fn test_call_target() {
        let logger = CallLogger::default().with_call_target("wc".to_string());
        assert_eq!(logger.call_target, "wc".to_string());
    }

    #[test]
    #[cfg(feature = "timestamps")]
    fn test_epoch_ms_timestamp() {
        let logger = CallLogger::default().with_epoch_ms_timestamp();
        assert_eq!(logger.timestamp, TimestampFormat::UtcEpochMs);
    }

    #[test]
    #[cfg(feature = "timestamps")]
    fn test_epoch_us_timestamp() {
        let logger = CallLogger::default().with_epoch_us_timestamp();
        assert_eq!(logger.timestamp, TimestampFormat::UtcEpochUs);
    }

    #[test]
    #[cfg(feature = "timestamps")]
    fn test_utc_timestamp() {
        let logger = CallLogger::default().with_utc_timestamp();
        assert_eq!(logger.timestamp, TimestampFormat::Utc);
    }

    #[test]
    #[cfg(feature = "timestamps")]
    fn test_local_timestamp() {
        let logger = CallLogger::default().with_local_timestamp();
        assert_eq!(logger.timestamp, TimestampFormat::Local);
    }
}
