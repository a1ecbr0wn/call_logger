use call_logger::CallLogger;
use log::LevelFilter;

/// Example of calling a script.  The script this is calling uses the JSON formatted log message to send the message on
/// to a Discord Webhook.  To run this example you will need to set up a Discord Webhook and store the URL in an
/// environment variable called `CALL_LOGGER_DISCORD` which is picked up by the script.  When you run this example it
/// writes a message in the channel that your Webhook is linked to.
///
/// ```
/// cargo run --example log_to_discord_script
/// ```
fn main() {
    let _ = CallLogger::new()
        .with_level(LevelFilter::Info)
        .with_call_target("examples/log_to_discord")
        .init();
    log::info!("Hello discord")
}
