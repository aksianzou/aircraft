use agb::display::Priority;
use agb::display::tiled::{AffineBackground, AffineBackgroundSize, AffineBackgroundWrapBehaviour};
use agb::interrupt::VBlank;

use super::*;

struct SharedFloor(core::cell::UnsafeCell<Option<AffineBackground>>);

unsafe impl Sync for SharedFloor {}

static FLOOR_LAYER: SharedFloor = SharedFloor(core::cell::UnsafeCell::new(None));

/// One floor layer for every test, as the game has one for its whole run.
/// agb frees a dropped layer's tiles and hands the slots out again in another
/// order, and the ground map writes tile `n` as hardware slot `n + 1`.
pub(crate) fn ground() -> &'static mut AffineBackground {
    let layer = unsafe { &mut *FLOOR_LAYER.0.get() };
    layer.get_or_insert_with(|| {
        AffineBackground::new(
            Priority::P1,
            AffineBackgroundSize::Background64x64,
            AffineBackgroundWrapBehaviour::Wrap,
        )
    })
}

/// A whole clock in the air still leaves the battery star. Hover spends less.
#[test_case]
fn flight_spends_more_than_hover(_gba: &mut agb::Gba) {
    let floor = ground();
    let budget = time_budget(0);
    let mut flying = World::preview(floor, 0);
    flying.hover = false;
    for frame in 0..budget {
        flying.age = frame;
        flying.wear();
    }
    let mut hovering = World::preview(floor, 0);
    hovering.hover = true;
    for frame in 0..budget {
        hovering.age = frame;
        hovering.wear();
    }
    assert!(
        flying.battery >= 30,
        "a full flight should still have the battery star, left {}",
        flying.battery
    );
    assert!(
        flying.battery < 80,
        "flight should spend the pack, left {}",
        flying.battery
    );
    assert!(
        hovering.battery > flying.battery,
        "hover left {}, flight left {}",
        hovering.battery,
        flying.battery
    );
}

/// The faster truck still has road left after the approach.
#[test_case]
fn convoy_has_not_reached_the_mark(_gba: &mut agb::Gba) {
    let floor = ground();
    let mut world = World::preview(floor, 4);
    let start = world.trucks[0].y;
    for _ in 0..300 {
        world.update_trucks();
    }
    let truck = &world.trucks[0];
    assert!(truck.alive);
    let travelled = start - truck.y;
    assert!(
        travelled > 10_000,
        "the truck should be driving, moved {travelled}"
    );
    assert!(
        tile_of(truck.y) > 4,
        "the mark is still ahead, tile {}",
        tile_of(truck.y)
    );
}

/// A charge on the hatch the sheets draw takes the tank at once, even a
/// couple of pixels off as a drop from cruise height scatters. The hatch is
/// back on the turret, and a still tank keeps its facing whatever the camera.
#[test_case]
fn hatch_is_where_it_is_drawn(_gba: &mut agb::Gba) {
    let floor = ground();
    for (dir, vertical) in [(0, false), (18, false), (-18, true)] {
        for (dx, dy) in [(0, 0), (2, 2), (-2, 2), (3, -1)] {
            let mut world = World::preview(floor, 9);
            world.tanks = [dead_tank(); MAX_TANKS];
            world.tanks[0] = Tank {
                x: tile_mid(24),
                y: tile_mid(60),
                dir,
                vertical,
                home_tx: 24,
                home_ty: 60,
                objective: true,
                hp: HP_TANK,
                cool: 255,
                alive: true,
            };
            let (hx, hy) = world.tanks[0].hatch();
            world.explode(floor, hx + (dx << 8), hy + (dy << 8), 10, 0, KIND_MINE);
            assert!(
                !world.tanks[0].alive,
                "dir {dir} vertical {vertical}: a mine {dx},{dy} off the hatch left hp {}",
                world.tanks[0].hp
            );
        }
    }
}

/// The house card: blue desk, the condensed title, the roof in the frame.
#[test_case]
fn house_briefing(gba: &mut agb::Gba) {
    reserve_glass();
    let vblank = VBlank::get();
    vblank.wait_for_vblank();
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    gfx.set_background_palette_colour_256(0, Rgb::new(24, 64, 128).to_rgb15());
    hide_ground();
    let mut frame = gfx.frame();
    draw_select(&mut frame, 3, 0, (true, false), 0, Sensor::Day, None);
    frame.commit();
    agb::test_runner::assert_image_output("tests/briefing_house.png");
}

/// Every card once in the campaign, and each stage card laid out where it
/// can be flown: the truck on a road, the roof and the marks on dry ground,
/// all of it inside the radio.
#[test_case]
fn stage_cards_hold_together(_gba: &mut agb::Gba) {
    let mut seen = 0u64;
    for card in ORDER {
        assert!(seen & (1 << card) == 0, "card {card} twice");
        seen |= 1 << card;
    }
    assert!(seen == (1u64 << LEVEL_COUNT) - 1, "cards missing {seen:#x}");
    assert!(ORDER[0] == 0 && chain(ORDER[ORDER.len() - 1]).is_none());
    for level in FIRST_CHAIN..LEVEL_COUNT {
        let card = chain(level).expect("a stage card");
        assert!(card.stages.len() >= 2, "card {level} has one stage");
        let (pad_x, pad_y) = card.pad;
        let near = |tx: i32, ty: i32| {
            let dx = (tx - pad_x).abs();
            let dy = (ty - pad_y).abs();
            assert!(
                dx.max(dy) + dx.min(dy) / 2 <= 80,
                "card {level}: ({tx}, {ty}) is past the radio"
            );
        };
        for stage in card.stages {
            for &(tx, ty) in stage.spots.iter().chain(stage.caches) {
                near(tx, ty);
                assert!(
                    terrain(level, tx, ty) != T_WATER,
                    "card {level}: ({tx}, {ty}) is in a pond"
                );
            }
            match stage.task {
                Task::Hq => {
                    let (hx, hy) = stage.at;
                    near(hx, hy);
                    for ty in hy..=hy + 1 {
                        for tx in hx..=hx + 2 {
                            let tile = terrain(level, tx, ty);
                            assert!(
                                tile != T_WATER && tile != T_ROAD,
                                "card {level}: roof ({tx}, {ty}) is {tile}"
                            );
                        }
                    }
                }
                Task::Convoy => {
                    let (sx, sy) = stage.spots[0];
                    let (lx, ly) = stage.at;
                    assert!(lx == sx && ly < sy, "card {level}: the line is not ahead");
                    for ty in ly..=sy {
                        assert!(
                            terrain(level, sx, ty) == T_ROAD,
                            "card {level}: no road at ({sx}, {ty})"
                        );
                    }
                }
                _ => {}
            }
        }
        for (index, gun) in aa_guns(level).iter().enumerate() {
            assert!(index < 8, "card {level}: more guns than bits");
            near(gun.tx, gun.ty);
        }
    }
}

/// A save from the thirty-card cartridge carries over. Its numbers are the
/// cards', so the clears and marks stay; the furthest card becomes its place
/// in the new order. Then the new record reads back as it was written.
#[test_case]
fn first_save_carries_over(_gba: &mut agb::Gba) {
    let fold = |bytes: &[u8], salt: u8| bytes.iter().fold(salt, |out, byte| out ^ byte);
    let mut old = [0u8; 48];
    old[..4].copy_from_slice(&[0x41, 0x52, 0x43, 0x31]);
    // The lane is the furthest card, cards 1 to 12 are cleared, the briefing is card 10.
    old[4] = 12;
    old[5] = 0xFF;
    old[6] = 9;
    old[7] = 12 ^ 0xFF ^ 9 ^ 0x5A;
    old[8] = 0x0F;
    old[9] = 0x0F ^ 0xA5;
    old[11] = 0x3C;
    old[13] = 0xC3;
    // Three marks on card 1 and on card 4.
    old[14] = 0b0000_0111;
    old[15] = 0b0000_1110;
    old[24] = fold(&old[14..24], 0x6B);
    old[32] = fold(&old[25..32], 0xC5);
    old[35] = 0x3D;
    old[37] = 0xA7;
    for (index, byte) in old.into_iter().enumerate() {
        unsafe { (0x0E00_0000 as *mut u8).add(index).write_volatile(byte) };
    }
    let progress = load_progress();
    assert!(progress.cleared == 0xFFF, "cleared {:#x}", progress.cleared);
    assert!(
        progress.unlocked == slot_of(12),
        "unlocked {}",
        progress.unlocked
    );
    assert!(progress.level == 9);
    assert!(progress.mark_row(0) == 7 && progress.mark_row(3) == 7);
    assert!(progress.mark_row(1) == 0);
    // The stage cards before the lane are open, and the one after it is not.
    assert!(slot_of(31) < progress.unlocked && slot_of(32) > progress.unlocked);
    store_progress(&progress);
    let again = load_progress();
    assert!(again.cleared == progress.cleared && again.unlocked == progress.unlocked);
    assert!(again.level == progress.level && again.mark_row(3) == 7);
    for index in 0..48 {
        unsafe { (0x0E00_0000 as *mut u8).add(index).write_volatile(0) };
    }
}

/// Timers 2 and 3 in cascade: CPU cycles, 32 bits wide.
fn clock_start() {
    unsafe {
        (0x0400_010A as *mut u16).write_volatile(0);
        (0x0400_010E as *mut u16).write_volatile(0);
        (0x0400_0108 as *mut u16).write_volatile(0);
        (0x0400_010C as *mut u16).write_volatile(0);
        (0x0400_010E as *mut u16).write_volatile(0x0084);
        (0x0400_010A as *mut u16).write_volatile(0x0080);
    }
}

fn clock() -> u32 {
    loop {
        let (high, low, again) = unsafe {
            (
                (0x0400_010C as *const u16).read_volatile(),
                (0x0400_0108 as *const u16).read_volatile(),
                (0x0400_010C as *const u16).read_volatile(),
            )
        };
        if high == again {
            return u32::from(high) << 16 | u32::from(low);
        }
    }
}

/// Cycles spent building the floor table, drawing the objects, and writing
/// the ground map.
#[derive(Default)]
struct Bench {
    banks: u32,
    vram_used: u32,
    vram_top: u32,
    objects: u32,
    floor: u32,
    draw: u32,
    ground: u32,
    frames: u32,
}

/// The flight picture, as the main loop draws it.
fn glass_frame(
    gfx: &mut agb::display::Graphics,
    world: &World,
    ground: &mut AffineBackground,
    sensor: Sensor,
    bench: &mut Bench,
) {
    forget_sheets();
    let looking_down = world.looking_down();
    set_nadir(if looking_down {
        nadir_scale(world.alt)
    } else {
        0
    });
    let horizon = if looking_down { -160 } else { world.horizon() };
    let eye = Eye::new(world, horizon);
    let start = clock();
    build_floor(
        ring_space(world.x),
        ring_space(world.y),
        world.alt.max(256),
        world.yaw,
        horizon,
        world.haze_cell(),
    );
    bench.floor += clock().wrapping_sub(start);
    let sky_h = sky_bottom(horizon, world.alt);
    let sealed = world.signal_loss() >= 16;
    let blown = world.recover > 0;
    queue_look(
        sensor,
        if blown {
            Rgb::new(32, 32, 32)
        } else {
            match sensor {
                Sensor::Night => Rgb::new(16, 96, 16),
                Sensor::Heat => Rgb::new(2, 1, 4),
                Sensor::Ir => Rgb::new(5, 5, 5),
                Sensor::Day => Rgb::new(120, 152, 168),
                Sensor::Winter => Rgb::new(184, 176, 144),
            }
        }
        .to_rgb15(),
    );
    let start = clock();
    let mut frame = gfx.frame();
    let ground_id = ground.show(&mut frame);
    mask_view(&mut frame, ground_id, sky_h, true, false, world.bloom > 0);
    if blown {
        draw_hud(&mut frame, world, true);
        draw_cockpit(&mut frame, world, &eye, true);
        draw_noise(&mut frame, world.age, true);
    } else {
        draw_hud(&mut frame, world, world.signal_loss() >= 8);
        draw_cockpit(&mut frame, world, &eye, world.signal_loss() >= 8);
        draw_hurt(&mut frame, world);
        if horizon > 12 && !sealed {
            draw_glass_glare(&mut frame, world, horizon, sensor);
        }
        if sealed {
            draw_noise(&mut frame, world.age, true);
        } else {
            set_detail(true);
            draw_world(&mut frame, world, &eye, true, sensor);
            draw_horizon(&mut frame, world, horizon, sky_h, true, sensor);
        }
    }
    bench.draw += clock().wrapping_sub(start);
    bench.frames += 1;
    frame.commit();
    arm_ground();
    arm_haze(world.bloom as i32);
    publish_floor();
}

struct Digest(u32);

impl Digest {
    fn eat(&mut self, value: u32) {
        self.0 = (self.0 ^ value).wrapping_mul(0x0100_0193);
    }

    fn eat_halves(&mut self, base: usize, halves: usize) {
        for index in 0..halves {
            self.eat(u32::from(unsafe {
                ((base + index * 2) as *const u16).read_volatile()
            }));
        }
    }

    /// One object as it lands on the screen. The tile slot and the palette
    /// bank are wherever the allocator put them, so the pixels are read
    /// through them as colours instead.
    fn eat_object(&mut self, slot: usize) {
        let oam = 0x0700_0000 as *const u16;
        let (a0, a1, a2) = unsafe {
            (
                oam.add(slot * 4).read_volatile(),
                oam.add(slot * 4 + 1).read_volatile(),
                oam.add(slot * 4 + 2).read_volatile(),
            )
        };
        if a0 & 0x0300 == 0x0200 {
            self.eat(0xDEAD);
            return;
        }
        self.eat(u32::from(a0));
        // Which matrix slot an affine object got is the allocator's business.
        // The matrix itself is read below.
        self.eat(u32::from(if a0 & 0x0100 != 0 { a1 & !0x3E00 } else { a1 }));
        self.eat(u32::from(a2 & 0x0C00));
        if a0 & 0x0100 != 0 {
            let group = usize::from((a1 >> 9) & 31);
            for entry in 0..4 {
                self.eat(u32::from(unsafe {
                    oam.add(group * 16 + entry * 4 + 3).read_volatile()
                }));
            }
        }
        const TILES: [[(usize, usize); 4]; 3] = [
            [(1, 1), (2, 2), (4, 4), (8, 8)],
            [(2, 1), (4, 1), (4, 2), (8, 4)],
            [(1, 2), (1, 4), (2, 4), (4, 8)],
        ];
        let shape = usize::from(a0 >> 14).min(2);
        let (w, h) = TILES[shape][usize::from(a1 >> 14)];
        let tile = usize::from(a2 & 0x03FF);
        let bank = usize::from(a2 >> 12);
        let wide = a0 & 0x2000 != 0;
        for index in 0..w * h {
            let base = 0x0601_0000 + (tile + index * if wide { 2 } else { 1 }) * 32;
            for byte in 0..if wide { 64 } else { 32 } {
                let pair = unsafe { ((base + byte) as *const u8).read_volatile() };
                for pixel in if wide {
                    [pair, 0]
                } else {
                    [pair & 15, pair >> 4]
                } {
                    if pixel == 0 {
                        self.eat(0x1_0000);
                    } else {
                        let entry = if wide {
                            usize::from(pixel)
                        } else {
                            bank * 16 + usize::from(pixel)
                        };
                        self.eat(u32::from(unsafe {
                            (0x0500_0200 as *const u16).add(entry).read_volatile()
                        }));
                    }
                }
            }
        }
    }

    /// What the hardware draws from: every object, the ground palette, the
    /// layer and blend registers, and the floor's scanline table.
    fn eat_picture(&mut self) {
        // The vblank after a commit is where the handlers write the glass and the
        // objects. Reading before it depends on how long the frame took.
        VBlank::get().wait_for_vblank();
        for slot in 0..128 {
            self.eat_object(slot);
        }
        self.eat_halves(0x0500_0000, 256);
        self.eat_halves(0x0400_0000, 1);
        self.eat_halves(0x0400_0048, 2);
        // BLDCNT and BLDALPHA. BLDY is write-only and reads back as open bus.
        self.eat_halves(0x0400_0050, 2);
        let lines = unsafe { &(*FLOOR.lines.get())[*FLOOR.front.get()] };
        self.eat_halves(lines.as_ptr() as usize, lines.len() * 8);
    }
}

/// Each glass, each airframe, a stretch of the pilot's flight drawn frame by
/// frame. A screenshot depends on where the frame is when it is taken, so this
/// compares what the hardware is given instead. A change that only reorganises
/// the drawing leaves every digest as it is.
#[test_case]
fn flight_glass(gba: &mut agb::Gba) {
    const SHOTS: [(u8, i32); 12] = [
        (0, 90),
        (2, 160),
        (5, 200),
        (6, 200),
        (7, 220),
        (9, 260),
        (11, 160),
        (20, 200),
        (23, 120),
        (26, 200),
        (31, 220),
        (32, 200),
    ];
    reserve_glass();
    install_floor();
    clock_start();
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    let floor = ground();
    clear_ground_map();
    for (level, frames) in SHOTS {
        let sensor = sensor_of(level);
        hide_ground();
        let mut hash = Digest(0x811C_9DC5);
        let mut bench = Bench::default();
        let world = crate::pilot::fly_for(floor, level, frames, &mut |world, floor| {
            glass_frame(&mut gfx, world, floor, sensor, &mut bench);
            hash.eat_picture();
            let mut starts = [(0u16, 0u32); 128];
            let mut count = 0;
            for slot in 0..128usize {
                let oam = 0x0700_0000 as *const u16;
                let (a0, a1, a2) = unsafe {
                    (
                        oam.add(slot * 4).read_volatile(),
                        oam.add(slot * 4 + 1).read_volatile(),
                        oam.add(slot * 4 + 2).read_volatile(),
                    )
                };
                if a0 & 0x0300 != 0x0200 {
                    bench.objects += 1;
                    let tiles = [[1, 4, 16, 64], [2, 4, 8, 32], [2, 4, 8, 32]]
                        [usize::from(a0 >> 14).min(2)][usize::from(a1 >> 14)];
                    bench.vram_top = bench.vram_top.max(u32::from(a2 & 0x03FF) + tiles);
                    if !starts[..count]
                        .iter()
                        .any(|&(start, _)| start == a2 & 0x03FF)
                    {
                        starts[count] = (a2 & 0x03FF, tiles);
                        count += 1;
                    }
                }
            }
            let mut seen = 0u16;
            for slot in 0..128usize {
                let oam = 0x0700_0000 as *const u16;
                let (a0, a2) = unsafe {
                    (
                        oam.add(slot * 4).read_volatile(),
                        oam.add(slot * 4 + 2).read_volatile(),
                    )
                };
                if a0 & 0x0300 != 0x0200 && a0 & 0x2000 == 0 {
                    seen |= 1 << (a2 >> 12);
                }
            }
            bench.banks = bench.banks.max(seen.count_ones());
            let used: u32 = starts[..count].iter().map(|&(_, tiles)| tiles).sum();
            bench.vram_used = bench.vram_used.max(used);
            let start = clock();
            world.sync_ground(&mut || {});
            bench.ground += clock().wrapping_sub(start);
        });
        // The floor map the frames wrote.
        hash.eat_halves(0x0600_C000, 8192);
        agb::println!("GLASS {:02} {:08x} age {}", level + 1, hash.0, world.age);
        let per = bench.frames.max(1);
        agb::println!(
            "BENCH {:02} objects {} banks {} vram {} top {}/1024 floor {} draw {} ground {} cycles a frame",
            level + 1,
            bench.objects / per,
            bench.banks,
            bench.vram_used,
            bench.vram_top,
            bench.floor / per,
            bench.draw / per,
            bench.ground / per
        );
    }
}

/// The cartridge's own division against the library's 64-bit one, which it
/// does not replace: the edges, then a spread of sizes. `black_box` keeps
/// the compiler from folding either side at build time.
#[test_case]
fn division_matches(_gba: &mut agb::Gba) {
    use core::hint::black_box;
    let edges = [
        0i32,
        1,
        -1,
        2,
        -2,
        3,
        7,
        -7,
        8,
        255,
        256,
        -256,
        1 << 15,
        65535,
        1 << 30,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN + 1,
        -(1 << 30),
    ];
    let mut seed = 0x1234_5678u32;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    let mut pairs = 0;
    for round in 0..20_000 {
        let (n, d) = if round < edges.len() * edges.len() {
            (edges[round / edges.len()], edges[round % edges.len()])
        } else {
            let shift_n = next() % 32;
            let shift_d = next() % 32;
            ((next() >> shift_n) as i32, (next() >> shift_d) as i32 | 1)
        };
        if d == 0 || (n == i32::MIN && d == -1) {
            continue;
        }
        let (bn, bd) = (black_box(n), black_box(d));
        let (wide_n, wide_d) = (i64::from(bn), i64::from(bd));
        assert!(
            i64::from(bn / bd) == wide_n / wide_d && i64::from(bn % bd) == wide_n % wide_d,
            "{n} / {d}"
        );
        let (un, ud) = (bn as u32, bd as u32);
        let (wide_un, wide_ud) = (u64::from(un), u64::from(ud));
        assert!(
            u64::from(un / ud) == wide_un / wide_ud && u64::from(un % ud) == wide_un % wide_ud,
            "{un} / {ud} unsigned"
        );
        pairs += 1;
    }
    // The floor's reciprocal division, every divisor in its table.
    for d in 1..512 {
        for step in 0..64u32 {
            let n = black_box(match step {
                0 => 0,
                1 => i32::MAX,
                2 => d - 1,
                3 => d,
                4 => d * 1000 - 1,
                _ => (next() >> (1 + step % 31)) as i32,
            });
            assert!(line_div(n, d) == n / d, "line {n} / {d}");
            pairs += 1;
        }
    }
    agb::println!("division checked on {} pairs", pairs);
}

/// Every cell of the ground window holds its own tile. A byte store to
/// background VRAM writes the whole halfword, and once wrote each tile over
/// its neighbour.
#[test_case]
fn ground_map_holds_each_tile(_gba: &mut agb::Gba) {
    let floor = ground();
    let mut world = World::preview(floor, 2);
    while !world.boot_ready() {
        world.boot_step(floor, 24, &mut || {});
    }
    let (tx, ty) = (tile_of(world.x), tile_of(world.y));
    for y in ty - 20..ty + 20 {
        for x in tx - 20..tx + 20 {
            let cell = (y.rem_euclid(VIEW) * VIEW + x.rem_euclid(VIEW)) as usize;
            let held = unsafe { ((0x0600_C000 + cell) as *const u8).read_volatile() };
            assert!(held == world.ground_at(x, y) + 1, "cell {x},{y}");
        }
    }
}

/// The pictures in docs/GUIDE.md: the cards, then each kind of sortie a
/// stretch into the pilot's flight. Only with `--features screens`.
#[cfg(feature = "screens")]
#[test_case]
fn guide_screens(gba: &mut agb::Gba) {
    const FLIGHTS: [(u8, i32, &str); 13] = [
        (0, 90, "gate"),
        (2, 160, "trench"),
        (4, 120, "convoy"),
        (9, 220, "night"),
        (6, 240, "heat"),
        (13, 260, "guidance"),
        (11, 160, "fpv"),
        (12, 300, "lane"),
        (20, 200, "lift"),
        (23, 150, "crows"),
        (26, 200, "winter"),
        (31, 40, "scout"),
        (32, 400, "guns"),
    ];
    reserve_glass();
    install_floor();
    clock_start();
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    let floor = ground();
    clear_ground_map();
    let vblank = VBlank::get();
    hide_ground();
    for (name, card) in [
        ("studio", None),
        ("briefing", Some(0u8)),
        ("briefing_lane", Some(12)),
        ("briefing_stage", Some(30)),
    ] {
        vblank.wait_for_vblank();
        forget_sheets();
        let mut frame = gfx.frame();
        if let Some(level) = card {
            gfx_backdrop(Rgb::new(24, 64, 128));
            draw_select(
                &mut frame,
                level,
                0b101,
                (slot_of(level) > 0, true),
                0,
                sensor_of(level),
                None,
            );
        } else {
            gfx_backdrop(Rgb::new(120, 152, 168));
            draw_studio(&mut frame, 0);
        }
        frame.commit();
        agb::test_runner::assert_image_output(&alloc::format!("target/screens/{name}.png"));
    }

    let mut bench = Bench::default();
    for (level, frames, name) in FLIGHTS {
        let sensor = sensor_of(level);
        hide_ground();
        let world = crate::pilot::fly_for(floor, level, frames, &mut |world, floor| {
            glass_frame(&mut gfx, world, floor, sensor, &mut bench);
            world.sync_ground(&mut || {});
        });
        for _ in 0..3 {
            glass_frame(&mut gfx, &world, floor, sensor, &mut bench);
        }
        agb::test_runner::assert_image_output(&alloc::format!("target/screens/{name}.png"));
    }
}

/// Colour 0 of the background palette, which the cards use as the backdrop.
#[cfg(feature = "screens")]
fn gfx_backdrop(colour: Rgb) {
    unsafe { (0x0500_0000 as *mut u16).write_volatile(colour.to_rgb15().0) };
}

/// The guide's picture of the fire: a tank set off through its hatch, a
/// truck bombed after it, then a fuel dump that takes the one beside it.
/// The blasts are planted, so the moment is the same on every run.
#[cfg(feature = "screens")]
#[test_case]
fn blast_screen(gba: &mut agb::Gba) {
    reserve_glass();
    install_floor();
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    let floor = ground();
    clear_ground_map();
    hide_ground();
    let mut bench = Bench::default();
    let mut idle = || {};
    let mut world = World::preview(floor, 3);
    while !world.boot_ready() {
        world.boot_step(floor, 24, &mut idle);
    }
    let x0 = tile_mid(24);
    let y0 = tile_mid(46);
    world.soldiers = [dead_soldier(); MAX_SOLDIERS];
    world.trucks = [dead_truck(); MAX_TRUCKS];
    world.tanks = [dead_tank(); MAX_TANKS];
    world.bunkers = [dead_bunker(); MAX_BUNKERS];
    world.drums = [dead_drum(); MAX_DRUMS];
    world.tanks[0] = Tank {
        x: x0,
        y: y0,
        dir: 0,
        vertical: false,
        home_tx: 24,
        home_ty: 46,
        objective: false,
        hp: HP_TANK,
        cool: 255,
        alive: true,
    };
    world.trucks[0] = Truck {
        x: x0 - (30 << 8),
        y: y0 + (6 << 8),
        hp: HP_TRUCK,
        cool: 255,
        alive: true,
        objective: false,
        ..dead_truck()
    };
    for (slot, (dx, dy)) in [(26, 4), (40, -8)].into_iter().enumerate() {
        world.drums[slot] = Drum {
            x: x0 + (dx << 8),
            y: y0 + (dy << 8),
            home_tx: tile_of(x0 + (dx << 8)),
            home_ty: tile_of(y0 + (dy << 8)),
            alive: true,
        };
    }
    world.x = x0 + (6 << 8);
    world.y = y0 + (130 << 8);
    world.alt = 14 << 8;
    world.pitch = num!(0.01);
    world.refill(&mut idle);
    let vblank = VBlank::get();
    for age in 0..=264 {
        world.age = age;
        match age {
            10 => world.explode(floor, x0, y0, 10, 0, KIND_MINE),
            150 => world.explode(floor, x0 - (30 << 8), y0 + (6 << 8), 14, 0, KIND_BOMB),
            250 => world.set_off_drum(0),
            _ => {}
        }
        world.update_pops(floor);
        world.update_booms();
        if world.bloom > 0 {
            world.bloom -= 1;
        }
        vblank.wait_for_vblank();
        world.sync_ground(&mut || {});
        glass_frame(&mut gfx, &world, floor, Sensor::Day, &mut bench);
    }
    agb::test_runner::assert_image_output("target/screens/blast.png");
}
