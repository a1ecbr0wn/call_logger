use call_logger::CallLogger;
use log::LevelFilter;

fn main() {
    let l1 = CallLogger::new().with_level(LevelFilter::Info);
    let l2 = CallLogger::new().with_level(LevelFilter::Error);
    let loggers: Vec<Box<dyn log::Log>> = vec![Box::new(l1), Box::new(l2)];

    let logger = multi_log::MultiLogger::new(loggers);
    log::set_max_level(log::Level::Trace.to_level_filter());
    log::set_boxed_logger(Box::new(logger)).unwrap();

    log::info!("only one log message at INFO level");
}
