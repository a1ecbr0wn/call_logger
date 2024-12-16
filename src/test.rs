use super::*;
use log::{
    info,
    kv::{Source, ToKey, ToValue},
    Level,
};
use std::{
    fs::{read_to_string, remove_file},
    thread, time,
};

#[test]
fn test_log() {
    let filename = "test_log.log";
    let logger = CallLogger::new().with_call_target(format!("scripts/to_file.sh {}", filename));
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Error)
            .build(),
    );
    for _ in 0..20 {
        if let Ok(test) = read_to_string(filename) {
            println!("{test}");
            assert!(test.contains("\"level\":\"ERROR\""));
            assert!(test.contains("\"file\":\"src/lib.rs\""));
            assert!(test.contains("\"module_path\":\"call_logger::test\""));
            assert!(test.contains("\"msg\":\"test message\""));
            remove_file(filename).unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("Failed to detect the log message");
}

#[test]
fn test_log_to_file() {
    let filename = "test_log_to_file.log";
    let logger = CallLogger::new()
        .with_level(LevelFilter::Error)
        .with_call_target("echo")
        .to_file(filename);
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Error)
            .build(),
    );
    for _ in 0..20 {
        if let Ok(test) = read_to_string(filename) {
            assert!(test.contains("\"level\":\"ERROR\""));
            assert!(test.contains("\"file\":\"src/lib.rs\""));
            assert!(test.contains("\"module_path\":\"call_logger::test\""));
            assert!(test.contains("\"msg\":\"test message\""));
            remove_file(filename).unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("Failed to detect the log message");
}

#[test]
fn test_log_default() {
    let logger = CallLogger::default();
    assert_eq!(logger.level, LevelFilter::Trace);
    assert_eq!(logger.call_target, "echo");
    let _ = logger.init();
    info!("test message");
}

#[test]
#[cfg(feature = "timestamps")]
fn test_log_format_ts() {
    let filename = "test_log_format_ts.log";
    let logger = CallLogger::default()
        .format(|timestamp, message, record| {
            format!(
                "{{\"ts\":\"{}\",\"level\":\"{}\",\"file\":\"{}\",\"module_path\":\"{}\",\"msg\":\"{}\"}}",
                timestamp,
                record.level(),
                record.file().unwrap_or_default(),
                record.module_path().unwrap_or_default(),
                message
            )
        })
        .to_file(filename);
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Error)
            .build(),
    );
    for _ in 0..20 {
        if let Ok(test) = read_to_string(filename) {
            println!("{test}");
            assert!(test.contains("\"ts\":\""));
            assert!(test.contains("\"level\":\"ERROR\""));
            assert!(test.contains("\"file\":\"src/lib.rs\""));
            assert!(test.contains("\"module_path\":\"call_logger::test\""));
            assert!(test.contains("\"msg\":\"test message\""));
            remove_file(filename).unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("Failed to detect the log message");
}

#[test]
#[cfg(not(feature = "timestamps"))]
fn test_log_format_no_ts() {
    let filename = "test_log_format_no_ts.log";
    let logger = CallLogger::default()
        .format(|message, record| {
            format!(
                "{{\"level\":\"{}\",\"file\":\"{}\",\"module_path\":\"{}\",\"msg\":\"{}\"}}",
                record.level(),
                record.file().unwrap_or_default(),
                record.module_path().unwrap_or_default(),
                message
            )
        })
        .to_file(filename);
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Error)
            .build(),
    );
    for _ in 0..20 {
        if let Ok(test) = read_to_string(filename) {
            println!("{test}");
            assert!(!test.contains("\"ts\":\""));
            assert!(test.contains("\"level\":\"ERROR\""));
            assert!(test.contains("\"file\":\"src/lib.rs\""));
            assert!(test.contains("\"module_path\":\"call_logger::test\""));
            assert!(test.contains("\"msg\":\"test message\""));
            remove_file(filename).unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("Failed to detect the log message");
}

#[test]
fn test_log_quoted_string() {
    let logger = CallLogger::default();
    assert_eq!(logger.level, LevelFilter::Trace);
    assert_eq!(logger.call_target, "echo".to_string());
    let msg = r#"{ "message": "test message" }"#;
    logger.log(&Record::builder().args(format_args!("{msg}")).build());
}

#[test]
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
fn test_level() {
    let logger = CallLogger::default().with_level(LevelFilter::Info);
    assert_eq!(logger.level, LevelFilter::Info);
}

#[test]
fn test_with_level_for_match() {
    let logger = CallLogger::default()
        .with_level(LevelFilter::Info)
        .with_level_for("test", LevelFilter::Warn);
    assert_eq!(logger.level, LevelFilter::Info);
    let trace_metadata = Metadata::builder()
        .level(Level::Trace)
        .target("call_logger::test::module")
        .build();
    let debug_metadata = Metadata::builder()
        .level(Level::Debug)
        .target("call_logger::test::module")
        .build();
    let info_metadata = Metadata::builder()
        .level(Level::Info)
        .target("call_logger::test::module")
        .build();
    let warn_metadata = Metadata::builder()
        .level(Level::Warn)
        .target("call_logger::test::module")
        .build();
    let error_metadata = Metadata::builder()
        .level(Level::Error)
        .target("call_logger::test::module")
        .build();
    assert!(!logger.enabled(&trace_metadata));
    assert!(!logger.enabled(&debug_metadata));
    assert!(!logger.enabled(&info_metadata));
    assert!(logger.enabled(&warn_metadata));
    assert!(logger.enabled(&error_metadata));
}

#[test]
fn test_with_level_for_no_match() {
    let logger = CallLogger::default()
        .with_level(LevelFilter::Info)
        .with_level_for("test", LevelFilter::Warn);
    assert_eq!(logger.level, LevelFilter::Info);
    let trace_metadata = Metadata::builder()
        .level(Level::Trace)
        .target("call_logger::module")
        .build();
    let debug_metadata = Metadata::builder()
        .level(Level::Debug)
        .target("call_logger::module")
        .build();
    let info_metadata = Metadata::builder()
        .level(Level::Info)
        .target("call_logger::module")
        .build();
    let warn_metadata = Metadata::builder()
        .level(Level::Warn)
        .target("call_logger::module")
        .build();
    let error_metadata = Metadata::builder()
        .level(Level::Error)
        .target("call_logger::module")
        .build();
    assert!(!logger.enabled(&trace_metadata));
    assert!(!logger.enabled(&debug_metadata));
    assert!(logger.enabled(&info_metadata));
    assert!(logger.enabled(&warn_metadata));
    assert!(logger.enabled(&error_metadata));
}

#[test]
fn test_call_target() {
    let logger = CallLogger::default().with_call_target("wc");
    assert_eq!(logger.call_target, "wc".to_string());
}

#[test]
#[cfg(feature = "timestamps")]
fn test_epoch_ms_timestamp() {
    let logger = CallLogger::default().with_epoch_ms_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::UtcEpochMs);
}

#[test]
#[cfg(feature = "timestamps")]
fn test_epoch_us_timestamp() {
    let logger = CallLogger::default().with_epoch_us_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::UtcEpochUs);
}

#[test]
#[cfg(feature = "timestamps")]
fn test_utc_timestamp() {
    let logger = CallLogger::default().with_utc_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::Utc);
}

#[test]
#[cfg(feature = "timestamps")]
fn test_local_timestamp() {
    let logger = CallLogger::default().with_local_timestamp();
    assert_eq!(logger.timestamp, TimestampFormat::Local);
}

#[test]
fn test_kv_log() {
    let filename = "test_kv_log.log";
    let logger = CallLogger::default().with_call_target(format!("scripts/to_file.sh {}", filename));
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .key_values(&TestSource::new("test_item", "test_value"))
            .file(Some("src/lib.rs"))
            .module_path(Some("call_logger::test"))
            .level(Level::Info)
            .build(),
    );
    thread::sleep(time::Duration::from_millis(20));
    for _ in 0..20 {
        if let Ok(test) = read_to_string(filename) {
            assert!(test.contains("\"test_item\":\"test_value\""));
            assert!(test.contains("\"level\":\"INFO\""));
            assert!(test.contains("\"file\":\"src/lib.rs\""));
            assert!(test.contains("\"module_path\":\"call_logger::test\""));
            assert!(test.contains("\"msg\":\"test message\""));
            remove_file(filename).unwrap();
            thread::sleep(time::Duration::from_millis(10));
            return;
        } else {
            thread::sleep(time::Duration::from_millis(100));
        }
    }
    panic!("{filename} cannot be read");
}

#[test]
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
    let logger = CallLogger::new()
        .with_level(LevelFilter::Error)
        .with_call_target(format!("{url}/test"));
    logger.log(
        &Record::builder()
            .args(format_args!("test message"))
            .key_values(&TestSource::new("test_item", "test_value"))
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

impl TestSource {
    fn new<T>(key: T, value: T) -> TestSource
    where
        T: Into<String>,
    {
        TestSource {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl Source for TestSource {
    fn visit<'kvs>(&'kvs self, visitor: &mut dyn VisitSource<'kvs>) -> Result<(), Error> {
        visitor.visit_pair(self.key.to_key(), self.value.to_value())
    }
}
