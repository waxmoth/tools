use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context, Result};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag;

use crate::config::Config;
use crate::ipc::{self, Shared};
use crate::paths::Paths;
use crate::session::{Effect, Phase, Session};
use crate::x11::X11;

pub fn run() -> Result<()> {
    let paths = Paths::resolve()?;
    let shutdown = Arc::new(AtomicBool::new(false));
    flag::register(SIGTERM, shutdown.clone())?;
    flag::register(SIGINT, shutdown.clone())?;

    let config = wait_for_config(&paths, &shutdown)?;
    if shutdown.load(Ordering::SeqCst) {
        return Ok(());
    }
    let listener = ipc::bind_socket(&paths.socket)?;
    let session = match Session::load(&paths.state, &config, Instant::now()) {
        Ok(session) => session,
        Err(err) => {
            eprintln!("sitbreak: ignoring damaged timer state: {err:#}");
            Session::new(&config)
        }
    };
    let shared = Arc::new(Mutex::new(Shared {
        config,
        session,
        idle: Duration::ZERO,
    }));
    {
        let shared = shared.clone();
        let config_path = paths.config.clone();
        thread::spawn(move || ipc::serve(listener, shared, config_path));
    }

    eprintln!("sitbreak: started, config {}", paths.config.display());
    let mut display: Option<X11> = None;
    let mut lock: Option<Child> = None;
    let mut last_tick = Instant::now();
    let mut last_persist = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);
    let mut next_wake = Instant::now();
    let mut config_mtime = mtime(&paths.config);

    while !shutdown.load(Ordering::SeqCst) {
        next_wake += Duration::from_secs(1);
        let now = Instant::now();
        let delta = now.saturating_duration_since(last_tick);
        last_tick = now;

        if mtime(&paths.config) != config_mtime {
            reload_config(&shared, &paths);
            config_mtime = mtime(&paths.config);
        }

        let idle = match idle_of(&mut display) {
            Some(idle) => idle,
            None => {
                sleep_until(next_wake);
                continue;
            }
        };

        let effect = {
            let mut guard = shared.lock().unwrap_or_else(|err| err.into_inner());
            guard.idle = idle;
            guard.session.tick(delta, idle, now)
        };
        match effect {
            Effect::StartedBreak => eprintln!("sitbreak: break started"),
            Effect::EndedBreak => {
                eprintln!("sitbreak: break ended");
                stop_lock(&mut lock);
            }
            Effect::None => {}
        }

        let phase = shared
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .session
            .phase;
        if phase == Phase::Breaking {
            ensure_lock(&mut lock);
        }

        if effect != Effect::None || last_persist.elapsed() >= Duration::from_secs(5) {
            persist(&shared, &paths);
            last_persist = Instant::now();
        }
        sleep_until(next_wake);
    }

    stop_lock(&mut lock);
    persist(&shared, &paths);
    let _ = std::fs::remove_file(&paths.socket);
    eprintln!("sitbreak: stopped");
    Ok(())
}

fn wait_for_config(paths: &Paths, shutdown: &AtomicBool) -> Result<Config> {
    loop {
        if shutdown.load(Ordering::SeqCst) {
            anyhow::bail!("startup interrupted");
        }
        match Config::load_or_create(&paths.config) {
            Ok(config) => return Ok(config),
            Err(err) => {
                eprintln!("sitbreak: {err:#}");
                thread::sleep(Duration::from_secs(5));
            }
        }
    }
}

fn reload_config(shared: &Mutex<Shared>, paths: &Paths) {
    match Config::load_or_create(&paths.config) {
        Ok(config) => {
            let mut guard = shared.lock().unwrap_or_else(|err| err.into_inner());
            guard.session.apply_config(&config);
            guard.config = config;
            eprintln!("sitbreak: config reloaded");
        }
        Err(err) => eprintln!("sitbreak: config was not applied: {err:#}"),
    }
}

fn idle_of(display: &mut Option<X11>) -> Option<Duration> {
    if display.is_none() {
        match X11::connect() {
            Ok(conn) => {
                eprintln!(
                    "sitbreak: connected to {}",
                    std::env::var("DISPLAY").unwrap_or_else(|_| "X11".to_string())
                );
                *display = Some(conn);
            }
            Err(err) => {
                eprintln!("sitbreak: waiting for X11: {err:#}");
                return None;
            }
        }
    }
    match display.as_ref().unwrap().idle() {
        Ok(idle) => Some(idle),
        Err(err) => {
            eprintln!("sitbreak: failed to read idle time: {err:#}");
            *display = None;
            None
        }
    }
}

fn persist(shared: &Mutex<Shared>, paths: &Paths) {
    let guard = shared.lock().unwrap_or_else(|err| err.into_inner());
    if let Err(err) = guard.session.store(&paths.state, Instant::now()) {
        eprintln!("sitbreak: failed to save state: {err:#}");
    }
}

fn ensure_lock(lock: &mut Option<Child>) {
    if let Some(child) = lock.as_mut() {
        match child.try_wait() {
            Ok(Some(status)) => {
                eprintln!("sitbreak: lock process exited ({status}), reopening");
                *lock = None;
            }
            Ok(None) => return,
            Err(err) => {
                eprintln!("sitbreak: failed to check lock process: {err}");
                *lock = None;
            }
        }
    }
    match spawn_lock() {
        Ok(child) => *lock = Some(child),
        Err(err) => eprintln!("sitbreak: failed to open the break screen: {err:#}"),
    }
}

fn spawn_lock() -> Result<Child> {
    let exe = std::env::current_exe().context("failed to locate the sitbreak executable")?;
    let mut cmd = Command::new(exe);
    cmd.arg("lock");
    unsafe {
        cmd.pre_exec(|| {
            libc::prctl(
                libc::PR_SET_PDEATHSIG,
                libc::SIGTERM as libc::c_ulong,
                0,
                0,
                0,
            );
            if libc::getppid() == 1 {
                libc::_exit(1);
            }
            Ok(())
        });
    }
    Ok(cmd.spawn()?)
}

fn stop_lock(lock: &mut Option<Child>) {
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn mtime(path: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
}

fn sleep_until(deadline: Instant) {
    let now = Instant::now();
    if deadline > now {
        thread::sleep(deadline - now);
    }
}
