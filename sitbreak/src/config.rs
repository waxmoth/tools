use std::fs;
use std::path::Path;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::i18n::{self, Language};

pub const DEFAULT_TOML: &str = "\
# Work time before a forced break. work_seconds overrides this.
work_minutes = 60

# Break length. break_seconds overrides this.
break_minutes = 15

# Pause the work timer after this many seconds without keyboard or mouse input.
idle_pause_seconds = 60

# Interface language: auto, en, or zh. auto follows LANG.
language = \"auto\"
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub work: Duration,
    pub break_len: Duration,
    pub idle_pause: Duration,
    pub language: Language,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            work: Duration::from_secs(60 * 60),
            break_len: Duration::from_secs(15 * 60),
            idle_pause: Duration::from_secs(60),
            language: Language::detect(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    #[serde(default)]
    work_minutes: Option<u64>,
    #[serde(default)]
    break_minutes: Option<u64>,
    #[serde(default)]
    work_seconds: Option<u64>,
    #[serde(default)]
    break_seconds: Option<u64>,
    #[serde(default)]
    idle_pause_seconds: Option<u64>,
    #[serde(default)]
    language: Option<String>,
}

impl Config {
    pub fn parse(text: &str) -> Result<Self> {
        let file: FileConfig =
            toml::from_str(text).context(i18n::user_message(Language::detect(), "invalid-toml"))?;
        let language = match file.language.as_deref() {
            None => Language::detect(),
            Some(value) => Language::parse_setting(value)?,
        };
        let work = duration_from(file.work_seconds, file.work_minutes, 60, "work", language)?;
        let break_len = duration_from(
            file.break_seconds,
            file.break_minutes,
            15,
            "break",
            language,
        )?;
        let idle_secs = file.idle_pause_seconds.unwrap_or(60);
        check_range(idle_secs, 1, 24 * 60 * 60, "idle_pause_seconds", language)?;
        Ok(Self {
            work,
            break_len,
            idle_pause: Duration::from_secs(idle_secs),
            language,
        })
    }

    pub fn load_or_create(path: &Path) -> Result<Self> {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, DEFAULT_TOML).with_context(|| {
                i18n::io_failed(Language::detect(), "write", &path.display().to_string())
            })?;
        }
        let text = fs::read_to_string(path).with_context(|| {
            i18n::io_failed(Language::detect(), "read", &path.display().to_string())
        })?;
        Self::parse(&text).with_context(|| {
            i18n::io_failed(Language::detect(), "parse", &path.display().to_string())
        })
    }
}

pub fn interface_language() -> Language {
    let Ok(paths) = crate::paths::Paths::resolve() else {
        return Language::detect();
    };
    if !paths.config.exists() {
        return Language::detect();
    }
    Config::load_or_create(&paths.config)
        .map(|config| config.language)
        .unwrap_or_else(|_| Language::detect())
}

fn duration_from(
    seconds: Option<u64>,
    minutes: Option<u64>,
    default_minutes: u64,
    name: &str,
    language: Language,
) -> Result<Duration> {
    if let Some(secs) = seconds {
        check_range(secs, 1, 24 * 60 * 60, &format!("{name}_seconds"), language)?;
        return Ok(Duration::from_secs(secs));
    }
    let mins = minutes.unwrap_or(default_minutes);
    check_range(mins, 1, 12 * 60, &format!("{name}_minutes"), language)?;
    Ok(Duration::from_secs(mins * 60))
}

fn check_range(value: u64, min: u64, max: u64, name: &str, language: Language) -> Result<()> {
    if value < min || value > max {
        bail!("{}", i18n::out_of_range(language, name, min, max, value));
    }
    Ok(())
}
