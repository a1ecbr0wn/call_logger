use call_logger::CallLogger;
use log::LevelFilter;

fn main() {
    let _ = CallLogger::new()
        .with_level(LevelFilter::Info)
        .with_call_target("examples/log_to_discord")
        .init();
    log::info!("Hello discord")
}
