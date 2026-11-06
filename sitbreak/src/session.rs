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
    boot_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Persisted {
    phase: String,
    paused: bool,
    work_ms: u64,
    break_remaining_ms: Option<u64>,
    /// Kernel boot id. A different id means the machine rebooted, so the timer starts over.
    #[serde(default)]
    boot_id: Option<String>,
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
            boot_id: current_boot_id(),
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
        let boot_id = current_boot_id();
        let (session, reset) = Self::load_for_boot(path, config, now, boot_id.as_deref())?;
        if reset {
            eprintln!("sitbreak: new boot detected, timer reset");
        }
        Ok(session)
    }

    pub fn load_for_boot(
        path: &Path,
        config: &Config,
        now: Instant,
        boot_id: Option<&str>,
    ) -> Result<(Self, bool)> {
        let Some(boot_id) = boot_id else {
            return Ok((Self::new(config), true));
        };
        if !path.exists() {
            let mut session = Self::new(config);
            session.boot_id = Some(boot_id.to_string());
            return Ok((session, false));
        }
        let text = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let persisted: Persisted = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        if persisted.boot_id.as_deref() != Some(boot_id) {
            let mut session = Self::new(config);
            session.boot_id = Some(boot_id.to_string());
            return Ok((session, true));
        }
        let mut session = Self::new(config);
        session.boot_id = Some(boot_id.to_string());
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
        Ok((session, false))
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
            boot_id: self.boot_id.clone(),
        };
        let body = serde_json::to_vec_pretty(&persisted)?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, body).with_context(|| format!("failed to write {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("failed to save {}", path.display()))?;
        Ok(())
    }

    pub fn set_boot_id(&mut self, boot_id: impl Into<String>) {
        self.boot_id = Some(boot_id.into());
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

fn current_boot_id() -> Option<String> {
    let text = fs::read_to_string("/proc/sys/kernel/random/boot_id").ok()?;
    let id = text.trim();
    if id.is_empty() {
        None
    } else {
        Some(id.to_string())
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
