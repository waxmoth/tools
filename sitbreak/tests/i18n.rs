use sitbreak::i18n::{self, Language};

#[test]
fn formats_durations() {
    assert_eq!(i18n::format_duration(Language::En, 3661), "1h 01m 01s");
    assert_eq!(i18n::format_duration(Language::Zh, 3661), "1小时01分01秒");
    assert_eq!(i18n::format_duration(Language::En, 5), "5s");
}

#[test]
fn translates_message_codes() {
    assert_eq!(
        i18n::user_message(Language::En, "paused"),
        "Work timer paused"
    );
    assert_eq!(i18n::user_message(Language::Zh, "paused"), "已暂停计时");
    assert_eq!(
        i18n::user_message(Language::En, "unknown:foo"),
        "Unknown command foo"
    );
}
