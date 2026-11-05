use std::fs;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use fontdue::Font;
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::dpms::{self, DPMSMode};
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::CURRENT_TIME;

use crate::ipc;
use crate::paths::Paths;
use crate::x11::{rgb_buffer_to_server, Monitor, X11};

const BG: [u8; 3] = [18, 20, 28];
const TITLE: [u8; 3] = [242, 244, 248];
const TIMER: [u8; 3] = [126, 182, 255];
const HINT: [u8; 3] = [168, 176, 192];
const BAR_BG: [u8; 3] = [42, 48, 64];
const XK_ESCAPE: u32 = 0xff1b;

pub enum LockMode {
    FollowDaemon,
    Preview(Duration),
}

pub fn run(mode: LockMode) -> Result<()> {
    match mode {
        LockMode::Preview(duration) => preview(duration),
        LockMode::FollowDaemon => follow_daemon(),
    }
}

fn follow_daemon() -> Result<()> {
    let paths = Paths::resolve()?;
    let started = Instant::now();
    loop {
        if let Ok(reply) = ipc::request(&paths.socket, "status", crate::i18n::Language::En) {
            if reply.phase == "breaking" {
                return show(Some(paths), Duration::from_millis(reply.break_remaining_ms));
            }
        }
        if started.elapsed() > Duration::from_secs(3) {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn preview(duration: Duration) -> Result<()> {
    let total = duration.max(Duration::from_secs(1));
    run_window(true, total, move |_now| Poll::Preview(total))
}

fn show(paths: Option<Paths>, initial_remaining: Duration) -> Result<()> {
    let paths = paths.context("lock screen is missing its daemon connection")?;
    let mut total = initial_remaining.max(Duration::from_secs(1));
    let mut fallback = Instant::now() + initial_remaining;
    let mut missed = 0u32;
    run_window(false, total, move |_now| {
        match ipc::request(&paths.socket, "status", crate::i18n::Language::En) {
            Ok(reply) if reply.phase == "breaking" => {
                missed = 0;
                let remaining = Duration::from_millis(reply.break_remaining_ms);
                total = Duration::from_secs(reply.break_limit_seconds.max(1));
                fallback = Instant::now() + remaining;
                Poll::Break { remaining, total }
            }
            Ok(_) => Poll::Stop,
            Err(_) => {
                missed += 1;
                if missed > 25 {
                    Poll::LostDaemon
                } else {
                    let remaining = fallback.saturating_duration_since(Instant::now());
                    Poll::Break { remaining, total }
                }
            }
        }
    })
}

enum Poll {
    Preview(Duration),
    Break {
        remaining: Duration,
        total: Duration,
    },
    Stop,
    LostDaemon,
}

fn run_window(
    preview: bool,
    initial_total: Duration,
    mut poll: impl FnMut(Instant) -> Poll,
) -> Result<()> {
    let display = X11::connect()?;
    let monitors = display.monitors()?;
    let window = create_lock_window(&display)?;
    let cursor = create_blank_cursor(&display).ok();
    let _ = try_grab(&display, window, cursor);
    let _ = wake_screen(&display);
    let fonts = load_font()?;
    let language = crate::config::interface_language();
    let escape = if preview {
        Some(escape_keycodes(&display)?)
    } else {
        None
    };
    let mut next_grab = Instant::now();
    let mut last_wake = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);
    let mut last_shown = u64::MAX;
    let mut total = initial_total;
    let started = Instant::now();

    loop {
        let now = Instant::now();
        let remaining = match poll(now) {
            Poll::Stop => return Ok(()),
            Poll::LostDaemon => bail!("daemon is not responding, closing the break screen"),
            Poll::Preview(duration) => {
                let elapsed = now.saturating_duration_since(started);
                if elapsed >= duration {
                    return Ok(());
                }
                duration.saturating_sub(elapsed)
            }
            Poll::Break {
                remaining,
                total: next_total,
            } => {
                total = next_total;
                remaining
            }
        };
        if last_wake.elapsed() > Duration::from_secs(20) {
            let _ = wake_screen(&display);
            last_wake = Instant::now();
        }
        if now >= next_grab {
            let _ = try_grab(&display, window, cursor);
            next_grab = now + Duration::from_secs(2);
        }
        let _ = raise_window(&display, window);

        while let Some(event) = display.conn.poll_for_event()? {
            match event {
                Event::KeyPress(key) if preview => {
                    if escape
                        .as_ref()
                        .is_some_and(|codes| codes.contains(&key.detail))
                    {
                        return Ok(());
                    }
                }
                Event::Error(err) => eprintln!("sitbreak: X11 error {err:?}"),
                _ => {}
            }
        }

        let shown = remaining.as_secs();
        if shown != last_shown {
            paint(
                &display, window, &monitors, &fonts, remaining, total, preview, language,
            )?;
            last_shown = shown;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

fn create_lock_window(display: &X11) -> Result<Window> {
    let window = display.conn.generate_id()?;
    display
        .conn
        .create_window(
            display.depth,
            window,
            display.root,
            0,
            0,
            display.width,
            display.height,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new()
                .override_redirect(1)
                .background_pixel(display.pixel(BG[0], BG[1], BG[2]))
                .event_mask(
                    EventMask::EXPOSURE
                        | EventMask::KEY_PRESS
                        | EventMask::KEY_RELEASE
                        | EventMask::BUTTON_PRESS
                        | EventMask::BUTTON_RELEASE
                        | EventMask::POINTER_MOTION,
                ),
        )?
        .check()?;
    display
        .conn
        .change_property8(
            PropMode::REPLACE,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            b"sitbreak",
        )?
        .check()?;
    display.conn.map_window(window)?.check()?;
    raise_window(display, window)?;
    Ok(window)
}

fn raise_window(display: &X11, window: Window) -> Result<()> {
    display.conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )?;
    display.conn.flush()?;
    Ok(())
}

fn try_grab(display: &X11, window: Window, cursor: Option<Cursor>) -> Result<()> {
    let keyboard = display
        .conn
        .grab_keyboard(
            false,
            window,
            CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )?
        .reply()?;
    if keyboard.status != GrabStatus::SUCCESS {
        eprintln!("sitbreak: keyboard grab status {:?}", keyboard.status);
    }
    let pointer = display
        .conn
        .grab_pointer(
            false,
            window,
            EventMask::BUTTON_PRESS | EventMask::BUTTON_RELEASE | EventMask::POINTER_MOTION,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            window,
            cursor.unwrap_or(0),
            CURRENT_TIME,
        )?
        .reply()?;
    if pointer.status != GrabStatus::SUCCESS {
        eprintln!("sitbreak: pointer grab status {:?}", pointer.status);
    }
    Ok(())
}

fn create_blank_cursor(display: &X11) -> Result<Cursor> {
    let pixmap = display.conn.generate_id()?;
    display
        .conn
        .create_pixmap(1, pixmap, display.root, 1, 1)?
        .check()?;
    let cursor = display.conn.generate_id()?;
    display
        .conn
        .create_cursor(cursor, pixmap, pixmap, 0, 0, 0, 0, 0, 0, 0, 0)?
        .check()?;
    display.conn.free_pixmap(pixmap)?.check()?;
    Ok(cursor)
}

fn wake_screen(display: &X11) -> Result<()> {
    if display
        .conn
        .extension_information(dpms::X11_EXTENSION_NAME)?
        .is_none()
    {
        return Ok(());
    }
    dpms::get_version(&display.conn, 1, 1)?.reply()?;
    dpms::force_level(&display.conn, DPMSMode::ON)?.check()?;
    Ok(())
}

fn escape_keycodes(display: &X11) -> Result<Vec<u8>> {
    let setup = display.conn.setup();
    let min = setup.min_keycode;
    let count = setup.max_keycode - min + 1;
    let mapping = display.conn.get_keyboard_mapping(min, count)?.reply()?;
    let per = mapping.keysyms_per_keycode as usize;
    let mut codes = Vec::new();
    if per == 0 {
        return Ok(codes);
    }
    for (index, chunk) in mapping.keysyms.chunks(per).enumerate() {
        if chunk.contains(&XK_ESCAPE) {
            codes.push(min + index as u8);
        }
    }
    Ok(codes)
}

struct Rgb {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Rgb {
    fn new(width: u16, height: u16) -> Self {
        let pixels = width as usize * height as usize;
        Self {
            width: u32::from(width),
            height: u32::from(height),
            data: vec![0; pixels * 3],
        }
    }

    fn fill(&mut self, color: [u8; 3]) {
        for pixel in self.data.chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
    }

    fn blend(&mut self, x: i32, y: i32, color: [u8; 3], coverage: u8) {
        if x < 0 || y < 0 || x as u32 >= self.width || y as u32 >= self.height || coverage == 0 {
            return;
        }
        let index = ((y as u32 * self.width + x as u32) * 3) as usize;
        let dest = &mut self.data[index..index + 3];
        for channel in 0..3 {
            let src = u32::from(color[channel]);
            let old = u32::from(dest[channel]);
            let alpha = u32::from(coverage);
            dest[channel] = ((src * alpha + old * (255 - alpha)) / 255) as u8;
        }
    }

    fn rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 3]) {
        for row in y..y + height {
            for col in x..x + width {
                self.blend(col, row, color, 255);
            }
        }
    }
}

struct Fonts {
    cjk: Font,
    latin: Font,
}

struct Glyph {
    metrics: fontdue::Metrics,
    bitmap: Vec<u8>,
    pen_x: f32,
}

fn layout(fonts: &Fonts, text: &str, size: f32) -> (Vec<Glyph>, f32) {
    let mut pen = 0.0f32;
    let mut glyphs = Vec::new();
    for ch in text.chars() {
        let font = if ch.is_ascii() {
            &fonts.latin
        } else {
            &fonts.cjk
        };
        let (metrics, bitmap) = font.rasterize(ch, size);
        glyphs.push(Glyph {
            metrics,
            bitmap,
            pen_x: pen,
        });
        pen += metrics.advance_width;
    }
    (glyphs, pen)
}

fn draw_text(image: &mut Rgb, glyphs: &[Glyph], left: f32, baseline: i32, color: [u8; 3]) {
    for glyph in glyphs {
        let origin_x = left + glyph.pen_x;
        let top = baseline - glyph.metrics.height as i32 - glyph.metrics.ymin;
        let left_px = origin_x as i32 + glyph.metrics.xmin;
        let width = glyph.metrics.width;
        for row in 0..glyph.metrics.height {
            for col in 0..width {
                let coverage = glyph.bitmap[row * width + col];
                image.blend(left_px + col as i32, top + row as i32, color, coverage);
            }
        }
    }
}

fn paint(
    display: &X11,
    window: Window,
    monitors: &[Monitor],
    fonts: &Fonts,
    remaining: Duration,
    total: Duration,
    preview: bool,
    language: crate::i18n::Language,
) -> Result<()> {
    let gc = display.conn.generate_id()?;
    display
        .conn
        .create_gc(gc, window, &CreateGCAux::new())?
        .check()?;
    for monitor in monitors {
        paint_monitor(
            display, window, gc, monitor, fonts, remaining, total, preview, language,
        )?;
    }
    display.conn.free_gc(gc)?;
    display.conn.flush()?;
    Ok(())
}

fn paint_monitor(
    display: &X11,
    window: Window,
    gc: Gcontext,
    monitor: &Monitor,
    fonts: &Fonts,
    remaining: Duration,
    total: Duration,
    preview: bool,
    language: crate::i18n::Language,
) -> Result<()> {
    let panel_w = monitor.width.min(1280).max(320);
    let panel_h = monitor.height.min(560).max(240);
    let dst_x = monitor.x + ((monitor.width - panel_w) / 2) as i16;
    let dst_y = monitor.y + ((monitor.height - panel_h) / 2) as i16;
    let mut image = Rgb::new(panel_w, panel_h);
    image.fill(BG);

    let copy = crate::i18n::lock_copy(language, preview);
    let title = copy.title;
    let hint = copy.hint;
    let timer = format_clock(remaining);
    let (title_glyphs, title_w) = layout(fonts, title, 72.0);
    let (timer_glyphs, timer_w) = layout(fonts, &timer, 128.0);
    let (hint_glyphs, hint_w) = layout(fonts, hint, 28.0);

    let center = panel_w as f32 / 2.0;
    let block_h = 72.0 + 48.0 + 128.0 + 56.0 + 10.0 + 48.0 + 28.0;
    let mut baseline = ((panel_h as f32 - block_h) / 2.0 + 62.0) as i32;
    draw_text(
        &mut image,
        &title_glyphs,
        center - title_w / 2.0,
        baseline,
        TITLE,
    );
    baseline += 72 + 56;
    draw_text(
        &mut image,
        &timer_glyphs,
        center - timer_w / 2.0,
        baseline,
        TIMER,
    );
    baseline += 48;
    let bar_w = (panel_w as i32 * 6) / 10;
    let bar_x = (panel_w as i32 - bar_w) / 2;
    let bar_y = baseline;
    image.rect(bar_x, bar_y, bar_w, 10, BAR_BG);
    let progress = if total.is_zero() {
        1.0
    } else {
        (1.0 - remaining.as_secs_f32() / total.as_secs_f32()).clamp(0.0, 1.0)
    };
    image.rect(bar_x, bar_y, ((bar_w as f32) * progress) as i32, 10, TIMER);
    baseline += 10 + 52;
    draw_text(
        &mut image,
        &hint_glyphs,
        center - hint_w / 2.0,
        baseline,
        HINT,
    );

    let server = rgb_buffer_to_server(display, &image.data, panel_w, panel_h);
    put_image(display, window, gc, panel_w, panel_h, dst_x, dst_y, &server)?;
    Ok(())
}

fn put_image(
    display: &X11,
    window: Window,
    gc: Gcontext,
    width: u16,
    height: u16,
    dst_x: i16,
    dst_y: i16,
    data: &[u8],
) -> Result<()> {
    let stride = width as usize * 4;
    let max_rows = (48 * 1024 / stride.max(1)).clamp(1, height as usize) as u16;
    let mut row = 0u16;
    while row < height {
        let rows = (height - row).min(max_rows);
        let start = row as usize * stride;
        let end = start + rows as usize * stride;
        display
            .conn
            .put_image(
                ImageFormat::Z_PIXMAP,
                window,
                gc,
                width,
                rows,
                dst_x,
                dst_y.saturating_add(row as i16),
                0,
                display.depth,
                &data[start..end],
            )?
            .check()?;
        row += rows;
    }
    Ok(())
}

fn format_clock(duration: Duration) -> String {
    let total = duration.as_secs();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn load_font() -> Result<Fonts> {
    let latin = read_font("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
        .or_else(|_| read_font("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"))?;
    let cjk = read_font("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf")
        .or_else(|_| read_font("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"))?;
    Ok(Fonts { cjk, latin })
}

fn read_font(path: &str) -> Result<Font> {
    let bytes = fs::read(path).with_context(|| format!("failed to read font {path}"))?;
    Font::from_bytes(bytes, fontdue::FontSettings::default())
        .map_err(|err| anyhow::anyhow!("failed to parse font {path}: {err}"))
}
