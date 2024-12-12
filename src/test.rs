use super::*;
use log::{
    info,
    kv::{Source, ToKey, ToValue},
    Level,
};
use serial_test::{parallel, serial};
use std::{
    fs::{read_to_string, remove_file},
    thread, time,
};

#[test]
#[serial]
fn test_log() {
    let logger = CallLogger::new()
        .with_level(LevelFilter::Error)
        .with_call_target("scripts/to_file.sh test_log.log".to_string());
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Error)
            .build(),
    );
    for _ in 0..20 {
        if let Ok(test) = read_to_string("test_log.log") {
            assert!(test.contains("\"level\": \"ERROR\""));
            assert!(test.contains("\"file\": \"src/lib.rs\""));
            assert!(test.contains("\"module_path\": \"call_logger::test\""));
            assert!(test.contains("\"msg\": \"test message\""));
            remove_file("test_log.log").unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("Failed to detect the log message");
}

#[test]
#[serial]
fn test_log_default() {
    let logger = CallLogger::default();
    assert_eq!(logger.level, LevelFilter::Trace);
    assert_eq!(logger.call_target, "echo".to_string());
    let _ = logger.init();
    info!("test message");
}

#[test]
#[parallel]
fn test_log_quoted_string() {
    let logger = CallLogger::default();
    assert_eq!(logger.level, LevelFilter::Trace);
    assert_eq!(logger.call_target, "echo".to_string());
    let msg = r#"{ "message": "test message" }"#;
    logger.log(&Record::builder().args(format_args!("{msg}")).build());
}

#[test]
#[parallel]
fn test_log_level_filter() {
    let logger = CallLogger::new().with_level(LevelFilter::Error);
    assert_eq!(logger.level, LevelFilter::Error);
    assert_eq!(logger.call_target, "echo".to_string());
    logger.log(
        &Record::builder()
            .args(format_args!("filtered message"))
            .build(),
    );
}

#[test]
#[parallel]
fn test_level() {
    let logger = CallLogger::default().with_level(LevelFilter::Info);
    assert_eq!(logger.level, LevelFilter::Info);
}

#[test]
#[parallel]
fn test_call_target() {
    let logger = CallLogger::default().with_call_target("wc".to_string());
    assert_eq!(logger.call_target, "wc".to_string());
}

#[test]
#[parallel]
#[cfg(feature = "timestamps")]
fn test_epoch_ms_timestamp() {
    let logger = CallLogger::default().with_epoch_ms_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::UtcEpochMs);
}

#[test]
#[parallel]
#[cfg(feature = "timestamps")]
fn test_epoch_us_timestamp() {
    let logger = CallLogger::default().with_epoch_us_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::UtcEpochUs);
}

#[test]
#[parallel]
#[cfg(feature = "timestamps")]
fn test_utc_timestamp() {
    let logger = CallLogger::default().with_utc_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::Utc);
}

#[test]
#[parallel]
#[cfg(feature = "timestamps")]
fn test_local_timestamp() {
    let logger = CallLogger::default().with_local_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::Local);
}

#[test]
#[serial]
fn test_kv_log() {
    let logger =
        CallLogger::default().with_call_target("scripts/to_file.sh test_kv_log.log".to_string());
    let source = TestSource {
        key: "test_item".to_string(),
        value: "test_value".to_string(),
    };
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .key_values(&source)
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Info)
            .build(),
    );
    thread::sleep(time::Duration::from_millis(20));
    if let Ok(test) = read_to_string("test_kv_log.log") {
        assert!(test.contains("\"test_item\": \"test_value\""));
        assert!(test.contains("\"level\": \"INFO\""));
        assert!(test.contains("\"file\": \"src/lib.rs\""));
        assert!(test.contains("\"module_path\": \"call_logger::test\""));
        assert!(test.contains("\"msg\": \"test message\""));
        remove_file("test_kv_log.log").unwrap();
        thread::sleep(time::Duration::from_millis(10));
    } else {
        panic!("test_kv_log.log cannot be read, consider increasing how long we wait for the test file to be written");
    }
}

#[test]
#[serial]
fn test_call_web_target_json() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/test")
        .with_status(200)
        .match_body(mockito::Matcher::PartialJsonString(
            "{\"level\": \"ERROR\"}".to_string(),
        ))
        .match_body(mockito::Matcher::PartialJsonString(
            "{\"module_path\": \"call_logger::test_call_web_target_json\"}".to_string(),
        ))
        .match_body(mockito::Matcher::PartialJsonString(
            "{\"test_item\": \"test_value\"}".to_string(),
        ))
        .with_body("msg")
        .create();
    let url = server.url();
    println!("url: {url}/test");
    let logger = CallLogger::new()
        .with_level(LevelFilter::Error)
        .with_call_target(format!("{url}/test").to_string())
        .echo();
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .key_values(&TestSource {
                key: "test_item".to_string(),
                value: "test_value".to_string(),
            })
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test_call_web_target_json"))
            .level(Level::Error)
            .build(),
    );
    mock.assert();
}

struct TestSource {
    key: String,
    value: String,
}

impl Source for TestSource {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
        visitor.visit_pair(self.key.to_key(), self.value.to_value())
    }
}
