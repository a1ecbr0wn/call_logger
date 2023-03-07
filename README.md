<!-- markdownlint-configure-file {
  "MD033": false,
  "MD041": false
} -->

# call_logger

[![Crates.io](https://img.shields.io/crates/l/call_logger)](https://github.com/a1ecbr0wn/call_logger/blob/main/LICENSE) [![Crates.io](https://img.shields.io/crates/v/call_logger)](https://crates.io/crates/call_logger) [![Build Status](https://github.com/a1ecbr0wn/call_logger/workflows/CI%20Build/badge.svg)](https://github.com/a1ecbr0wn/call_logger/actions/workflows/build.yml) [![docs.rs](https://img.shields.io/docsrs/call_logger)](https://docs.rs/call_logger) [![dependency status](https://deps.rs/repo/github/a1ecbr0wn/call_logger/status.svg)](https://deps.rs/repo/github/a1ecbr0wn/call_logger)

A logger that calls another application for every logged item, passing a json formatted string that contains the details of the log event.

## Usage

Use of the builder model to set up the logger to call a script called `store_log`:

``` rust
use call_logger::CallLogger;
use log::LevelFilter;

fn main() {
    let _ = CallLogger::new()
        .with_level(LevelFilter::Info)
        .with_call_target("store_log".to_string())
        .with_local_timestamp()
        .init();
    log::info!("Hello logging world")
}
```

## Contribute

This is just an early attempt at a general purpose logger that calls out to another process.  If you have any ideas for missing features, please raise an [issue](https://github.com/a1ecbr0wn/call_logger/issues) or a PR.
