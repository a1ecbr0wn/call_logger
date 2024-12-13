use call_logger::CallLogger;
use log::LevelFilter;

fn main() {
    if let Ok(endpoint) = std::env::var("CALL_LOGGER_DISCORD") {
        let _ = CallLogger::new()
            .with_call_target(endpoint)
            .with_level(LevelFilter::Info)
            .format(|timestamp, message, record| {
                format!(
                    "{{ \"content\": \"{} [{}] {} - {}\" }}",
                    timestamp,
                    record.level(),
                    record.module_path().unwrap_or_default(),
                    message
                )
            })
            .init();
        log::info!("Hello discord");
    }
}
