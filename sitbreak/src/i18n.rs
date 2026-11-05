use std::env;

use anyhow::{bail, Result};

/// Resolved interface language. `auto` in the config is resolved before use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    En,
    Zh,
}

impl Language {
    pub fn detect() -> Self {
        let raw = env::var("LC_ALL")
            .or_else(|_| env::var("LC_MESSAGES"))
            .or_else(|_| env::var("LANG"))
            .unwrap_or_default();
        if raw.to_ascii_lowercase().starts_with("zh") {
            Self::Zh
        } else {
            Self::En
        }
    }

    pub fn parse_setting(value: &str) -> Result<Self> {
        match value {
            "auto" => Ok(Self::detect()),
            "en" => Ok(Self::En),
            "zh" => Ok(Self::Zh),
            other => bail!("{}", invalid_language(Self::detect(), other)),
        }
    }
}

pub struct LockCopy {
    pub title: &'static str,
    pub hint: &'static str,
}

pub fn lock_copy(language: Language, preview: bool) -> LockCopy {
    match (language, preview) {
        (Language::Zh, true) => LockCopy {
            title: "预览锁屏",
            hint: "按 Esc 或等待倒计时结束",
        },
        (Language::Zh, false) => LockCopy {
            title: "该休息了",
            hint: "离开座位，看看远处。结束前键盘和鼠标不可用",
        },
        (Language::En, true) => LockCopy {
            title: "Break screen preview",
            hint: "Press Esc, or wait for the countdown",
        },
        (Language::En, false) => LockCopy {
            title: "Time for a break",
            hint: "Step away and look into the distance. Input stays locked.",
        },
    }
}

pub fn help_text(language: Language) -> &'static str {
    match language {
        Language::Zh => {
            "\
用法：
  sitbreak daemon     启动计时（一般由 systemd 拉起）
  sitbreak status     查看已使用时间和距离休息的时间
  sitbreak pause      暂停累计使用时间
  sitbreak resume     继续累计
  sitbreak reload     重新读取配置
  sitbreak preview N  预览锁屏 N 秒（1–60，默认 8），Esc 可退出

配置文件：~/.config/sitbreak/config.toml
语言：language = \"auto\"、\"en\" 或 \"zh\"
"
        }
        Language::En => {
            "\
Usage:
  sitbreak daemon     Run the timer (usually started by systemd)
  sitbreak status     Show used time and time until the next break
  sitbreak pause      Pause the work timer
  sitbreak resume     Resume the work timer
  sitbreak reload     Reread the config file
  sitbreak preview N  Show the break screen for N seconds (1-60, default 8). Esc exits

Config: ~/.config/sitbreak/config.toml
Language: language = \"auto\", \"en\", or \"zh\"
"
        }
    }
}

pub fn format_duration(language: Language, total_secs: u64) -> String {
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    match language {
        Language::Zh => {
            if hours > 0 {
                format!("{hours}小时{minutes:02}分{seconds:02}秒")
            } else if minutes > 0 {
                format!("{minutes}分{seconds:02}秒")
            } else {
                format!("{seconds}秒")
            }
        }
        Language::En => {
            if hours > 0 {
                format!("{hours}h {minutes:02}m {seconds:02}s")
            } else if minutes > 0 {
                format!("{minutes}m {seconds:02}s")
            } else {
                format!("{seconds}s")
            }
        }
    }
}

pub fn status_report(
    language: Language,
    phase: &str,
    paused: bool,
    work_seconds: u64,
    work_limit_seconds: u64,
    break_remaining_seconds: u64,
    break_limit_seconds: u64,
    idle_seconds: u64,
    message: &str,
) -> String {
    let state = match (language, phase, paused) {
        (Language::Zh, "breaking", _) => "休息中",
        (Language::Zh, "working", true) => "已暂停",
        (Language::Zh, "working", false) => "工作中",
        (Language::En, "breaking", _) => "On break",
        (Language::En, "working", true) => "Paused",
        (Language::En, "working", false) => "Working",
        (_, other, _) => other,
    };
    let used = format_duration(language, work_seconds);
    let limit = format_duration(language, work_limit_seconds);
    let mut lines = match language {
        Language::Zh => vec![
            format!("状态：{state}"),
            format!("已使用：{used} / {limit}"),
        ],
        Language::En => vec![
            format!("Status: {state}"),
            format!("Used: {used} / {limit}"),
        ],
    };
    if phase == "breaking" {
        let left = format_duration(language, break_remaining_seconds);
        let break_limit = format_duration(language, break_limit_seconds);
        lines.push(match language {
            Language::Zh => format!("剩余休息：{left} / {break_limit}"),
            Language::En => format!("Break left: {left} / {break_limit}"),
        });
    } else {
        let until = format_duration(language, work_limit_seconds.saturating_sub(work_seconds));
        lines.push(match language {
            Language::Zh => format!("距离休息：{until}"),
            Language::En => format!("Until break: {until}"),
        });
    }
    let idle = format_duration(language, idle_seconds);
    lines.push(match language {
        Language::Zh => format!("当前空闲：{idle}"),
        Language::En => format!("Idle: {idle}"),
    });
    let note = user_message(language, message);
    if !note.is_empty() {
        lines.push(note);
    }
    lines.join("\n")
}

/// Translate a daemon message code. Unknown text is returned unchanged.
pub fn user_message(language: Language, code: &str) -> String {
    let text = match (language, code) {
        (_, "") => "",
        (Language::Zh, "paused") => "已暂停计时",
        (Language::En, "paused") => "Work timer paused",
        (Language::Zh, "already-paused") => "已经暂停",
        (Language::En, "already-paused") => "Already paused",
        (Language::Zh, "break-in-progress") => "休息进行中，不能暂停",
        (Language::En, "break-in-progress") => "A break is in progress and cannot be paused",
        (Language::Zh, "resumed") => "已继续计时",
        (Language::En, "resumed") => "Work timer resumed",
        (Language::Zh, "not-paused") => "当前没有暂停",
        (Language::En, "not-paused") => "The timer is not paused",
        (Language::Zh, "reloaded") => "已重新加载配置",
        (Language::En, "reloaded") => "Config reloaded",
        (Language::Zh, "preview-not-integer") => "预览秒数必须是整数",
        (Language::En, "preview-not-integer") => "Preview seconds must be an integer",
        (Language::Zh, "preview-out-of-range") => "预览秒数需要在 1 到 60 之间",
        (Language::En, "preview-out-of-range") => "Preview seconds must be from 1 to 60",
        (Language::Zh, "empty-reply") => "守护进程没有返回结果",
        (Language::En, "empty-reply") => "The daemon returned an empty response",
        (Language::Zh, "bad-reply") => "守护进程返回了无法解析的结果",
        (Language::En, "bad-reply") => "The daemon returned a response that could not be parsed",
        (Language::Zh, "invalid-toml") => "配置不是合法的 TOML",
        (Language::En, "invalid-toml") => "Config is not valid TOML",
        (Language::Zh, "startup-interrupted") => "启动被中断",
        (Language::En, "startup-interrupted") => "Startup was interrupted",
        _ if code.starts_with("unknown:") => {
            return match language {
                Language::Zh => format!("未知命令 {}", &code["unknown:".len()..]),
                Language::En => format!("Unknown command {}", &code["unknown:".len()..]),
            };
        }
        _ => return code.to_string(),
    };
    text.to_string()
}

pub fn invalid_language(language: Language, value: &str) -> String {
    match language {
        Language::Zh => format!("language 只能是 auto、en 或 zh，当前是 {value}"),
        Language::En => format!("language must be auto, en, or zh, got {value}"),
    }
}

pub fn out_of_range(language: Language, name: &str, min: u64, max: u64, value: u64) -> String {
    match language {
        Language::Zh => format!("{name} 必须在 {min} 到 {max} 之间，当前是 {value}"),
        Language::En => format!("{name} must be from {min} to {max}, got {value}"),
    }
}

pub fn cannot_connect(language: Language, socket: &str) -> String {
    match language {
        Language::Zh => {
            format!("连不上 {socket}。先运行 sitbreak daemon，或执行 install.sh")
        }
        Language::En => {
            format!("Cannot connect to {socket}. Start sitbreak daemon, or run install.sh")
        }
    }
}

pub fn io_failed(language: Language, action: &str, path: &str) -> String {
    match (language, action) {
        (Language::Zh, "read") => format!("无法读取 {path}"),
        (Language::En, "read") => format!("Failed to read {path}"),
        (Language::Zh, "write") => format!("无法写入 {path}"),
        (Language::En, "write") => format!("Failed to write {path}"),
        (Language::Zh, "parse") => format!("无法解析 {path}"),
        (Language::En, "parse") => format!("Failed to parse {path}"),
        (Language::Zh, "save") => format!("无法保存 {path}"),
        (Language::En, "save") => format!("Failed to save {path}"),
        (Language::Zh, "listen") => format!("无法监听 {path}"),
        (Language::En, "listen") => format!("Failed to listen on {path}"),
        (Language::Zh, _) => format!("无法处理 {path}"),
        (Language::En, _) => format!("Failed to process {path}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_durations() {
        assert_eq!(format_duration(Language::En, 3661), "1h 01m 01s");
        assert_eq!(format_duration(Language::Zh, 3661), "1小时01分01秒");
        assert_eq!(format_duration(Language::En, 5), "5s");
    }

    #[test]
    fn translates_message_codes() {
        assert_eq!(user_message(Language::En, "paused"), "Work timer paused");
        assert_eq!(user_message(Language::Zh, "paused"), "已暂停计时");
        assert_eq!(
            user_message(Language::En, "unknown:foo"),
            "Unknown command foo"
        );
    }
}
