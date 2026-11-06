use std::time::{Duration, Instant};

use sitbreak::config::Config;
use sitbreak::i18n::Language;
use sitbreak::session::{Effect, Phase, Session};

fn config(work: u64, break_len: u64, idle_pause: u64) -> Config {
    Config {
        work: Duration::from_secs(work),
        break_len: Duration::from_secs(break_len),
        idle_pause: Duration::from_secs(idle_pause),
        language: Language::En,
    }
}

fn session(work: u64, break_len: u64, idle_pause: u64) -> Session {
    Session::new(&config(work, break_len, idle_pause))
}

#[test]
fn accumulates_while_active() {
    let mut session = session(60, 15, 60);
    let now = Instant::now();
    let effect = session.tick(Duration::from_secs(4), Duration::from_secs(5), now);
    assert_eq!(effect, Effect::None);
    assert_eq!(session.work, Duration::from_secs(4));
    assert_eq!(session.phase, Phase::Working);
}

#[test]
fn pauses_when_idle_or_manually_paused() {
    let mut session = session(60, 15, 60);
    let now = Instant::now();
    session.tick(Duration::from_secs(10), Duration::from_secs(60), now);
    assert_eq!(session.work, Duration::ZERO);
    session.paused = true;
    session.tick(Duration::from_secs(10), Duration::ZERO, now);
    assert_eq!(session.work, Duration::ZERO);
}

#[test]
fn starts_and_ends_break() {
    let mut session = session(5, 15, 60);
    let start = Instant::now();
    let effect = session.tick(Duration::from_secs(5), Duration::ZERO, start);
    assert_eq!(effect, Effect::StartedBreak);
    assert_eq!(session.phase, Phase::Breaking);
    assert_eq!(session.work, Duration::ZERO);

    let mid = session.tick(
        Duration::from_secs(1),
        Duration::ZERO,
        start + Duration::from_secs(14),
    );
    assert_eq!(mid, Effect::None);
    assert_eq!(session.phase, Phase::Breaking);

    let end = session.tick(
        Duration::ZERO,
        Duration::ZERO,
        start + Duration::from_secs(15),
    );
    assert_eq!(end, Effect::EndedBreak);
    assert_eq!(session.phase, Phase::Working);
}

#[test]
fn long_gap_does_not_count_as_usage() {
    let mut session = session(3600, 900, 60);
    let now = Instant::now();
    session.tick(Duration::from_secs(120), Duration::ZERO, now);
    assert_eq!(session.work, Duration::ZERO);
    assert_eq!(session.phase, Phase::Working);
}

#[test]
fn reboot_discards_saved_time() {
    let dir = std::env::temp_dir().join(format!(
        "sitbreak-boot-test-{}-{}",
        std::process::id(),
        Instant::now().elapsed().as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("state.json");
    let config = config(3600, 900, 60);
    let now = Instant::now();
    let mut saved = session(3600, 900, 60);
    saved.work = Duration::from_secs(3500);
    saved.set_boot_id("boot-a");
    saved.store(&path, now).unwrap();

    let (same_boot, reset) = Session::load_for_boot(&path, &config, now, Some("boot-a")).unwrap();
    assert!(!reset);
    assert_eq!(same_boot.work, Duration::from_secs(3500));

    let (new_boot, reset) = Session::load_for_boot(&path, &config, now, Some("boot-b")).unwrap();
    assert!(reset);
    assert_eq!(new_boot.work, Duration::ZERO);
    assert_eq!(new_boot.phase, Phase::Working);

    let _ = std::fs::remove_dir_all(&dir);
}
