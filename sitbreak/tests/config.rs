use std::time::Duration;

use sitbreak::config::Config;
use sitbreak::i18n::Language;

#[test]
fn defaults_when_empty() {
    let cfg = Config::parse("").unwrap();
    assert_eq!(cfg.work, Duration::from_secs(3600));
    assert_eq!(cfg.break_len, Duration::from_secs(900));
    assert_eq!(cfg.idle_pause, Duration::from_secs(60));
}

#[test]
fn seconds_override_minutes() {
    let cfg = Config::parse("work_seconds = 12\nbreak_seconds = 5\n").unwrap();
    assert_eq!(cfg.work, Duration::from_secs(12));
    assert_eq!(cfg.break_len, Duration::from_secs(5));
}

#[test]
fn rejects_zero() {
    assert!(Config::parse("work_minutes = 0\n").is_err());
}

#[test]
fn language_setting() {
    let cfg = Config::parse("language = \"zh\"\n").unwrap();
    assert_eq!(cfg.language, Language::Zh);
    assert!(Config::parse("language = \"fr\"\n").is_err());
}
