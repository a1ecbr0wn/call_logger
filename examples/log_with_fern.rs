use call_logger::CallLogger;
use fern::Dispatch;
use log::{info, LevelFilter, Log};

/// This is an example that shows how `call_logger` can be chained with the [`fern`] logging framework.
///
/// ```
/// cargo run --example log_with_fern
/// ```
fn main() {
    let call_logger: Box<dyn Log + 'static> = Box::new(
        CallLogger::new()
            .format(|_, message, _| message.to_string())
            .with_level(LevelFilter::Info)
            .echo(),
    );
    let _ = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}|{}|{} {}",
                chrono::Local::now().format("%H:%M:%S %Y-%m-%d"),
                record.target(),
                record.level(),
                message
            ))
        })
        .chain(call_logger)
        .apply();

    info!("Hello fern");
}
