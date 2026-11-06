use std::time::Duration;

use anyhow::{bail, Context, Result};
use x11rb::connection::{Connection, RequestConnection};
use x11rb::protocol::randr;
use x11rb::protocol::screensaver;
use x11rb::protocol::xproto::{ImageOrder, Screen, Setup};
use x11rb::rust_connection::RustConnection;

pub struct Monitor {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

pub struct X11 {
    pub conn: RustConnection,
    pub root: u32,
    pub width: u16,
    pub height: u16,
    pub depth: u8,
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    lsb: bool,
    screensaver: bool,
}

impl X11 {
    pub fn connect() -> Result<Self> {
        let (conn, screen_num) =
            RustConnection::connect(None).context("failed to connect to X11")?;
        let setup = conn.setup();
        let screen = setup
            .roots
            .get(screen_num)
            .context("X11 screen not found")?;
        let (red, green, blue) = visual_masks(setup, screen)?;
        let bpp = pixmap_bpp(setup, screen.root_depth)?;
        if bpp != 32 {
            bail!("X11 pixel format is {bpp} bpp; sitbreak needs 32 bpp");
        }
        let screensaver = conn
            .extension_information(screensaver::X11_EXTENSION_NAME)?
            .is_some();
        if screensaver {
            screensaver::query_version(&conn, 1, 1)?.reply()?;
        } else {
            bail!("X server has no MIT-SCREEN-SAVER extension, so idle time is unavailable");
        }
        Ok(Self {
            root: screen.root,
            width: screen.width_in_pixels,
            height: screen.height_in_pixels,
            depth: screen.root_depth,
            red_mask: red,
            green_mask: green,
            blue_mask: blue,
            lsb: setup.image_byte_order == ImageOrder::LSB_FIRST,
            screensaver,
            conn,
        })
    }

    pub fn idle(&self) -> Result<Duration> {
        if !self.screensaver {
            bail!("screensaver extension is missing");
        }
        let info = screensaver::query_info(&self.conn, self.root)?.reply()?;
        Ok(Duration::from_millis(u64::from(info.ms_since_user_input)))
    }

    pub fn monitors(&self) -> Result<Vec<Monitor>> {
        let fallback = vec![Monitor {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }];
        if self
            .conn
            .extension_information(randr::X11_EXTENSION_NAME)?
            .is_none()
        {
            return Ok(fallback);
        }
        randr::query_version(&self.conn, 1, 5)?.reply()?;
        let resources = randr::get_screen_resources(&self.conn, self.root)?.reply()?;
        let mut monitors = Vec::new();
        for crtc in resources.crtcs {
            let info = randr::get_crtc_info(&self.conn, crtc, 0)?.reply()?;
            if info.mode == 0 || info.width == 0 || info.height == 0 {
                continue;
            }
            monitors.push(Monitor {
                x: info.x,
                y: info.y,
                width: info.width,
                height: info.height,
            });
        }
        if monitors.is_empty() {
            Ok(fallback)
        } else {
            Ok(monitors)
        }
    }

    pub fn pixel(&self, r: u8, g: u8, b: u8) -> u32 {
        pack_channel(r, self.red_mask)
            | pack_channel(g, self.green_mask)
            | pack_channel(b, self.blue_mask)
    }

    pub fn encode_rgb(&self, r: u8, g: u8, b: u8) -> [u8; 4] {
        let pixel = self.pixel(r, g, b);
        if self.lsb {
            pixel.to_le_bytes()
        } else {
            pixel.to_be_bytes()
        }
    }
}

fn pack_channel(value: u8, mask: u32) -> u32 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let bits = mask.count_ones();
    let max = (1u32 << bits) - 1;
    let scaled = (u32::from(value) * max + 127) / 255;
    scaled << shift
}

fn pixmap_bpp(setup: &Setup, depth: u8) -> Result<u8> {
    setup
        .pixmap_formats
        .iter()
        .find(|format| format.depth == depth)
        .map(|format| format.bits_per_pixel)
        .context("no pixmap format matches the root window depth")
}

fn visual_masks(setup: &Setup, screen: &Screen) -> Result<(u32, u32, u32)> {
    let _ = setup;
    for depth in &screen.allowed_depths {
        if depth.depth != screen.root_depth {
            continue;
        }
        for visual in &depth.visuals {
            if visual.visual_id == screen.root_visual {
                return Ok((visual.red_mask, visual.green_mask, visual.blue_mask));
            }
        }
    }
    bail!("root window visual not found")
}

pub fn rgb_buffer_to_server(display: &X11, rgb: &[u8], width: u16, height: u16) -> Vec<u8> {
    let pixels = width as usize * height as usize;
    let mut out = vec![0u8; pixels * 4];
    for (index, chunk) in rgb.chunks_exact(3).take(pixels).enumerate() {
        let pixel = display.encode_rgb(chunk[0], chunk[1], chunk[2]);
        out[index * 4..index * 4 + 4].copy_from_slice(&pixel);
    }
    out
}
