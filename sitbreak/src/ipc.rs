use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::i18n::{self, Language};
use crate::session::{Phase, Session};

#[derive(Debug)]
pub struct Shared {
    pub config: Config,
    pub session: Session,
    pub idle: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Reply {
    pub ok: bool,
    pub message: String,
    pub phase: String,
    pub paused: bool,
    pub work_seconds: u64,
    pub work_limit_seconds: u64,
    pub break_remaining_seconds: u64,
    pub break_remaining_ms: u64,
    pub break_limit_seconds: u64,
    pub idle_seconds: u64,
}

pub fn bind_socket(path: &Path) -> Result<UnixListener> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    match UnixStream::connect(path) {
        Ok(_) => bail!("sitbreak is already running"),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            let _ = fs::remove_file(path);
        }
    }
    let listener = UnixListener::bind(path).with_context(|| {
        i18n::io_failed(Language::detect(), "listen", &path.display().to_string())
    })?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

pub fn serve(listener: UnixListener, shared: Arc<Mutex<Shared>>, config_path: PathBuf) {
    for conn in listener.incoming() {
        let Ok(stream) = conn else { continue };
        if let Err(err) = handle_client(stream, &shared, &config_path) {
            eprintln!("sitbreak: control command failed: {err:#}");
        }
    }
}

fn handle_client(stream: UnixStream, shared: &Mutex<Shared>, config_path: &Path) -> Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let reply = dispatch(line.trim(), shared, config_path);
    let mut stream = reader.into_inner();
    writeln!(stream, "{}", serde_json::to_string(&reply)?)?;
    Ok(())
}

fn dispatch(cmd: &str, shared: &Mutex<Shared>, config_path: &Path) -> Reply {
    let mut guard = shared.lock().unwrap_or_else(|err| err.into_inner());
    let mut ok = true;
    let message = match cmd {
        "status" | "" => String::new(),
        "pause" => {
            if guard.session.phase == Phase::Breaking {
                ok = false;
                "break-in-progress".to_string()
            } else if guard.session.paused {
                "already-paused".to_string()
            } else {
                guard.session.paused = true;
                "paused".to_string()
            }
        }
        "resume" => {
            if guard.session.paused {
                guard.session.paused = false;
                "resumed".to_string()
            } else {
                "not-paused".to_string()
            }
        }
        "reload" => match Config::load_or_create(config_path) {
            Ok(config) => {
                guard.session.apply_config(&config);
                guard.config = config;
                "reloaded".to_string()
            }
            Err(err) => {
                ok = false;
                err.to_string()
            }
        },
        _ => {
            ok = false;
            format!("unknown:{cmd}")
        }
    };
    snapshot(&guard, ok, message)
}

pub fn snapshot(shared: &Shared, ok: bool, message: String) -> Reply {
    let now = Instant::now();
    Reply {
        ok,
        message,
        phase: match shared.session.phase {
            Phase::Working => "working",
            Phase::Breaking => "breaking",
        }
        .to_string(),
        paused: shared.session.paused,
        work_seconds: shared.session.work.as_secs(),
        work_limit_seconds: shared.session.work_limit.as_secs(),
        break_remaining_seconds: shared
            .session
            .break_remaining(now)
            .unwrap_or(Duration::ZERO)
            .as_secs(),
        break_remaining_ms: u64::try_from(
            shared
                .session
                .break_remaining(now)
                .unwrap_or(Duration::ZERO)
                .as_millis(),
        )
        .unwrap_or(u64::MAX),
        break_limit_seconds: shared.session.break_len.as_secs(),
        idle_seconds: shared.idle.as_secs(),
    }
}

pub fn request(socket: &Path, cmd: &str, language: Language) -> Result<Reply> {
    let mut stream = UnixStream::connect(socket)
        .with_context(|| i18n::cannot_connect(language, &socket.display().to_string()))?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    writeln!(stream, "{cmd}")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    if line.is_empty() {
        bail!("{}", i18n::user_message(language, "empty-reply"));
    }
    serde_json::from_str(&line).context(i18n::user_message(language, "bad-reply"))
}

pub fn print_reply(reply: &Reply, language: Language) -> Result<()> {
    let report = i18n::status_report(
        language,
        &reply.phase,
        reply.paused,
        reply.work_seconds,
        reply.work_limit_seconds,
        reply.break_remaining_seconds,
        reply.break_limit_seconds,
        reply.idle_seconds,
        &reply.message,
    );
    if !reply.ok {
        bail!("{report}");
    }
    println!("{report}");
    Ok(())
}
