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
            let params = format!("{{{timestamp}{level}{file}{line}{module_path}{kv_str}{msg}}}");
            if self.call_target.starts_with("http://") || self.call_target.starts_with("https://") {
                if self.echo {
                    println!("Calling: `{}\n\t{params}`", self.call_target);
                }
                reqwest::blocking::Client::new()
                    .post(&self.call_target)
                    .header("Content-Type", "application/json")
                    .body(params)
                    .send()
                    .unwrap();
            } else {
                if self.echo {
                    println!("Calling: `{} {params}`", self.call_target);
                }
                let mut args = self.call_target.split(' ');
                let call_target = args.next().unwrap();
                let call_rtn = if args.clone().count() > 0 {
                    Command::new(call_target).args(args).args([&params]).spawn()
                } else {
                    Command::new(call_target).args([&params]).spawn()
                };
                match call_rtn {
                    Ok(_) => {}
                    Err(x) => {
                        println!("call to {} {params} failed {x}", self.call_target);
                    }
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
mod test;
