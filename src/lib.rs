//! A logger that calls another application on each log event
//!
//! The target application that this library calls, is passed a JSON formatted parameter that passes the 
//! information about the log call to the target application.
//! 
//! Why would you do this?
//! 
//! - There are quick a dirty things that you might want to do with log output
//! - You want your log output to be handled differently in different environments which you can configure
//! - You want to use call a webhook to notify another service (e.g. Pushover.net or discord)

use std::str::FromStr;

use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};

/// Implements [`Log`] and some simple builder methods to configure.
pub struct CallLogger {
    /// The default logging level
    default_level: LevelFilter,

    /// The target call to make every time a logging event occurs
    call_target: String,
}

impl CallLogger {
    pub fn new() -> CallLogger {
        CallLogger {
            default_level: LevelFilter::Trace,

            // default to calling echo which will output the log event to console
            call_target: "echo".to_string(),
        }
    }

    #[must_use = "You must call init() to before logging"]
    pub fn env(mut self) -> CallLogger {
        self.default_level = std::env::var("RUST_LOG")
            .ok()
            .as_deref()
            .map(log::LevelFilter::from_str)
            .and_then(Result::ok)
            .unwrap_or(self.default_level);

        self
    }

    #[must_use = "You must call init() to before logging"]
    pub fn level(mut self, level: LevelFilter) -> CallLogger {
        self.default_level = level;
        self
    }

    #[must_use = "You must call init() to before logging"]
    pub fn call_target(mut self, call_target: String) -> CallLogger {
        self.call_target = call_target;
        self
    }

    pub fn init(mut self) -> Result<(), SetLoggerError> {
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
    fn enabled(&self, metadata: &Metadata) -> bool {
        todo!()
    }

    fn log(&self, record: &Record) {
        todo!()
    }

    fn flush(&self) { }
}
