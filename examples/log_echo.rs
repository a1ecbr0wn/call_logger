use call_logger::CallLogger;
use log::{info, LevelFilter};

/// Echo a log message to the console
///
/// ```
/// cargo run --example log_echo
/// ```
fn main() {
    let _ = CallLogger::new()
        .with_level(LevelFilter::Info)
        .echo()
        .init();

    // The call to echo will be written to stdout and then the log line will be
    // written to the default call target which is `echo`.
    info!("msg");
}
