//! A logger that calls another application on each log event
//!
//! The target application that this library calls, is passed a JSON formatted parameter that displays the
//! information about the log call to the target application.
//!
//! # Why would you do this?
//!
//! - There are quick a dirty things that you might want to do with log output
//! - You want your log output to be handled differently in different environments which you can configure
//! - You want to use call a webhook/webservice to notify another service (e.g. Pushover.net, discord, AWS Cloudwatch)
//!
//! # Features
//!
//! - `timestamps`
//!   - add a timestamp to the output
//!   - the timestamp can be set to one of a number of formats specified by a number of [`CallLogger`] builder functions
//!
//! # Example
//! ```
//! let _ = call_logger::CallLogger::new().with_level(log::LevelFilter::Info).init();
//! log::info!("msg");
//! ```

use std::{collections::HashMap, process::Command};

use log::kv::{Error, Key, Value, VisitSource};
use log::{LevelFilter, Log, Metadata, Record, SetLoggerError};

#[cfg(feature = "timestamps")]
use chrono::{DateTime, Local, Utc};
#[cfg(feature = "timestamps")]
use std::time::{SystemTime, UNIX_EPOCH};

/// The format to use when outputting the timestamp of the log.  Timestamps are only part
/// of the log output if the `timestamps` feature is enabled for `call_logger`/
#[cfg(feature = "timestamps")]
#[derive(PartialEq, Debug)]
enum TimestampFormat {
    UtcEpochMs,
    UtcEpochUs,
    Utc,
    Local,
}

/// Implements [`Log`] and some simple builder methods to configure.
pub struct CallLogger {
    /// The default logging level
    level: LevelFilter,

    /// The target call to make every time a logging event occurs
    call_target: String,

    /// The format to be used to output the timestamp
    #[cfg(feature = "timestamps")]
    timestamp: TimestampFormat,

    /// Echo everything to console just before making the call, to aid debugging.
    echo: bool,
}

impl CallLogger {
    /// Creates a new `CallLogger`, use this along with the builder methods and then call `init` to
    /// set up the logger.  The default timestamp format is utc epoch (if the `timestamps` feature
    /// is enabled), and the default call app that is called is `echo`.
    pub fn new() -> CallLogger {
        CallLogger {
            level: LevelFilter::Trace,

            // default to calling echo which will output the log event to console
            call_target: "echo".to_string(),

            #[cfg(feature = "timestamps")]
            timestamp: TimestampFormat::Utc,

            echo: false,
        }
    }

    /// The maximum log level that would be logged
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn with_level(mut self, level: LevelFilter) -> CallLogger {
        self.level = level;
        log::set_max_level(self.level);
        self
    }

    /// Sets the command line app or script that is called and passed the log details
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn with_call_target(mut self, call_target: String) -> CallLogger {
        self.call_target = call_target;
        self
    }

    /// Sets the timestamp to the number of milliseconds since the epoch
    #[inline]
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_epoch_ms_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::UtcEpochMs;
        self
    }

    /// Sets the timestamp to the number of microseconds since the epoch
    #[inline]
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_epoch_us_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::UtcEpochUs;
        self
    }

    /// Sets the timestamp to a the UTC timezone
    #[inline]
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_utc_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::Utc;
        self
    }

    /// Sets the timestamp to a the local timezone
    #[inline]
    #[must_use = "You must call init() before logging"]
    #[cfg(feature = "timestamps")]
    pub fn with_local_timestamp(mut self) -> CallLogger {
        self.timestamp = TimestampFormat::Local;
        self
    }

    /// Writes each call to console before making the call, use for debugging
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn echo(mut self) -> CallLogger {
        self.echo = true;
        self
    }

    /// This needs to be called after the builder has set up the logger
    pub fn init(self) -> Result<(), SetLoggerError> {
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
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
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
                Some(file) => format!("\"file\": \"{file}\", "),
                None => "".to_string(),
            };
            let line = match record.line() {
                Some(line) => format!("\"line\": \"{line}\", "),
                None => "".to_string(),
            };
            let module_path = match record.module_path() {
                Some(module_path) => format!("\"module_path\": \"{module_path}\", "),
                None => "".to_string(),
            };
            let mut visitor = LogVisitor {
                map: HashMap::new(),
            };
            let kv_str = if let Ok(()) = record.key_values().visit(&mut visitor) {
                let mut msg = String::new();
                for (key, value) in visitor.map {
                    msg.push_str(&format!("\"{key}\": \"{value}\", "));
                }
                msg
            } else {
                "".to_string()
            };
            let msg = format!(
                "\"msg\": \"{}\"",
                record
                    .args()
                    .to_string()
                    .replace('\\', "\\\\")
                    .replace('\"', "\\\"")
            );
            let json = format!("{{ {timestamp}{level}{file}{line}{module_path}{kv_str}{msg} }}");
            if self.echo {
                println!("Calling: `{} {json}`", self.call_target);
            }
            let mut args = self.call_target.split(' ');
            let call_target = args.next().unwrap();
            let call_rtn = if args.clone().count() > 0 {
                Command::new(call_target)
                    .args(args)
                    .args([json.as_str()])
                    .spawn()
            } else {
                Command::new(call_target).args([json]).spawn()
            };
            match call_rtn {
                Ok(_) => {}
                Err(x) => {
                    println!("call to {} failed {x}", self.call_target);
                }
            }
        }
    }

    fn flush(&self) {
        log::logger().flush()
    }
}

struct LogVisitor {
    map: HashMap<String, String>,
}

impl<'kvs> VisitSource<'kvs> for LogVisitor {
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error> {
        self.map.insert(key.to_string(), value.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use log::{
        kv::{Source, ToKey, ToValue},
        Level,
    };
    use std::{
        fs::{read_to_string, remove_file},
        thread, time,
    };

    #[test]
    fn test_log() {
        let logger = CallLogger::new()
            .with_level(LevelFilter::Error)
            .with_call_target("scripts/to_file.sh test_log.log".to_string());
        logger.log(
            &Record::builder()
                .args(format_args!("test message"))
                .file(Some("src/lib.rs"))
                .module_path(Some("call_logger::test"))
                .level(Level::Error)
                .build(),
        );
        for _ in 0..20 {
            if let Ok(test) = read_to_string("test_log.log") {
                println!("test_log.log: {test}");
                assert!(test.contains("\"level\": \"ERROR\""));
                assert!(test.contains("\"file\": \"src/lib.rs\""));
                assert!(test.contains("\"module_path\": \"call_logger::test\""));
                assert!(test.contains("\"msg\": \"test message\""));
                remove_file("test_log.log").unwrap();
                thread::sleep(time::Duration::from_millis(10));
                return;
            } else {
                thread::sleep(time::Duration::from_millis(100));
            }
        }
        panic!("Failed to detect the log message");
    }

    #[test]
    fn test_log_default() {
        let logger = CallLogger::default();
        assert_eq!(logger.level, LevelFilter::Trace);
        assert_eq!(logger.call_target, "echo".to_string());
        let _ = logger.init();
        log::info!("test message");
    }

    #[test]
    fn test_log_quoted_string() {
        let logger = CallLogger::default();
        assert_eq!(logger.level, LevelFilter::Trace);
        assert_eq!(logger.call_target, "echo".to_string());
        let msg = r#"{ "message": "test message" }"#;
        logger.log(&Record::builder().args(format_args!("{msg}")).build());
    }

    #[test]
    fn test_log_level_filter() {
        let logger = CallLogger::new().with_level(LevelFilter::Error);
        assert_eq!(logger.level, LevelFilter::Error);
        assert_eq!(logger.call_target, "echo".to_string());
        logger.log(
            &Record::builder()
                .args(format_args!("filtered message"))
                .build(),
        );
    }

    #[test]
    fn test_level() {
        let logger = CallLogger::default().with_level(LevelFilter::Info);
        assert_eq!(logger.level, LevelFilter::Info);
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

    #[test]
    fn test_kv_log() {
        let logger = CallLogger::default()
            .with_call_target("scripts/to_file.sh test_kv_log.log".to_string());
        let source = TestSource {
            key: "test".to_string(),
            value: "value".to_string(),
        };
        logger.log(
            &Record::builder()
                .args(format_args!("test message"))
                .key_values(&source)
                .file(Some("src/lib.rs"))
                .module_path(Some("call_logger::test"))
                .level(Level::Info)
                .build(),
        );
        thread::sleep(time::Duration::from_millis(20));
        if let Ok(test) = read_to_string("test_kv_log.log") {
            println!("test_kv_log.log: {test}");
            assert!(test.contains("\"test\": \"value\""));
            assert!(test.contains("\"level\": \"INFO\""));
            assert!(test.contains("\"file\": \"src/lib.rs\""));
            assert!(test.contains("\"module_path\": \"call_logger::test\""));
            assert!(test.contains("\"msg\": \"test message\""));
            remove_file("test_kv_log.log").unwrap();
            thread::sleep(time::Duration::from_millis(10));
        } else {
            panic!("test_kv_log.log cannot be read, consider increasing how long we wait for the test file to be written");
        }
    }

    struct TestSource {
        key: String,
        value: String,
    }

    impl Source for TestSource {
        fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
            visitor.visit_pair(self.key.to_key(), self.value.to_value())
        }
    }
}
