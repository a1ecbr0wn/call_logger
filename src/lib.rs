//! A logger that calls another application or a web service on each log event.
//!
//! The target application or URL that this library calls, is passed a formatted
//! string that defaults to a JSON representation of the logged record.
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
//! # Example - Call default application (`echo`) for each log and default info level,
//! `.new()` defaults to calling `echo` and therefore is analagous to `.with_call_target("echo")`
//! ```
//! let _ = call_logger::CallLogger::new()
//!     .with_level(log::LevelFilter::Info).init();
//! log::info!("msg");
//! ```
//!
//! # Example - Call an application for each log and write the result of the call to a file
//! ```
//! let _ = call_logger::CallLogger::new()
//!     .with_call_target("echo")
//!     .to_file("test.log")
//!     .with_level(log::LevelFilter::Info)
//!     .init();
//! log::info!("msg");
//! # use std::fs::remove_file;
//! # remove_file("test.log").unwrap()
//! ```
//!
//! # Example - Send all output to Discord via their API
//! ```
//! // Get the API endpoint from an environment variable, URL should start with `https://discord.com/api/webhooks/`
//! #[cfg(feature = "timestamps")]
//! if let Ok(endpoint) = std::env::var("DISCORD_API") {
//!     let _ = call_logger::CallLogger::new()
//!         .with_call_target(endpoint)
//!         .with_level(log::LevelFilter::Info)
//!         .format(|timestamp, message, record| {
//!             format!(
//!                 "{{ \"content\": \"{} [{}] {} - {}\" }}",
//!                 timestamp,
//!                 record.level(),
//!                 record.module_path().unwrap_or_default(),
//!                 message
//!             )
//!         })
//!         .init();
//!     log::info!("msg");
//! }
//! ```

use std::{
    borrow::Cow,
    collections::{HashMap, VecDeque},
    fmt::Arguments,
    fs::write,
    path::{Path, PathBuf},
    process::Command,
};

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

/// The `CallLogger` implements [`Log`] and provides some simple builder methods to help configure what and how to log.
/// Some sensible defaults are provided to perform the simple case of calling the `echo` program for all error level
/// logs with a JSON representation of the logged item.  The logger then needs to be initialized (`.init()`) before use.
///
/// # Example - The simple logger that calls `echo`
/// ```
/// # use call_logger::CallLogger;
/// CallLogger::new().init();
/// ```
pub struct CallLogger {
    /// The default logging level filter
    level: LevelFilter,

    /// Custom level filters per module
    levels: Vec<(Cow<'static, str>, log::LevelFilter)>,

    /// The target call to make every time a logging event occurs
    call_target: String,

    /// The format to be used to output the timestamp
    #[cfg(feature = "timestamps")]
    timestamp: TimestampFormat,

    /// The file to write the output of the call to
    file: Option<PathBuf>,

    formatter: Box<Formatter>,

    /// Echo everything to console just before making the call, to aid debugging.
    echo: bool,
}

impl CallLogger {
    /// Creates a new `CallLogger`, use this along with the builder methods and then call `init` to set up the logger.  
    /// The default timestamp format is utc epoch (if the `timestamps` feature is enabled), and the default call app
    /// that is called is `echo`.
    ///
    /// # Example
    /// ```
    /// # use call_logger::CallLogger;
    /// CallLogger::new().init();
    /// ```
    pub fn new() -> CallLogger {
        CallLogger {
            level: LevelFilter::Trace,
            levels: Vec::new(),

            // default to calling echo which will output the log event to console
            call_target: "echo".into(),

            #[cfg(feature = "timestamps")]
            timestamp: TimestampFormat::Utc,
            file: None,
            echo: false,
            formatter: Box::new(Self::json_formatter),
        }
    }

    /// The maximum log level that would be logged.
    ///
    /// # Example
    /// ```
    /// # use call_logger::CallLogger;
    /// # use log::LevelFilter;
    /// CallLogger::new()
    ///     .with_level(LevelFilter::Error)
    ///     .init();
    /// ```
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn with_level(mut self, level: LevelFilter) -> CallLogger {
        self.level = level;
        log::set_max_level(self.level);
        self
    }

    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn with_level_for<T: Into<Cow<'static, str>>>(
        mut self,
        module: T,
        level: log::LevelFilter,
    ) -> Self {
        let module = module.into();

        if let Some((index, _)) = self
            .levels
            .iter()
            .enumerate()
            .find(|(_, (name, _))| *name == module)
        {
            self.levels.remove(index);
        }

        self.levels.push((module, level));
        self
    }

    /// Sets the command line application, script or URL that is called and passed the log details.
    ///
    /// Example - Call an application with parameters
    /// ```
    /// # use call_logger::CallLogger;
    /// CallLogger::new()
    ///     .with_call_target("echo -n")
    ///     .init();    
    /// ```
    ///
    /// Example - Call a URL
    /// ```
    /// # use call_logger::CallLogger;
    /// CallLogger::new()
    ///     .with_call_target("https://postman-echo.com/post")
    ///     .init();    
    /// ```
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn with_call_target<T>(mut self, call_target: T) -> CallLogger
    where
        T: Into<String>,
    {
        self.call_target = call_target.into();
        self
    }

    /// Sets the timestamp to the number of milliseconds since the epoch.
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

    /// Writes each call to console before making the call, use for debugging.
    ///
    /// Example
    /// ```
    /// # use call_logger::CallLogger;
    /// CallLogger::new()
    ///     .echo()
    ///     .init();
    /// ```
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn echo(mut self) -> CallLogger {
        self.echo = true;
        self
    }

    /// Write the output of the call to a file
    #[inline]
    #[must_use = "You must call init() before logging"]
    pub fn to_file<P>(mut self, file: P) -> CallLogger
    where
        P: AsRef<Path>,
    {
        self.file = Some(PathBuf::from(file.as_ref()));
        self
    }

    /// Sets the formatter of this logger. The closure should accept a formatted
    /// value for a timestamp, a message and a log record, and return a `String`
    /// representation of the message that has been formatted.
    ///
    /// [`fmt::Arguments`]: https://doc.rust-lang.org/std/fmt/struct.Arguments.html
    ///
    /// Example usage:
    ///
    /// ```
    ///     let _ = call_logger::CallLogger::new()
    ///         .format(|timestamp, message, record| {
    ///             format!(
    ///                 "{{ \"content\": \"{} [{}] {} - {}\" }}",
    ///                 timestamp,
    ///                 record.level(),
    ///                 record.module_path().unwrap_or_default(),
    ///                 message
    ///             )
    ///         })
    ///         .init();
    ///     log::info!("msg");
    /// ```
    #[inline]
    #[cfg(feature = "timestamps")]
    pub fn format<F>(mut self, formatter: F) -> Self
    where
        F: Fn(String, &Arguments, &log::Record) -> String + Sync + Send + 'static,
    {
        self.formatter = Box::new(formatter);
        self
    }

    /// Sets the formatter of this logger. The closure should accept a message
    /// and a log record, and return a `String` representation of the message
    /// that has been formatted.
    ///
    /// [`fmt::Arguments`]: https://doc.rust-lang.org/std/fmt/struct.Arguments.html
    ///
    /// Example usage:
    ///
    /// ```
    ///     let _ = call_logger::CallLogger::new()
    ///         .format(|message, record| {
    ///             format!(
    ///                 "{{ \"content\": \"[{}] {} - {}\" }}",
    ///                 record.level(),
    ///                 record.module_path().unwrap_or_default(),
    ///                 message
    ///             )
    ///         })
    ///         .init();
    ///     log::info!("msg");
    /// ```
    #[inline]
    #[cfg(not(feature = "timestamps"))]
    pub fn format<F>(mut self, formatter: F) -> Self
    where
        F: Fn(&Arguments, &log::Record) -> String + Sync + Send + 'static,
    {
        self.formatter = Box::new(formatter);
        self
    }

    /// This needs to be called after the builder has set up the logger.
    ///
    /// # Example
    /// ```
    /// # use call_logger::CallLogger;
    /// CallLogger::new().init();
    /// ```
    pub fn init(self) -> Result<(), SetLoggerError> {
        log::set_boxed_logger(Box::new(self))?;
        Ok(())
    }

    #[cfg(feature = "timestamps")]
    fn format_timestamp(&self) -> String {
        match &self.timestamp {
            TimestampFormat::UtcEpochMs => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Leap second or time went backwards")
                .as_millis()
                .to_string(),
            TimestampFormat::UtcEpochUs => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Leap second or time went backwards")
                .as_micros()
                .to_string(),
            TimestampFormat::Utc => Into::<DateTime<Utc>>::into(SystemTime::now())
                .to_rfc3339()
                .to_string(),
            TimestampFormat::Local => Into::<DateTime<Local>>::into(SystemTime::now())
                .to_rfc3339()
                .to_string(),
        }
    }

    #[cfg(not(feature = "timestamps"))]
    fn json_formatter(message: &Arguments, record: &log::Record) -> String {
        Self::json_formatter_inner(String::new(), message, record)
    }

    #[cfg(feature = "timestamps")]
    fn json_formatter(timestamp: String, message: &Arguments, record: &log::Record) -> String {
        Self::json_formatter_inner(timestamp.to_string(), message, record)
    }

    fn json_formatter_inner(
        timestamp: String,
        message: &Arguments,
        record: &log::Record,
    ) -> String {
        let timestamp = format!("\"ts\":\"{timestamp}\",");
        let level = format!("\"level\":\"{}\",", record.level());
        let file = match record.file() {
            Some(file) => format!("\"file\":\"{file}\","),
            None => "".to_string(),
        };
        let line = match record.line() {
            Some(line) => format!("\"line\":\"{line}\","),
            None => "".to_string(),
        };
        let module_path = match record.module_path() {
            Some(module_path) => format!("\"module_path\":\"{module_path}\","),
            None => "".to_string(),
        };
        let mut visitor = LogVisitor {
            map: HashMap::new(),
        };
        let kv_str = if let Ok(()) = record.key_values().visit(&mut visitor) {
            let mut msg = String::new();
            for (key, value) in visitor.map {
                msg.push_str(&format!("\"{key}\":\"{value}\","));
            }
            msg
        } else {
            "".to_string()
        };
        let msg = format!(
            "\"msg\":\"{}\"",
            message
                .to_string()
                .replace('\\', "\\\\")
                .replace('\"', "\\\"")
        );
        format!("{{{timestamp}{level}{file}{line}{module_path}{kv_str}{msg}}}")
    }
}

impl Default for CallLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Log for CallLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatter = &self.formatter;
            #[cfg(feature = "timestamps")]
            let params = formatter(self.format_timestamp(), record.args(), record);
            #[cfg(not(feature = "timestamps"))]
            let params = formatter(record.args(), record);
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
                let mut args = if let Some((header, trailer)) = self.call_target.split_once("{}") {
                    let mut args = header.split(' ').collect::<VecDeque<&str>>();
                    args.push_back(params.as_str());
                    for arg in trailer.split(' ') {
                        args.push_back(arg);
                    }
                    args
                } else {
                    let mut args = self.call_target.split(' ').collect::<VecDeque<&str>>();
                    args.push_back(params.as_str());
                    args
                };
                if self.echo {
                    println!("Calling: `{}`", Vec::from(args.clone()).join(" "));
                }
                let call_target = args.pop_front().unwrap();
                match self.file {
                    Some(_) => match Command::new(call_target).args(args).output() {
                        Ok(output) => {
                            if let Some(file) = &self.file {
                                let _ = write(file, &output.stdout);
                            }
                        }
                        Err(x) => {
                            println!("call to {} failed {x}", self.call_target);
                        }
                    },
                    None => match Command::new(call_target).args(args).spawn() {
                        Ok(_) => {}
                        Err(x) => {
                            println!("call to {} failed {x}", self.call_target);
                        }
                    },
                }
            }
        }
    }

    fn flush(&self) {
        log::logger().flush()
    }
}

// Visitor for querying the kv pairs in a log record.
struct LogVisitor {
    map: HashMap<String, String>,
}

impl<'kvs> VisitSource<'kvs> for LogVisitor {
    fn visit_pair(&mut self, key: Key<'kvs>, value: Value<'kvs>) -> Result<(), Error> {
        self.map.insert(key.to_string(), value.to_string());
        Ok(())
    }
}

/// The type alias for a log formatter.
#[cfg(feature = "timestamps")]
pub type Formatter = dyn Fn(String, &Arguments, &log::Record) -> String + Sync + Send + 'static;
/// The type alias for a log formatter.
#[cfg(not(feature = "timestamps"))]
pub type Formatter = dyn Fn(&Arguments, &log::Record) -> String + Sync + Send + 'static;

#[cfg(test)]
mod test;
