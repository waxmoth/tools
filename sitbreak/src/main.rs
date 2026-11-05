mod config;
mod daemon;
mod i18n;
mod ipc;
mod lock;
mod paths;
mod session;
mod x11;

use std::env;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{bail, Context, Result};

use crate::lock::LockMode;

fn main() -> ExitCode {
    match dispatch() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("sitbreak: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn dispatch() -> Result<()> {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    match cmd.as_deref() {
        Some("daemon") => daemon::run(),
        Some("lock") => lock::run(LockMode::FollowDaemon),
        Some("preview") => {
            let language = config::interface_language();
            let seconds = match args.next() {
                Some(value) => value
                    .parse::<u64>()
                    .context(i18n::user_message(language, "preview-not-integer"))?,
                None => 8,
            };
            if !(1..=60).contains(&seconds) {
                bail!("{}", i18n::user_message(language, "preview-out-of-range"));
            }
            lock::run(LockMode::Preview(Duration::from_secs(seconds)))
        }
        Some("status") => control("status"),
        Some("pause") => control("pause"),
        Some("resume") => control("resume"),
        Some("reload") => control("reload"),
        Some("-h") | Some("--help") | Some("help") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            let language = config::interface_language();
            bail!(
                "{}\n\n{}",
                i18n::user_message(language, &format!("unknown:{other}")),
                i18n::help_text(language)
            )
        }
        None => control("status"),
    }
}

fn control(cmd: &str) -> Result<()> {
    let paths = paths::Paths::resolve()?;
    let language = config::interface_language();
    let reply = ipc::request(&paths.socket, cmd, language)?;
    ipc::print_reply(&reply, language)
}

fn print_help() {
    print!("{}", i18n::help_text(config::interface_language()));
}
