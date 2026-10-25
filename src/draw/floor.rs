//! The mode 7 ground: the scanline table, its DMA, and the windows over it.

use agb::{
    display::{HEIGHT, WIDTH, WinIn, tiled::AffineBackgroundId},
    fixnum::{Num, rect, vec2},
    interrupt::{self, Interrupt},
};

use super::*;

pub(crate) fn mask_view(
    frame: &mut agb::display::GraphicsFrame,
    ground_id: AffineBackgroundId,
    sky_h: i32,
    cockpit: bool,
    night: bool,
    wash: bool,
) {
    if !cockpit {
        {
            let sky = frame.windows().win_in(WinIn::Win0).enable_objects();
            if haze_tile() != 0 {
                sky.enable_background(ground_id);
            }
            if night || wash {
                sky.enable_blending();
            }
            sky.set_pos(rect(vec2(0, 0), vec2(WIDTH, sky_h)));
        }
        let outside = frame
            .windows()
            .win_out()
            .enable_background(ground_id)
            .enable_objects();
        if night {
            outside.enable_blending();
        }
        return;
    }
    let view_bot = VIEW_Y + VIEW_H;
    let sky_bot = sky_h.clamp(VIEW_Y, view_bot);
    let sky_span = sky_bot - VIEW_Y;
    if sky_span > 0 {
        let sky = frame.windows().win_in(WinIn::Win1).enable_objects();
        if haze_tile() != 0 {
            sky.enable_background(ground_id);
        }
        if night || wash {
            sky.enable_blending();
        }
        sky.set_pos(rect(vec2(VIEW_X, VIEW_Y), vec2(VIEW_W, sky_span)));
    }
    let ground_span = view_bot - sky_bot;
    if ground_span > 0 {
        let ground = frame
            .windows()
            .win_in(WinIn::Win0)
            .enable_objects()
            .enable_background(ground_id);
        if night || wash {
            ground.enable_blending();
        }
        ground.set_pos(rect(vec2(VIEW_X, sky_bot), vec2(VIEW_W, ground_span)));
    }
    let outside = frame.windows().win_out().enable_objects();
    if night {
        outside.enable_blending();
    }
}

pub(crate) fn clear_ground_map() {
    unsafe {
        // Whole halfwords: a byte store to background VRAM is written twice.
        let map = 0x0600_C000 as *mut u16;
        let pair = u16::from(T_FLOOR + 1) * 0x0101;
        for cell in 0..(128 * 128 / 2) {
            map.add(cell).write_volatile(pair);
        }
    }
}

/// BG2 control: priority 1, screenblock 24, wrap, size 128×128.
pub(crate) fn arm_ground() {
    unsafe {
        (0x0400_000C as *mut u16).write_volatile(0xF801);
    }
}

/// Drop the ground layer at once. Map writes during a scanline tear the
/// picture, so a load hides the floor before it touches the table.
pub(crate) fn hide_ground() {
    unsafe {
        let disp = 0x0400_0000 as *mut u16;
        let cur = disp.read_volatile();
        disp.write_volatile(cur & !(1 << 10));
    }
}

/// The displayed map is 128 tiles, 1024 pixels. The hardware wraps that, but a
/// negative reference does not wrap: it shows the sky colour as one scanline.
/// Two extra rings keep every visible line positive without moving the tile.
pub(crate) fn ring_space(sub: i32) -> i32 {
    const RING: i32 = VIEW * 8 * 256;
    sub.rem_euclid(RING) + RING * 2
}

pub(crate) struct Nadir(core::cell::UnsafeCell<i32>);
unsafe impl Sync for Nadir {}
pub(crate) static NADIR: Nadir = Nadir(core::cell::UnsafeCell::new(0));

pub(crate) fn set_nadir(scale: i32) {
    unsafe { *NADIR.0.get() = scale }
}

pub(crate) fn nadir_on() -> i32 {
    unsafe { *NADIR.0.get() }
}

/// World pixels per screen pixel while the glass looks straight down.
pub(crate) fn nadir_scale(alt: i32) -> i32 {
    ((alt >> 8).max(8) / 8).clamp(1, 4)
}

/// The affine step fits in a signed byte-pair. Past ±100 the road bends.
#[inline(always)]
fn clamp_step(value: i32) -> i16 {
    value.clamp(-100 << 8, 100 << 8) as i16
}

/// `2^32 / n`, rounded down, for the distances of the floor's lines.
const RECIPROCAL: [u32; 512] = {
    let mut table = [0u32; 512];
    let mut n = 2;
    while n < 512 {
        table[n] = ((1u64 << 32) / n as u64) as u32;
        n += 1;
    }
    table
};

/// `n / d` for `n >= 0` and `d >= 1`, the same as `/`. The reciprocal is at
/// most one short, so one step up makes it exact.
#[inline(always)]
pub(crate) fn line_div(n: i32, d: i32) -> i32 {
    if d == 1 {
        return n;
    }
    if d >= RECIPROCAL.len() as i32 {
        return n / d;
    }
    let (n, d) = (n as u32, d as u32);
    let mut quot = ((u64::from(n) * u64::from(RECIPROCAL[d as usize])) >> 32) as u32;
    if (quot + 1) * d <= n {
        quot += 1;
    }
    quot as i32
}

/// One ground line, in register units. Integer multiply, not the fixed-point
/// helper: that helper splits every product into four multiplies, and this
/// runs once per scanline.
#[inline(always)]
fn perspective_line(
    origin_x: i32,
    origin_y: i32,
    alt: i32,
    sin: i32,
    cos: i32,
    scanline: i32,
    horizon: i32,
) -> FloorLine {
    let rel = (scanline - horizon).max(1);
    let mut lambda = line_div(alt.saturating_mul(FLOOR_ZOOM).max(0), rel);
    if lambda > 8 << 8 {
        lambda = 8 << 8;
    }
    // One perspective for the whole line. A straight road then stays a
    // straight line on screen. Changing the distance without the scale bends it.
    let along = lambda.saturating_mul(FOCAL);
    let lcf = (lambda * cos) >> 8;
    let lsf = (lambda * sin) >> 8;
    let half = WIDTH / 2;
    FloorLine {
        a: clamp_step(lcf),
        b: 0,
        c: clamp_step(lsf),
        d: 0,
        x: origin_x - lcf * half + ((sin * along) >> 8),
        y: origin_y - lsf * half - ((cos * along) >> 8),
    }
}

/// One scanline of the mode 7 floor, in register units (8.8 fixed point).
/// Layout matches BG2PA..BG2Y so a DMA can copy a line in one burst.
#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct FloorLine {
    pub(crate) a: i16,
    pub(crate) b: i16,
    pub(crate) c: i16,
    pub(crate) d: i16,
    pub(crate) x: i32,
    pub(crate) y: i32,
}

pub(crate) const FLAT_LINE: FloorLine = FloorLine {
    a: 0x100,
    b: 0,
    c: 0,
    d: 0,
    x: 0,
    y: 0,
};

pub(crate) struct FloorBuf {
    pub(crate) front: core::cell::UnsafeCell<usize>,
    pub(crate) lines: core::cell::UnsafeCell<[[FloorLine; 162]; 2]>,
}

unsafe impl Sync for FloorBuf {}

#[unsafe(link_section = ".iwram")]
pub(crate) static FLOOR: FloorBuf = FloorBuf {
    front: core::cell::UnsafeCell::new(0),
    lines: core::cell::UnsafeCell::new([[FLAT_LINE; 162]; 2]),
};

pub(crate) fn write_floor_line(line: &FloorLine) {
    unsafe {
        (0x0400_0020 as *mut u16).write_volatile(line.a as u16);
        (0x0400_0022 as *mut u16).write_volatile(line.b as u16);
        (0x0400_0024 as *mut u16).write_volatile(line.c as u16);
        (0x0400_0026 as *mut u16).write_volatile(line.d as u16);
        (0x0400_0028 as *mut i32).write_volatile(line.x);
        (0x0400_002C as *mut i32).write_volatile(line.y);
    }
}

/// Four words, destination reloads, source walks forward, repeat on HBlank.
pub(crate) const FLOOR_DMA: u32 =
    4 | (0b11 << 21) | (1 << 25) | (1 << 26) | (0b10 << 28) | (1 << 31);

/// The sound mixer takes an interrupt every frame. An HBlank interrupt then
/// misses lines and the ground sticks. DMA0 copies the line in hardware.
pub(crate) fn install_floor() {
    let stop = unsafe {
        interrupt::add_interrupt_handler(Interrupt::VBlank, |_| {
            // Before the first line. A palette written on a live scanline
            // stripes the whole floor.
            flush_look();
            (0x0400_00BA as *mut u16).write_volatile(0);
        })
    };
    let start = unsafe {
        interrupt::add_interrupt_handler(Interrupt::VCounter, |_| {
            arm_floor_dma();
        })
    };
    core::mem::forget(stop);
    core::mem::forget(start);
    unsafe {
        let stat = 0x0400_0004 as *mut u16;
        let cur = stat.read_volatile();
        stat.write_volatile((cur & 0x00FF) | (1 << 5));
    }
}

pub(crate) fn arm_floor_dma() {
    unsafe {
        let front = *FLOOR.front.get();
        let lines = &(*FLOOR.lines.get())[front];
        write_floor_line(&lines[0]);
        (0x0400_00B0 as *mut u32).write_volatile(lines.as_ptr().add(1) as u32);
        (0x0400_00B4 as *mut u32).write_volatile(0x0400_0020);
        (0x0400_00B8 as *mut u32).write_volatile(FLOOR_DMA);
    }
}

pub(crate) fn publish_floor() {
    unsafe {
        let front = FLOOR.front.get();
        *front ^= 1;
        write_floor_line(&(*FLOOR.lines.get())[*front][0]);
    }
}

/// The hardware slot of the haze ramp. agb drops duplicate tiles, so its
/// slot is not simply its place in the sheet. 0 until the sheet is parked.
pub(crate) struct HazeTile(core::cell::UnsafeCell<u8>);

unsafe impl Sync for HazeTile {}

static HAZE_TILE: HazeTile = HazeTile(core::cell::UnsafeCell::new(0));

pub(crate) fn haze_tile() -> u8 {
    unsafe { *HAZE_TILE.0.get() }
}

/// Find the ramp among the parked tiles by its shape: sixteen different
/// texels, then the last of them repeated. Then have the glass paint it.
pub(crate) fn find_haze_tile() {
    let found = (1..=64u8).find(|&slot| {
        let texel = |index: usize| unsafe {
            ((0x0600_0000 + usize::from(slot) * 64 + index) as *const u8).read_volatile()
        };
        let mut seen = 0u32;
        for index in 0..16 {
            seen |= 1 << (texel(index) & 31);
        }
        seen.count_ones() == 16 && (16..64).all(|index| texel(index) == texel(15))
    });
    unsafe { *HAZE_TILE.0.get() = found.unwrap_or(0) };
    repaint_haze();
}

/// Lines from the edge of the ground to the top of the ramp.
const HAZE_SPAN: i32 = 96;

/// A sky line: no step across, so the whole line is one texel of the ramp.
/// The bottom of the sky is the haze, the zenith is texel 0.
#[inline(always)]
fn haze_line(haze: (i32, i32), sky_h: i32, scanline: i32) -> FloorLine {
    let up = sky_h - 1 - scanline;
    // In quarter steps. Between two texels the lines take turns, a quarter,
    // a half or three quarters of them a step higher, so the ramp has no bands.
    let quarters = (up * 15 * 4 / HAZE_SPAN).min(60);
    let mut step = 15 - quarters / 4;
    if quarters % 4 > [0, 2, 1, 3][(scanline & 3) as usize] {
        step -= 1;
    }
    let (hx, hy) = haze;
    FloorLine {
        a: 0,
        b: 0,
        c: 0,
        d: 0,
        x: ((hx.rem_euclid(VIEW) * 8 + step % 8) << 8) + 128,
        y: ((hy.rem_euclid(VIEW) * 8 + step / 8) << 8) + 128,
    }
}

/// ARM code in IWRAM: 160 lines of multiplies and a division each, every frame.
#[unsafe(link_section = ".iwram")]
#[instruction_set(arm::a32)]
pub(crate) fn build_floor(
    cam_x: i32,
    cam_y: i32,
    alt: i32,
    yaw: Num<i32, 8>,
    horizon: i32,
    haze: (i32, i32),
) {
    let cos = yaw.cos().to_raw();
    let sin = yaw.sin().to_raw();
    let down = nadir_on();
    unsafe {
        let back = *FLOOR.front.get() ^ 1;
        let buf = &mut (*FLOOR.lines.get())[back];
        if down > 0 {
            // Flat glass. Scanline 88, pixel 120 is the ground under the airframe.
            // The step is the same on every line. Only the origin walks.
            let step_x = (cos * down).clamp(-32767, 32767) as i16;
            let step_y = (sin * down).clamp(-32767, 32767) as i16;
            let right = (-120 * down) << 8;
            let cos_right = (right * cos) >> 8;
            let sin_right = (right * sin) >> 8;
            for scanline in 0..HEIGHT {
                let ahead = ((88 - scanline) * down) << 8;
                buf[scanline as usize] = FloorLine {
                    a: step_x,
                    b: 0,
                    c: step_y,
                    d: 0,
                    x: cam_x + cos_right + ((ahead * sin) >> 8),
                    y: cam_y + sin_right - ((ahead * cos) >> 8),
                };
            }
        } else if haze_tile() != 0 {
            // The sky window shows these lines down to its bottom edge, which
            // also hides the far floor.
            let sky_h = sky_bottom(horizon, alt).min(HEIGHT);
            for scanline in 0..sky_h {
                buf[scanline as usize] = haze_line(haze, sky_h, scanline);
            }
            for scanline in sky_h..HEIGHT {
                buf[scanline as usize] =
                    perspective_line(cam_x, cam_y, alt, sin, cos, scanline, horizon);
            }
        } else {
            // Above the horizon every line is the same vanishing point.
            let start = horizon.clamp(0, HEIGHT - 1);
            let sky = perspective_line(cam_x, cam_y, alt, sin, cos, start, horizon);
            for scanline in 0..=start {
                buf[scanline as usize] = sky;
            }
            for scanline in (start + 1)..HEIGHT {
                buf[scanline as usize] =
                    perspective_line(cam_x, cam_y, alt, sin, cos, scanline, horizon);
            }
        }
        buf[160] = buf[159];
        buf[161] = buf[159];
    }
}
