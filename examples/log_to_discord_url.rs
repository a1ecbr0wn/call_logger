use call_logger::CallLogger;
use log::LevelFilter;

/// Example of calling a URL.  This example formats the log message to something that Discord understands via a Webhook.
/// To run this example you will need to set up a Discord Webhook and store the URL in an environment variable called
/// `CALL_LOGGER_DISCORD` which is picked up by the example.  When you run this it writes a message in the channel that
/// your Webhook is linked to.
///
/// ```
/// cargo run --example log_to_discord_url
/// ```
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
