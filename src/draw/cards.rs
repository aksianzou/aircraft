//! The cards around the flight: the briefing, the pause, the studio and the credits.

use agb::display::{HEIGHT, WIDTH, object::Sprite};

use crate::sprites;

use super::*;

/// One card for every mission. The field loads behind it, so changing
/// sortie does not swap the picture for another splash.
pub(crate) fn draw_select(
    frame: &mut agb::display::GraphicsFrame,
    level: u8,
    marks: u8,
    (can_left, can_right): (bool, bool),
    age: i32,
    sensor: Sensor,
    glass: Option<u8>,
) {
    // The operator's screen. Blue desktop, one window, the target in the frame.
    draw_digits(frame, level as i32 + 1, 2, 8, 4);
    show_small_at(frame, b"MISSION", 28, 4);
    let (top, bottom) = mission_title(level);
    show_lockup_at(frame, top, bottom, 8, 16);
    let (craft, sight, order, note) = mission_brief(level, sensor);
    show_small_at(frame, craft, 8, 42);
    show_small_at(frame, sight, 8, 54);
    if let Some(which) = glass {
        show(frame, sprites::GLASS.sprite(which as usize), 8, 64);
    }
    show_small_at(frame, order, 8, 78);
    show_small_at(frame, note, 8, 90);
    show_stars_at(frame, marks, 8, 112);
    show_target(frame, level);
    let step = (age / 6).rem_euclid(6);
    let nudge = if step < 4 { step } else { 6 - step };
    if can_left {
        show_flip(frame, sprites::ARROW.sprite(0), 4 - nudge, 132, true, false);
    }
    if can_right {
        show_flip(
            frame,
            sprites::ARROW.sprite(0),
            220 + nudge,
            132,
            false,
            false,
        );
    }
    draw_controls(frame);
}

/// The stick. Short lines sit under the photograph; the long ones sit on the bottom.
fn draw_controls(frame: &mut agb::display::GraphicsFrame) {
    show_small_at(frame, b"CROSS AIM", 168, 104);
    show_small_at(frame, b"A CYCLE", 168, 112);
    show_small_at(frame, b"B DROP", 168, 120);
    show_small(frame, b"SELECT HOVER", 144);
    show_small(frame, b"START PAUSE", 152);
}

/// The still in the window. One photograph per card, reduced to colours the cartridge already has.
fn show_target(frame: &mut agb::display::GraphicsFrame, level: u8) {
    let index = (level as usize) % LEVEL_COUNT as usize;
    show(frame, sprites::STILLS.sprite(index), 168, 36);
}

/// Aircraft, sight, then the two lines you need before the stick goes forward.
fn mission_brief(
    level: u8,
    sensor: Sensor,
) -> (&'static [u8], &'static [u8], &'static [u8], &'static [u8]) {
    let craft: &'static [u8] = match task_of(level) {
        Task::Fpv | Task::Nest | Task::Dart => b"KAMIKAZE",
        Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => b"SPOTTER",
        Task::Fetch => b"MINE QUAD",
        _ => b"RECON QUAD",
    };
    let sight: &'static [u8] = match sensor {
        Sensor::Day => b"DAY",
        Sensor::Night => b"NIGHT SIGHT",
        Sensor::Heat => b"HEAT SIGHT",
        Sensor::Ir => b"INFRARED",
        Sensor::Winter => b"WINTER DAY",
    };
    let (order, note): (&'static [u8], &'static [u8]) = match task_of(level) {
        Task::Officer => (b"1 KILL THE OFFICER", b"2 GUN OR GRENADES"),
        Task::Trench => (b"1 SIX MEN IN TRENCH", b"2 BOMBS AND GRENADES"),
        Task::Hq => (b"1 BREAK SIX ROOF TILES", b"2 BOMBS AND SHELLS"),
        Task::Convoy => (b"1 STOP THE TRUCK", b"2 THREE BOMBS"),
        Task::Night => (b"1 THREE TRUCKS", b"2 SIX BOMBS"),
        Task::Heat => (b"1 THREE HOT TRUCKS", b"2 SIX BOMBS"),
        Task::Gate => (b"1 HOVER AND DROP", b"2 FLY THE GAP"),
        Task::Guide => (b"1 MARK THREE TRUCKS", b"2 ARTILLERY FIRES"),
        Task::Column => (b"1 LEAD THE COLUMN", b"2 ARTILLERY FIRES"),
        Task::Armor => (b"1 THREE TANKS", b"2 MINES AND BOMBS"),
        Task::Pill => (b"1 THREE BUNKERS", b"2 SIX SHELLS"),
        Task::Fpv => (b"1 EIGHT TARGETS", b"2 SIX DRONES"),
        Task::Lane => (b"1 THREE TRUCKS", b"2 STAY OUT OF GUNS"),
        Task::Rush => (b"1 STOP SIX MEN", b"2 ARTILLERY FIRES"),
        Task::Flare => (b"1 THREE TANKS", b"2 GUNS ON THE LANE"),
        Task::Nest => (b"1 FOUR NESTS", b"2 FIVE DRONES"),
        Task::Dart => (b"1 THREE TRUCKS", b"2 FOUR DRONES"),
        Task::Push => (b"1 NINE ON THE PUSH", b"2 ARTILLERY FIRES"),
        Task::Skirt => (b"1 THREE BUNKERS", b"2 PAST THE GUNS"),
        Task::Bend => (b"1 THREE TRUCKS", b"2 STAY LOW"),
        Task::Fetch if haul_card(level) => (b"1 THREE TANKS", b"2 MINES ARE FAR"),
        Task::Fetch => (b"1 THREE TANKS", b"2 ONE MINE EACH"),
        Task::Swarm if defend_card(level) => (b"1 HOLD THE PAD", b"2 EIGHT AIRCRAFT"),
        Task::Swarm => (b"1 SIX QUADS", b"2 GUN ONLY"),
        Task::Cover => (b"1 AIR AND ONE TRUCK", b"2 GUN THEN A BOMB"),
        Task::Pits => (b"1 EIGHT IN THE PITS", b"2 GRENADES AND GUN"),
        Task::Grove => (b"1 EIGHT IN THE WOOD", b"2 GUN AND FEW BOMBS"),
        Task::Drift => (b"1 TRUCK AND THREE MEN", b"2 BOMB THE TRUCK"),
        Task::Thaw => (b"1 TANKS AND FOUR MEN", b"2 MINES THEN THE GUN"),
        Task::Hearth => (b"1 OFFICER AND FOUR", b"2 STAY ABOVE 20"),
    };
    (craft, sight, order, note)
}

fn mission_title(level: u8) -> (&'static [u8], &'static [u8]) {
    match task_of(level) {
        Task::Officer => (b"OFFICER", b"HUNT"),
        Task::Trench => (b"TRENCH", b"LINE"),
        Task::Hq => (b"HOUSE", b"BOMB"),
        Task::Convoy => (b"KILL", b"TRUCK"),
        Task::Night => (b"NIGHT", b"SIGHT"),
        Task::Heat => (b"HEAT", b"SCAN"),
        Task::Gate => (b"FIRST", b"FLIGHT"),
        Task::Guide => (b"FIRE", b"MARK"),
        Task::Column => (b"COLUMN", b"MARK"),
        Task::Armor => (b"ARMOR", b"HUNT"),
        Task::Pill => (b"PILL", b"BOX"),
        Task::Fpv => (b"LAST", b"DIVE"),
        Task::Lane => (b"THE", b"LANE"),
        Task::Rush => (b"ON", b"FOOT"),
        Task::Flare => (b"NIGHT", b"LANE"),
        Task::Nest => (b"OPEN", b"NEST"),
        Task::Dart => (b"HEAT", b"DART"),
        Task::Push => (b"THE", b"PUSH"),
        Task::Skirt => (b"PAST", b"GUNS"),
        Task::Bend => (b"LOW", b"ROAD"),
        Task::Fetch if haul_card(level) => (b"LONG", b"HAUL"),
        Task::Fetch => (b"ONE", b"CHARGE"),
        Task::Swarm if defend_card(level) => (b"THE", b"CROWS"),
        Task::Swarm => (b"THE", b"SWARM"),
        Task::Cover => (b"COVER", b"FIRE"),
        Task::Pits => (b"ASH", b"PITS"),
        Task::Grove => (b"SNOW", b"WOOD"),
        Task::Drift => (b"SNOW", b"ROAD"),
        Task::Thaw => (b"MUD", b"IRON"),
        Task::Hearth => (b"PINE", b"POST"),
    }
}

/// Width of a card title letter. The face is the condensed briefing cut, not the splash.
fn card_advance(ch: u8) -> i32 {
    match ch {
        b' ' => 4,
        b'I' => 3,
        b'J' => 6,
        b'A' => 9,
        b'M' => 10,
        b'W' => 12,
        b'K' | b'Q' | b'R' | b'V' | b'X' | b'Y' => 8,
        _ => 7,
    }
}

/// Both words on one line, with a gap, so the card is a title and not a stack.
fn show_lockup_at(
    frame: &mut agb::display::GraphicsFrame,
    top: &[u8],
    bottom: &[u8],
    mut x: i32,
    y: i32,
) {
    for (index, text) in [top, bottom].into_iter().enumerate() {
        for &ch in text {
            if ch != b' ' {
                show(frame, sprites::CARD.sprite(glyph(ch)), x, y);
            }
            x += card_advance(ch);
        }
        if index == 0 {
            x += 6;
        }
    }
}

/// The outline face. Cells are 16 wide; the letter itself sits inside that.
pub(crate) fn show_face(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 8 } else { 14 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 8 } else { 14 };
        if ch != b' ' {
            show(frame, sprites::FONT.sprite(glyph(ch)), x - 1, y);
        }
        x += adv;
    }
}

fn show_stars_at(frame: &mut agb::display::GraphicsFrame, marks: u8, x: i32, y: i32) {
    for index in 0..3 {
        let which = if marks & (1 << index) != 0 { 1 } else { 0 };
        show(frame, sprites::MEDAL.sprite(which), x + index * 20, y);
    }
}

fn show_small_at(frame: &mut agb::display::GraphicsFrame, text: &[u8], mut x: i32, y: i32) {
    for &ch in text {
        let adv = if ch == b' ' { 4 } else { 6 };
        if ch != b' ' && (-8..HEIGHT).contains(&y) && (-8..WIDTH).contains(&x) {
            show(frame, sprites::SMALL.sprite(glyph(ch)), x, y);
        }
        x += adv;
    }
}

#[allow(dead_code)]
pub(crate) fn mission_card(level: u8) -> &'static Sprite {
    match task_of(level) {
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
        Task::Fetch if haul_card(level) => sprites::LVL_HAUL.sprite(0),
        Task::Fetch => sprites::LVL_LIFT.sprite(0),
        Task::Swarm if defend_card(level) => sprites::LVL_CROWS.sprite(0),
        Task::Swarm => sprites::LVL_SWARM.sprite(0),
        Task::Cover => sprites::LVL_COVER.sprite(0),
        Task::Pits => sprites::LVL_TRENCH.sprite(0),
        Task::Grove => sprites::LVL_TRENCH.sprite(0),
        Task::Drift => sprites::LVL_CONVOY.sprite(0),
        Task::Thaw => sprites::LVL_ARMOR.sprite(0),
        Task::Hearth => sprites::LVL_OFFICER.sprite(0),
    }
}

#[allow(dead_code)]
pub(crate) fn mission_help(level: u8) -> &'static Sprite {
    match task_of(level) {
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
        Task::Fetch if haul_card(level) => sprites::HELP_HAUL.sprite(0),
        Task::Fetch => sprites::HELP_LIFT.sprite(0),
        Task::Swarm if defend_card(level) => sprites::HELP_CROWS.sprite(0),
        Task::Swarm => sprites::HELP_SWARM.sprite(0),
        Task::Cover => sprites::HELP_COVER.sprite(0),
        Task::Armor => sprites::HELP_ARMOR.sprite(0),
        Task::Pill => sprites::HELP_PILL.sprite(0),
        _ => sprites::HELP_QUAD.sprite(0),
    }
}

pub(crate) fn draw_pause(frame: &mut agb::display::GraphicsFrame, menu: u8) {
    let resume = if menu == 0 {
        sprites::PAUSE_RESUME_ON.sprite(0)
    } else {
        sprites::PAUSE_RESUME.sprite(0)
    };
    let exit = if menu == 1 {
        sprites::PAUSE_EXIT_ON.sprite(0)
    } else {
        sprites::PAUSE_EXIT.sprite(0)
    };
    show(frame, resume, 88, 48);
    show(frame, exit, 88, 88);
}

fn glyph(ch: u8) -> usize {
    match ch {
        b'A'..=b'Z' => 1 + (ch - b'A') as usize,
        b'0'..=b'9' => 27 + (ch - b'0') as usize,
        b'a' => 37,
        b'e' => 38,
        b'g' => 39,
        b'm' => 40,
        b's' => 41,
        _ => 0,
    }
}

/// A role, smaller than the name under it. Same index order as the name face.
pub(crate) fn show_role(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 3 } else { 5 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 3 } else { 5 };
        if ch != b' ' && (-8..HEIGHT).contains(&y) && (-8..WIDTH).contains(&x) {
            show(frame, sprites::ROLE.sprite(glyph(ch)), x, y);
        }
        x += adv;
    }
}

/// One small line, centred. Every letter is the same 5x7 on the same baseline.
pub(crate) fn show_small(frame: &mut agb::display::GraphicsFrame, text: &[u8], y: i32) {
    let mut width = 0;
    for &ch in text {
        width += if ch == b' ' { 4 } else { 6 };
    }
    let mut x = (WIDTH - width) / 2;
    for &ch in text {
        let adv = if ch == b' ' { 4 } else { 6 };
        if ch != b' ' && (-8..HEIGHT).contains(&y) && (-8..WIDTH).contains(&x) {
            show(frame, sprites::SMALL.sprite(glyph(ch)), x, y);
        }
        x += adv;
    }
}

/// Title card. The name is drawn first, so it sits on the photograph.
pub(crate) fn draw_studio(frame: &mut agb::display::GraphicsFrame, age: i32) {
    show_face(frame, b"DRONE", 0);
    if (age / 28).rem_euclid(2) == 0 {
        show_face(frame, b"PRESS START", 124);
    }
    show_small(frame, b"MARBLE ZOO", 144);
    show_small(frame, b"2026", 152);
    for index in 0..12 {
        show(
            frame,
            sprites::TITLE.sprite(index as usize),
            (index % 4) * 64,
            (index / 4) * 64,
        );
    }
}

/// Quads and one wing, seen from above, drifting behind the roll.
fn draw_credit_craft(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let lanes: [(i32, i32, i32, bool); 4] = [
        (10, 12, 36, false),
        (120, 48, 24, true),
        (40, 96, 44, false),
        (180, 132, 30, false),
    ];
    for (shift, y, speed, wing) in lanes {
        let x = (age * speed / 40 + shift) % 300 - 40;
        let bob = ((age / 20 + shift) & 3) - 1;
        if wing {
            show(frame, sprites::GERAN.sprite(1), x, y + bob);
        } else {
            show(frame, sprites::FOE.sprite(1), x, y + bob);
        }
    }
}

struct Person {
    first: &'static [u8],
    last: &'static [u8],
}

struct Credit {
    role: &'static [u8],
    who: &'static [&'static Person],
}

static ANDREI: Person = Person {
    first: b"ANDREI",
    last: b"KSIANZOU",
};
static ALINA: Person = Person {
    first: b"ALINA",
    last: b"VELIHAYEVA",
};
static VITALI: Person = Person {
    first: b"VITALI",
    last: b"KSIANZOU",
};
static ALEX: Person = Person {
    first: b"ALEX",
    last: b"KORSHUNAU",
};

/// One line per post. A name lives here once, and the roll repeats it.
static STAFF: &[Credit] = &[
    Credit {
        role: b"DIRECTOR",
        who: &[&ANDREI],
    },
    Credit {
        role: b"PRODUCER",
        who: &[&VITALI],
    },
    Credit {
        role: b"GAME DESIGN",
        who: &[&ANDREI],
    },
    Credit {
        role: b"LEVEL DESIGN",
        who: &[&ANDREI],
    },
    Credit {
        role: b"MAIN PROGRAM",
        who: &[&ANDREI],
    },
    Credit {
        role: b"PROGRAM",
        who: &[&ALEX],
    },
    Credit {
        role: b"CHARACTER DESIGN",
        who: &[&ALINA],
    },
    Credit {
        role: b"GRAPHIC DESIGN",
        who: &[&ALINA],
    },
    Credit {
        role: b"MUSIC",
        who: &[&ANDREI],
    },
    Credit {
        role: b"SOUND EFFECTS",
        who: &[&ANDREI],
    },
];

/// End roll for the last sortie. Roles are the short titles of that era.
pub(crate) fn draw_credits(frame: &mut agb::display::GraphicsFrame, age: i32) {
    let mut end = 16i32;
    for post in STAFF {
        end += 8 + post.who.len() as i32 * 18 + 6;
    }
    end += 24;
    // The year rests just above the bottom.
    let scroll = (age / 4).min(32 + end);
    let base = 160 - scroll;
    show_small(frame, b"DRONE", base);
    let mut y = 16i32;
    for post in STAFF {
        show_role(frame, post.role, base + y);
        y += 8;
        for person in post.who {
            show_small(frame, person.first, base + y);
            y += 9;
            show_small(frame, person.last, base + y);
            y += 9;
        }
        y += 6;
    }
    y += 4;
    show_small(frame, b"MARBLE ZOO", base + y);
    show_small(frame, b"games", base + y + 10);
    show_small(frame, b"2026", base + y + 20);
    draw_credit_craft(frame, age);
}
