use agb::display::Priority;
use agb::display::tiled::{AffineBackground, AffineBackgroundSize, AffineBackgroundWrapBehaviour};
use agb::interrupt::VBlank;

use super::*;

fn ground() -> AffineBackground {
    AffineBackground::new(
        Priority::P1,
        AffineBackgroundSize::Background64x64,
        AffineBackgroundWrapBehaviour::Wrap,
    )
}

/// A whole clock in the air still leaves the battery star. Hover spends less.
#[test_case]
fn flight_spends_more_than_hover(_gba: &mut agb::Gba) {
    let mut floor = ground();
    let budget = time_budget(0);
    let mut flying = World::preview(&mut floor, 0);
    flying.hover = false;
    for frame in 0..budget {
        flying.age = frame;
        flying.wear();
    }
    let mut hovering = World::preview(&mut floor, 0);
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
    let mut floor = ground();
    let mut world = World::preview(&mut floor, 4);
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
    draw_select(&mut frame, 3, 0, true, false, 0, Sensor::Day, None);
    frame.commit();
    agb::test_runner::assert_image_output("tests/briefing_house.png");
}
