use call_logger::CallLogger;
use log::{error, info, Level, LevelFilter};
use multi_log::MultiLogger;

/// Example of log filtering in action.  This example uses a `MultiLogger` to send each log line to two `CallLogger`s.
/// One of the `CallLogger`s filters Info messages and below and the other filters Error messages and below.  When
/// something is logged at Info level, only one of the loggers logs it so it appears once, however when something is
/// logged at Error level, both loggers log it so it appears twice.
///
/// ```
/// cargo run --example log_filtering
/// ```
fn main() {
    let l1 = CallLogger::new().with_level(LevelFilter::Info);
    let l2 = CallLogger::new().with_level(LevelFilter::Error);
    let _ = MultiLogger::init(vec![Box::new(l1), Box::new(l2)], Level::Trace);

    // Only one log message at INFO level will be printed from l1 because l2 is filtered for error
    info!("only one log message at INFO level");
    // but both loggers
    error!("two log messages at ERROR level");
}
