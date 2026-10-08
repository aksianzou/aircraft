//! Forest drone on the Game Boy Advance.
//!
//! The ground is a mode 7 floor. Every level flies forward: left and right yaw,
//! down dives toward the ground, up climbs away. The last level is kamikaze.

#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use agb::{
    display::{
        Priority, Rgb,
        tiled::{AffineBackground, AffineBackgroundSize, AffineBackgroundWrapBehaviour},
    },
    fixnum::{Num, num},
    include_aseprite, include_background_gfx,
    input::{Button, ButtonController, Tri},
    sound::mixer::{ChannelId, Frequency},
};

include_background_gfx!(mod backgrounds, TERRAIN => 256 "gfx/terrain.png");

include_aseprite!(
    mod sprites,
    "gfx/reticle.png",
    "gfx/bomb.png",
    "gfx/he.png",
    "gfx/mine.png",
    "gfx/vog.png",
    16x16 "gfx/boom.png",
    32x32 "gfx/truck.png",
    32x32 "gfx/tank.png",
    32x32 "gfx/tank_top.png",
    32x32 "gfx/truck_top.png",
    32x32 "gfx/bunker_top.png",
    32x32 "gfx/bunker.png",
    32x32 "gfx/soldier.png",
    16x16 "gfx/foe.png",
    32x32 "gfx/geran.png",
    8x8 "gfx/spark.png",
    8x8 "gfx/digits.png",
    "gfx/lbl_score.png",
    "gfx/lbl_bomb.png",
    "gfx/lbl_he.png",
    "gfx/lbl_mine.png",
    "gfx/lbl_vog.png",
    "gfx/lbl_alt.png",
    "gfx/lbl_drone.png",
    16x16 "gfx/font.png",
    "gfx/help_quad.png",
    "gfx/help_fpv.png",
    "gfx/lvl_trench.png",
    "gfx/lvl_officer.png",
    "gfx/lvl_night.png",
    "gfx/lvl_heat.png",
    "gfx/help_heat.png",
    "gfx/lvl_guide.png",
    "gfx/help_guide.png",
    "gfx/lvl_armor.png",
    "gfx/help_armor.png",
    "gfx/lvl_pill.png",
    "gfx/help_pill.png",
    "gfx/lvl_column.png",
    "gfx/help_column.png",
    "gfx/lvl_rush.png",
    "gfx/help_rush.png",
    "gfx/lvl_push.png",
    "gfx/help_push.png",
    "gfx/lvl_lane.png",
    "gfx/help_lane.png",
    "gfx/lvl_flare.png",
    "gfx/help_flare.png",
    "gfx/lvl_nest.png",
    "gfx/help_nest.png",
    "gfx/lvl_dart.png",
    "gfx/help_dart.png",
    "gfx/lvl_bend.png",
    "gfx/help_bend.png",
    "gfx/lvl_skirt.png",
    "gfx/help_skirt.png",
    "gfx/lvl_lift.png",
    "gfx/help_lift.png",
    "gfx/lvl_haul.png",
    "gfx/help_haul.png",
    "gfx/lvl_swarm.png",
    "gfx/help_swarm.png",
    "gfx/lvl_crows.png",
    "gfx/help_crows.png",
    "gfx/lvl_cover.png",
    "gfx/help_cover.png",
    "gfx/lvl_gate.png",
    "gfx/lvl_fpv.png",
    "gfx/lvl_convoy.png",
    "gfx/lvl_hq.png",
    32x32 "gfx/marble.png",
    32x32 "gfx/hq.png",
    "gfx/help_gate.png",
    "gfx/help_convoy.png",
    "gfx/crash.png",
    "gfx/why_cell.png",
    "gfx/why_hull.png",
    "gfx/why_truck.png",
    "gfx/why_line.png",
    "gfx/why_base.png",
    "gfx/win.png",
    32x16 "gfx/cloud.png",
    32x16 "gfx/horizon.png",
    32x32 "gfx/panel.png",
    8x32 "gfx/post.png",
    32x8 "gfx/beam.png",
    32x32 "gfx/radar.png",
    32x16 "gfx/adi_bar.png",
    32x16 "gfx/adi_wings.png",
    8x8 "gfx/blip.png",
    8x8 "gfx/dot.png",
    32x8 "gfx/glass.png",
    8x8 "gfx/round.png",
    16x16 "gfx/wep.png",
    32x8 "gfx/hull.png",
    32x32 "gfx/crack.png",
    8x8 "gfx/static.png",
    8x8 "gfx/bead.png",
    32x16 "gfx/shadow.png",
    8x8 "gfx/bracket.png",
    8x8 "gfx/north.png",
    "gfx/pause_resume.png",
    "gfx/pause_resume_on.png",
    "gfx/pause_exit.png",
    "gfx/pause_exit_on.png",
    64x64 "gfx/noise.png",
    64x64 "gfx/flash.png",
    32x32 "gfx/sun.png",
    32x32 "gfx/moon.png",
    8x8 "gfx/star.png",
);

mod audio;
mod draw;
mod save;
mod world;

use audio::*;
use draw::*;
use save::*;
use world::*;

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    core::hint::black_box(&SAVE_KIND);
    let mut gfx = gba.graphics.get();
    gfx.set_background_palettes(backgrounds::PALETTES);
    // Colour 0 is the backdrop and is unused by the ground tiles. The sky is
    // just this fill; clouds and ridges are sprites on top of it.
    gfx.set_background_palette_colour_256(0, Rgb::new(120, 152, 168).to_rgb15());

    // agb refuses a 16 KB map (its heap check is exclusive), so the loader is
    // 64×64. The real 128×128 window is written straight into the next 16 KB.
    let mut ground = AffineBackground::new(
        Priority::P1,
        AffineBackgroundSize::Background64x64,
        AffineBackgroundWrapBehaviour::Wrap,
    );
    clear_ground_map();
    install_floor();
    let mut progress = load_progress();
    let mut phase = Phase::Splash;
    let mut splash_age = 0i32;
    let mut menu = 0u8;
    let mut pick_hold = 0i32;
    let mut pick_wait = 0i32;
    let mut input = ButtonController::new();
    let mut mixer = gba.mixer.mixer(Frequency::Hz10512);
    let mut music_id: Option<ChannelId> = None;
    let mut end_tune = false;
    let mut world = World::new(&mut ground, progress.level, &mut || {
        mixer.frame();
    });

    loop {
        input.update();

        if phase == Phase::Splash {
            let skip = input.is_just_pressed(Button::Start) || input.is_just_pressed(Button::A);
            splash_age += 1;
            if skip || splash_age > 240 {
                phase = Phase::Title;
                splash_age = 0;
            }
        } else if phase == Phase::Title {
            // The card changes on the tap. The field follows after the stick is released.
            let edge = input.just_pressed_x_tri();
            let held = input.x_tri();
            let step = if edge != Tri::Zero {
                pick_hold = 0;
                edge
            } else if held != Tri::Zero {
                pick_hold += 1;
                if pick_hold >= 8 && pick_hold % 3 == 0 {
                    held
                } else {
                    Tri::Zero
                }
            } else {
                pick_hold = 0;
                Tri::Zero
            };
            let picked = match step {
                Tri::Positive if progress.level < progress.unlocked => Some(progress.level + 1),
                Tri::Negative if progress.level > 0 => Some(progress.level - 1),
                _ => None,
            };
            if let Some(level) = picked {
                progress.level = level;
                store_progress(&progress);
                pick_wait = 24;
            }
            if glass_open(&progress, progress.level) {
                let now = progress.glass_n(progress.level);
                let next = match input.just_pressed_y_tri() {
                    Tri::Positive => Some((now + 1) % 3),
                    Tri::Negative => Some((now + 2) % 3),
                    Tri::Zero => None,
                };
                if let Some(glass) = next {
                    progress.set_glass_n(progress.level, glass);
                    store_progress(&progress);
                }
            }
        }

        if phase == Phase::Pause {
            match input.just_pressed_y_tri() {
                Tri::Positive => menu = 1,
                Tri::Negative => menu = 0,
                Tri::Zero => {}
            }
            let choose =
                input.is_just_pressed(Button::A) || input.is_just_pressed(Button::Start);
            if choose {
                if menu == 0 {
                    phase = Phase::Fly;
                } else {
                    world = World::new(&mut ground, world.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
            }
        } else if phase != Phase::Splash && input.is_just_pressed(Button::Start) {
            match phase {
                Phase::Title => {
                    world = World::new(&mut ground, progress.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Fly;
                }
                Phase::Crash => {
                    world = World::new(&mut ground, world.level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
                Phase::Win | Phase::Done => {
                    let next = if world.level < progress.unlocked {
                        world.level + 1
                    } else {
                        world.level
                    };
                    progress.level = next;
                    store_progress(&progress);
                    world = World::new(&mut ground, next, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                }
                Phase::Fly => {
                    menu = 0;
                    phase = Phase::Pause;
                }
                Phase::Splash | Phase::Pause => {}
            }
        }

        match phase {
            Phase::Splash => {}
            Phase::Title => {
                world.yaw += num!(1) / Num::new(360);
                wrap_yaw(&mut world.yaw);
                // The card is already the new sortie. The field follows once the stick is up.
                if input.x_tri() != Tri::Zero {
                    pick_wait = 24;
                } else if pick_wait > 0 {
                    pick_wait -= 1;
                } else if world.level != progress.level {
                    world = World::preview(&mut ground, progress.level);
                    world.boot_step(&mut ground, 8, &mut || {
                        mixer.frame();
                    });
                } else if !world.boot_ready() {
                    world.boot_step(&mut ground, 8, &mut || {
                        mixer.frame();
                    });
                }
                if world.boot_ready() && world.level == progress.level {
                    world.update_trucks();
                    world.update_tanks();
                    world.update_soldiers();
                }
            }
            Phase::Fly => {
                world.fly(&input, &mut ground);
                let restart = world.again;
                if world.targets <= 0 {
                    tone(&mut mixer, CLEAR_SFX);
                    progress.cleared |= 1u32 << world.level;
                    progress.keep_marks(world.level, earned_marks(&world));
                    if world.level + 1 < LEVEL_COUNT {
                        progress.unlocked = progress.unlocked.max(world.level + 1);
                    }
                    store_progress(&progress);
                    phase = Phase::Win;
                    if world.level + 1 >= LEVEL_COUNT {
                        splash_age = 0;
                    }
                } else if world.crashed {
                    phase = Phase::Crash;
                }
                let shot = world.sfx & SFX_GUN != 0;
                let blew = world.sfx & SFX_BOOM != 0;
                play_cues(&mut mixer, &mut world);
                if restart && phase == Phase::Fly {
                    let level = world.level;
                    world = World::new(&mut ground, level, &mut || {
                        mixer.frame();
                    });
                    phase = Phase::Title;
                } else if phase == Phase::Fly {
                    ambience(
                        &mut mixer,
                        &world,
                        shot,
                        blew,
                        glass_of(&progress, world.level),
                    );
                }
            }
            Phase::Pause | Phase::Crash | Phase::Win | Phase::Done => {}
        }

        if phase != Phase::Pause {
            world.maintain_actors();
        }

        // Ahead of the floor build. A second fill before vblank covers a long
        // frame, which otherwise replays a slice of the music.
        let ending = phase == Phase::Win && world.level + 1 >= LEVEL_COUNT;
        if ending != end_tune {
            if let Some(id) = music_id.take() {
                if let Some(channel) = mixer.channel(&id) {
                    channel.stop();
                }
            }
            end_tune = ending;
        }
        if ending {
            end_bed(&mut mixer, &mut music_id);
        } else {
            music_bed(
                &mut mixer,
                &mut music_id,
                !matches!(phase, Phase::Fly | Phase::Pause),
            );
        }
        mixer.frame();

        if phase == Phase::Splash {
            gfx.set_background_palette_colour_256(0, Rgb::new(0, 0, 0).to_rgb15());
            let mut frame = gfx.frame();
            draw_studio(&mut frame, splash_age);
            mixer.frame();
            frame.commit();
            // Same blend the flight uses, so the glint is glass and the next card is not surprised.
            arm_haze(0);
            continue;
        }
        if phase == Phase::Win && world.level + 1 >= LEVEL_COUNT {
            splash_age += 1;
            gfx.set_background_palette_colour_256(0, Rgb::new(0, 0, 0).to_rgb15());
            let mut frame = gfx.frame();
            draw_credits(&mut frame, splash_age);
            mixer.frame();
            frame.commit();
            continue;
        }

        let looking_down = world.hover && matches!(phase, Phase::Fly | Phase::Pause);
        set_nadir(if looking_down {
            nadir_scale(world.alt)
        } else {
            0
        });
        let horizon = if looking_down {
            -160
        } else {
            world.horizon()
        };
        let (sin_r, cos_r) = world.heading();
        build_floor(
            ring_space(world.x),
            ring_space(world.y),
            world.alt.max(256),
            world.yaw,
            horizon,
        );

        let sky_h = sky_bottom(horizon, world.alt);
        let sensor = glass_of(
            &progress,
            if phase == Phase::Title {
                progress.level
            } else {
                world.level
            },
        );
        let playing = matches!(phase, Phase::Fly | Phase::Pause);
        let blown = playing && world.recover > 0;
        paint_sensor(sensor);
        gfx.set_background_palette_colour_256(
            0,
            if blown {
                Rgb::new(32, 32, 32)
            } else {
                match sensor {
                    Sensor::Night => Rgb::new(1, 8, 2),
                    Sensor::Heat => Rgb::new(2, 1, 4),
                    Sensor::Ir => Rgb::new(5, 5, 5),
                    Sensor::Day => Rgb::new(120, 152, 168),
                    Sensor::Winter => Rgb::new(168, 184, 196),
                }
            }
            .to_rgb15(),
        );

        let mut frame = gfx.frame();
        let ground_id = ground.show(&mut frame);
        let cockpit = playing;
        mask_view(&mut frame, ground_id, sky_h, cockpit, false);

        // First object submitted is the front-most. Instruments go in before
        // the snow, so the operator's panel stays up while the glass is noise.
        if blown {
            if phase == Phase::Pause {
                draw_pause(&mut frame, menu);
            }
            draw_hud(&mut frame, &world);
            draw_cockpit(&mut frame, &world, sin_r, cos_r);
            draw_noise(&mut frame, world.age);
            mixer.frame();
            frame.commit();
            arm_ground();
            arm_haze(world.bloom as i32);
            publish_floor();
            world.sync_ground(&mut || {
                mixer.frame();
            });
            continue;
        }
        match phase {
            Phase::Title => {
                show_text(&mut frame, b"DRONE", 4);
                let picked = progress.level;
                let card = match task_of(picked) {
                    Task::Trench => sprites::LVL_TRENCH.sprite(0),
                    Task::Officer => sprites::LVL_OFFICER.sprite(0),
                    Task::Night => sprites::LVL_NIGHT.sprite(0),
                    Task::Heat => sprites::LVL_HEAT.sprite(0),
                    Task::Gate => sprites::LVL_GATE.sprite(0),
                    Task::Fpv => sprites::LVL_FPV.sprite(0),
                    Task::Convoy => sprites::LVL_CONVOY.sprite(0),
                    Task::Hq => sprites::LVL_HQ.sprite(0),
                    Task::Guide => sprites::LVL_GUIDE.sprite(0),
                    Task::Armor => sprites::LVL_ARMOR.sprite(0),
                    Task::Pill => sprites::LVL_PILL.sprite(0),
                    Task::Column => sprites::LVL_COLUMN.sprite(0),
                    Task::Rush => sprites::LVL_RUSH.sprite(0),
                    Task::Push => sprites::LVL_PUSH.sprite(0),
                    Task::Lane => sprites::LVL_LANE.sprite(0),
                    Task::Flare => sprites::LVL_FLARE.sprite(0),
                    Task::Nest => sprites::LVL_NEST.sprite(0),
                    Task::Dart => sprites::LVL_DART.sprite(0),
                    Task::Bend => sprites::LVL_BEND.sprite(0),
                    Task::Skirt => sprites::LVL_SKIRT.sprite(0),
                    Task::Fetch if haul_card(picked) => sprites::LVL_HAUL.sprite(0),
                    Task::Fetch => sprites::LVL_LIFT.sprite(0),
                    Task::Swarm if defend_card(picked) => sprites::LVL_CROWS.sprite(0),
                    Task::Swarm => sprites::LVL_SWARM.sprite(0),
                    Task::Cover => sprites::LVL_COVER.sprite(0),
                };
                show(&mut frame, card, 88, 40);
                draw_digits(&mut frame, picked as i32 + 1, 2, 64, 52);
                if progress.cleared & (1u32 << picked) != 0 {
                    show(&mut frame, sprites::DOT.sprite(1), 84, 52);
                    show_ribbon(&mut frame, progress.mark_row(picked), 108, 76);
                }
                if glass_open(&progress, picked) {
                    show(
                        &mut frame,
                        sprites::GLASS.sprite(progress.glass_n(picked) as usize),
                        104,
                        88,
                    );
                }
                let help = match task_of(picked) {
                    Task::Fpv => sprites::HELP_FPV.sprite(0),
                    Task::Gate => sprites::HELP_GATE.sprite(0),
                    Task::Convoy => sprites::HELP_CONVOY.sprite(0),
                    Task::Heat => sprites::HELP_HEAT.sprite(0),
                    Task::Guide => sprites::HELP_GUIDE.sprite(0),
                    Task::Column => sprites::HELP_COLUMN.sprite(0),
                    Task::Rush => sprites::HELP_RUSH.sprite(0),
                    Task::Push => sprites::HELP_PUSH.sprite(0),
                    Task::Lane => sprites::HELP_LANE.sprite(0),
                    Task::Flare => sprites::HELP_FLARE.sprite(0),
                    Task::Nest => sprites::HELP_NEST.sprite(0),
                    Task::Dart => sprites::HELP_DART.sprite(0),
                    Task::Bend => sprites::HELP_BEND.sprite(0),
                    Task::Skirt => sprites::HELP_SKIRT.sprite(0),
                    Task::Fetch if haul_card(picked) => sprites::HELP_HAUL.sprite(0),
                    Task::Fetch => sprites::HELP_LIFT.sprite(0),
                    Task::Swarm if defend_card(picked) => sprites::HELP_CROWS.sprite(0),
                    Task::Swarm => sprites::HELP_SWARM.sprite(0),
                    Task::Cover => sprites::HELP_COVER.sprite(0),
                    Task::Armor => sprites::HELP_ARMOR.sprite(0),
                    Task::Pill => sprites::HELP_PILL.sprite(0),
                    _ => sprites::HELP_QUAD.sprite(0),
                };
                show(&mut frame, help, 88, 108);
            }
            Phase::Crash => {
                show(&mut frame, sprites::CRASH.sprite(0), 88, 44);
                let card = match world.why {
                    Why::Cell => Some(sprites::WHY_CELL.sprite(0)),
                    Why::Hull => Some(sprites::WHY_HULL.sprite(0)),
                    Why::Truck => Some(sprites::WHY_TRUCK.sprite(0)),
                    Why::Line => Some(sprites::WHY_LINE.sprite(0)),
                    Why::Base => Some(sprites::WHY_BASE.sprite(0)),
                    Why::None => None,
                };
                if let Some(card) = card {
                    show(&mut frame, card, 88, 84);
                }
            }
            Phase::Win | Phase::Done => {
                show(&mut frame, sprites::WIN.sprite(0), 88, 56);
                draw_digits(&mut frame, world.score, 4, 104, 96);
                show_ribbon(&mut frame, progress.mark_row(world.level), 108, 112);
            }
            Phase::Fly | Phase::Pause => {
                if phase == Phase::Pause {
                    draw_pause(&mut frame, menu);
                }
                draw_hud(&mut frame, &world);
                draw_cockpit(&mut frame, &world, sin_r, cos_r);
                draw_hurt(&mut frame, &world);
            }
            Phase::Splash => {}
        }
        if playing && horizon > 12 {
            draw_glass_glare(&mut frame, &world, horizon, sensor);
        }
        draw_world(
            &mut frame,
            &world,
            sin_r,
            cos_r,
            horizon,
            cockpit,
            sensor,
        );
        draw_horizon(&mut frame, &world, horizon, sky_h, cockpit, sensor);

        mixer.frame();
        frame.commit();
        // Still in vblank: retarget BG2, publish this frame's scanlines, then
        // write new map columns before the picture starts.
        arm_ground();
        arm_haze(world.bloom as i32);
        publish_floor();
        world.sync_ground(&mut || {
            mixer.frame();
        });
    }
}
