use call_logger::CallLogger;
use log::{error, info, Level, LevelFilter};
use multi_log::MultiLogger;

fn main() {
    let l1 = CallLogger::new().with_level(LevelFilter::Info);
    let l2 = CallLogger::new().with_level(LevelFilter::Error);
    let _ = MultiLogger::init(vec![Box::new(l1), Box::new(l2)], Level::Trace);

    // Only one log message at INFO level will be printed from l1 because l2 is filtered for error
    info!("only one log message at INFO level");
    // but both loggers
    error!("two log messages at ERROR level");
}
