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
    16x16 "gfx/scrap.png",
    32x32 "gfx/smoke.png",
    32x32 "gfx/truck.png",
    64x32 "gfx/truck_near.png",
    32x32 "gfx/tank.png",
    64x32 "gfx/tank_near.png",
    32x32 "gfx/tank_top.png",
    32x32 "gfx/truck_top.png",
    32x32 "gfx/bunker_top.png",
    32x32 "gfx/bunker.png",
    64x32 "gfx/bunker_near.png",
    32x32 "gfx/soldier.png",
    32x64 "gfx/soldier_near.png",
    32x32 "gfx/soldier_top.png",
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
    8x8 "gfx/small.png",
    8x8 "gfx/role.png",
    64x32 "gfx/word.png",
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
    64x64 "gfx/hq_near.png",
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
    16x32 "gfx/grove.png",
    16x32 "gfx/grove_ir.png",
    32x64 "gfx/grove_near.png",
    32x64 "gfx/grove_ir_near.png",
    16x16 "gfx/bush.png",
    16x16 "gfx/bush_ir.png",
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
    64x64 "gfx/title.png",
    64x64 "gfx/stills.png",
    64x64 "gfx/noise.png",
    32x8 "gfx/lip.png",
    64x64 "gfx/flash.png",
    32x32 "gfx/sun.png",
    32x32 "gfx/moon.png",
    8x8 "gfx/star.png",
    16x16 "gfx/font.png",
    16x16 "gfx/card.png",
    16x16 "gfx/medal.png",
    16x16 "gfx/arrow.png",
);

mod audio;
mod divide;
mod draw;
#[cfg(test)]
mod pilot;
mod save;
#[cfg(test)]
mod tests;
mod world;

use audio::*;
use draw::*;
use save::*;
use world::*;

#[agb::entry]
fn main(mut gba: agb::Gba) -> ! {
    core::hint::black_box(&SAVE_KIND);
    reserve_glass();
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
    let mut fly_when_ready = false;
    let mut card_age = 0i32;
    let mut world = World::preview(&mut ground, progress.level);

    loop {
        input.update();
        forget_sheets();

        if phase == Phase::Splash {
            let skip = input.is_just_pressed(Button::Start) || input.is_just_pressed(Button::A);
            splash_age += 1;
            if skip {
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
                fly_when_ready = false;
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
            let choose = input.is_just_pressed(Button::A) || input.is_just_pressed(Button::Start);
            if choose {
                if menu == 0 {
                    phase = Phase::Fly;
                } else {
                    hide_ground();
                    world = World::preview(&mut ground, world.level);
                    phase = Phase::Title;
                }
            }
        } else if phase != Phase::Splash && input.is_just_pressed(Button::Start) {
            match phase {
                Phase::Title => {
                    fly_when_ready = true;
                    if world.boot_ready() || world.level != progress.level {
                        hide_ground();
                        world = World::preview(&mut ground, progress.level);
                    }
                }
                Phase::Crash => {
                    hide_ground();
                    world = World::preview(&mut ground, world.level);
                    phase = Phase::Title;
                }
                Phase::Win => {
                    let next = if world.level < progress.unlocked {
                        world.level + 1
                    } else {
                        world.level
                    };
                    progress.level = next;
                    store_progress(&progress);
                    hide_ground();
                    world = World::preview(&mut ground, next);
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
                    hide_ground();
                    world = World::preview(&mut ground, progress.level);
                }
                if fly_when_ready && world.boot_ready() && world.level == progress.level {
                    fly_when_ready = false;
                    phase = Phase::Fly;
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
                    world.bloom = 0;
                    phase = Phase::Win;
                    if world.level + 1 >= LEVEL_COUNT {
                        splash_age = 0;
                    }
                } else if world.crashed {
                    world.bloom = 0;
                    phase = Phase::Crash;
                }
                let shot = world.sfx & SFX_GUN != 0;
                let blew = world.sfx & SFX_BOOM != 0;
                play_cues(&mut mixer, &mut world);
                if restart && phase == Phase::Fly {
                    let level = world.level;
                    hide_ground();
                    world = World::preview(&mut ground, level);
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
            Phase::Pause | Phase::Crash | Phase::Win => {}
        }

        if phase != Phase::Pause && (phase != Phase::Title || world.boot_ready()) {
            world.maintain_actors();
        }

        // Ahead of the floor build. A second fill before vblank covers a long
        // frame, which otherwise replays a slice of the music.
        let ending = phase == Phase::Win && world.level + 1 >= LEVEL_COUNT;
        if ending != end_tune {
            if let Some(id) = music_id.take()
                && let Some(channel) = mixer.channel(&id)
            {
                channel.stop();
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
            gfx.set_background_palette_colour_256(0, Rgb::new(120, 152, 168).to_rgb15());
            let mut frame = gfx.frame();
            draw_studio(&mut frame, splash_age);
            mixer.frame();
            frame.commit();
            // Same blend the flight uses, so the glint is glass and the next card is not surprised.
            arm_haze(0);
            if !world.boot_ready() {
                hide_ground();
                world.boot_step(&mut ground, 6, &mut || {
                    mixer.frame();
                });
            }
            continue;
        }
        if phase == Phase::Title {
            hide_ground();
            card_age += 1;
            let sensor = glass_of(&progress, progress.level);
            gfx.set_background_palette_colour_256(0, Rgb::new(24, 64, 128).to_rgb15());
            let picked = progress.level;
            let mut frame = gfx.frame();
            draw_select(
                &mut frame,
                picked,
                progress.mark_row(picked),
                (picked > 0, picked < progress.unlocked),
                card_age,
                sensor,
                if glass_open(&progress, picked) {
                    Some(progress.glass_n(picked))
                } else {
                    None
                },
            );
            mixer.frame();
            frame.commit();
            if !world.boot_ready() {
                world.boot_step(&mut ground, 6, &mut || {
                    mixer.frame();
                });
            }
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

        let looking_down = world.looking_down() && matches!(phase, Phase::Fly | Phase::Pause);
        set_nadir(if looking_down {
            nadir_scale(world.alt)
        } else {
            0
        });
        let horizon = if looking_down { -160 } else { world.horizon() };
        let eye = Eye::new(&world, horizon);
        build_floor(
            ring_space(world.x),
            ring_space(world.y),
            world.alt.max(256),
            world.yaw,
            horizon,
            world.haze_cell(),
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
        let airframe_dead = phase == Phase::Crash && matches!(world.why, Why::Hull | Why::Cell);
        let sealed = airframe_dead || (playing && world.signal_loss() >= 16);
        let cockpit = playing || airframe_dead;
        let blown = playing && world.recover > 0;
        queue_look(
            sensor,
            if blown {
                Rgb::new(32, 32, 32)
            } else {
                match sensor {
                    // Eight-bit. A small green is shifted away and the day blue stays.
                    Sensor::Night => Rgb::new(16, 96, 16),
                    Sensor::Heat => Rgb::new(2, 1, 4),
                    Sensor::Ir => Rgb::new(5, 5, 5),
                    Sensor::Day => Rgb::new(120, 152, 168),
                    Sensor::Winter => Rgb::new(184, 176, 144),
                }
            }
            .to_rgb15(),
        );

        let mut frame = gfx.frame();
        let ground_id = ground.show(&mut frame);
        mask_view(
            &mut frame,
            ground_id,
            sky_h,
            cockpit,
            false,
            world.bloom > 0,
        );

        // First object submitted is the front-most. Instruments go in before
        // the snow, so the operator's panel stays up while the glass is noise.
        if blown {
            if phase == Phase::Pause {
                draw_pause(&mut frame, menu);
            }
            draw_hud(&mut frame, &world, true);
            draw_cockpit(&mut frame, &world, &eye, true);
            draw_noise(&mut frame, world.age, true);
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
            Phase::Title => {}
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
                if matches!(world.why, Why::Hull | Why::Cell) {
                    world.age = world.age.wrapping_add(1);
                    draw_hud(&mut frame, &world, true);
                    draw_cockpit(&mut frame, &world, &eye, true);
                }
            }
            Phase::Win => {
                show(&mut frame, sprites::WIN.sprite(0), 88, 56);
                draw_digits(&mut frame, world.score, 4, 104, 96);
                show_ribbon(&mut frame, progress.mark_row(world.level), 94, 112);
            }
            Phase::Fly | Phase::Pause => {
                if phase == Phase::Pause {
                    draw_pause(&mut frame, menu);
                }
                draw_hud(&mut frame, &world, world.signal_loss() >= 8);
                draw_cockpit(&mut frame, &world, &eye, world.signal_loss() >= 8);
                draw_hurt(&mut frame, &world);
            }
            Phase::Splash => {}
        }
        if playing && horizon > 12 && !sealed {
            draw_glass_glare(&mut frame, &world, horizon, sensor);
        }
        if sealed {
            draw_noise(&mut frame, world.age, true);
        } else {
            // The crowns and the raid sit in this call. Fill first, so that
            // work does not run on for two frames without a new music slice.
            mixer.frame();
            set_detail(playing);
            draw_world(&mut frame, &world, &eye, cockpit, sensor);
            draw_horizon(&mut frame, &world, horizon, sky_h, cockpit, sensor);
        }

        mixer.frame();
        frame.commit();
        arm_ground();
        arm_haze(world.bloom as i32);
        publish_floor();
        if phase != Phase::Title {
            world.sync_ground(&mut || {
                mixer.frame();
            });
        }
    }
}
