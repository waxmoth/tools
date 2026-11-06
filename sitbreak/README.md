# sitbreak

Tracks real keyboard and mouse use on an X11 session. When the work limit is reached, it covers every screen and grabs input until the break timer ends.

This is a voluntary limit. Switching to a text console (Ctrl+Alt+F3) or stopping the user service still exits, so the machine cannot be locked out.

## Install

```bash
./install.sh
```

The script builds a release binary, installs it to `~/.local/bin/sitbreak`, and enables the systemd user service. The service starts with the graphical session and retries until X11 is available.

## Config

`~/.config/sitbreak/config.toml` is created on first start:

```toml
work_minutes = 60
break_minutes = 15
idle_pause_seconds = 60
language = "auto"
```

`language` is `auto`, `en`, or `zh`. `auto` follows `LANG`. The work timer pauses after `idle_pause_seconds` without input. Saving the file reloads it; `sitbreak reload` does the same.

To try the full cycle quickly, set seconds. When a seconds key is present it overrides the matching minutes key:

```toml
work_seconds = 30
break_seconds = 10
```

## Commands

| Command | Effect |
|---|---|
| `sitbreak status` | Used time, time until the break, and current idle time |
| `sitbreak pause` / `resume` | Pause or resume the work timer. A break that has already started cannot be paused |
| `sitbreak reload` | Reread the config file |
| `sitbreak preview 8` | Show the break screen for 8 seconds. Esc exits early |

## Uninstall

```bash
./uninstall.sh
```

Config and accumulated time are kept. Delete `~/.local/state/sitbreak/state.json` to start the timer from zero.

## Notes

- This version supports X11 only. On a Wayland session the daemon waits for X11.
- Idle time comes from the X server's MIT-SCREEN-SAVER extension. Watching video without touching the keyboard or mouse pauses the work timer.
- The break countdown uses a monotonic clock. Stopping the service and starting it again during the same boot continues an unfinished break. A reboot starts the work timer from zero.
- Chinese text uses `DroidSansFallbackFull`. Digits and English use `DejaVu Sans`. Both ship with an Ubuntu desktop install.
