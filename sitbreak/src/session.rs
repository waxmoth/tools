use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Working,
    Breaking,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    StartedBreak,
    EndedBreak,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub phase: Phase,
    pub paused: bool,
    pub work: Duration,
    pub break_deadline: Option<Instant>,
    pub work_limit: Duration,
    pub break_len: Duration,
    pub idle_pause: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    phase: String,
    paused: bool,
    work_ms: u64,
    break_remaining_ms: Option<u64>,
}

impl Session {
    pub fn new(config: &Config) -> Self {
        Self {
            phase: Phase::Working,
            paused: false,
            work: Duration::ZERO,
            break_deadline: None,
            work_limit: config.work,
            break_len: config.break_len,
            idle_pause: config.idle_pause,
        }
    }

    pub fn apply_config(&mut self, config: &Config) {
        self.work_limit = config.work;
        self.break_len = config.break_len;
        self.idle_pause = config.idle_pause;
        if self.phase == Phase::Working && self.work >= self.work_limit {
            self.enter_break(Instant::now());
        }
    }

    pub fn load(path: &Path, config: &Config, now: Instant) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new(config));
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let persisted: Persisted = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let mut session = Self::new(config);
        session.paused = persisted.paused;
        if persisted.phase == "breaking" {
            let remaining = Duration::from_millis(persisted.break_remaining_ms.unwrap_or(0));
            if remaining.is_zero() {
                session.phase = Phase::Working;
            } else {
                session.phase = Phase::Breaking;
                session.break_deadline = Some(now + remaining);
            }
        } else {
            session.work = Duration::from_millis(persisted.work_ms);
            if session.work >= session.work_limit {
                session.enter_break(now);
            }
        }
        Ok(session)
    }

    pub fn store(&self, path: &Path, now: Instant) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let persisted = Persisted {
            phase: match self.phase {
                Phase::Working => "working",
                Phase::Breaking => "breaking",
            }
            .to_string(),
            paused: self.paused,
            work_ms: u64::try_from(self.work.as_millis()).unwrap_or(u64::MAX),
            break_remaining_ms: self.break_remaining(now).map(|d| d.as_millis() as u64),
        };
        let body = serde_json::to_vec_pretty(&persisted)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, body).with_context(|| format!("failed to write {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("failed to save {}", path.display()))?;
        Ok(())
    }

    pub fn break_remaining(&self, now: Instant) -> Option<Duration> {
        if self.phase != Phase::Breaking {
            return None;
        }
        Some(
            self.break_deadline
                .map(|deadline| deadline.saturating_duration_since(now))
                .unwrap_or(Duration::ZERO),
        )
    }

    pub fn tick(&mut self, delta: Duration, idle: Duration, now: Instant) -> Effect {
        let delta = clamp_delta(delta);
        match self.phase {
            Phase::Breaking => {
                let finished = self
                    .break_deadline
                    .map(|deadline| now >= deadline)
                    .unwrap_or(true);
                if finished {
                    self.phase = Phase::Working;
                    self.work = Duration::ZERO;
                    self.break_deadline = None;
                    Effect::EndedBreak
                } else {
                    Effect::None
                }
            }
            Phase::Working => {
                if !self.paused && idle < self.idle_pause {
                    self.work = self.work.saturating_add(delta);
                }
                if self.work >= self.work_limit {
                    self.enter_break(now);
                    Effect::StartedBreak
                } else {
                    Effect::None
                }
            }
        }
    }

    fn enter_break(&mut self, now: Instant) {
        self.phase = Phase::Breaking;
        self.work = Duration::ZERO;
        self.break_deadline = Some(now + self.break_len);
    }
}

fn clamp_delta(delta: Duration) -> Duration {
    if delta > Duration::from_secs(30) {
        Duration::ZERO
    } else if delta > Duration::from_secs(5) {
        Duration::from_secs(5)
    } else {
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(work: u64, break_len: u64, idle_pause: u64) -> Session {
        Session::new(&Config {
            work: Duration::from_secs(work),
            break_len: Duration::from_secs(break_len),
            idle_pause: Duration::from_secs(idle_pause),
            language: crate::i18n::Language::En,
        })
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
}
