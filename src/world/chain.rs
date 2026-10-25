//! Cards of more than one stage. Each stage is a task an older card already
//! asks, set somewhere else on the field. Clear it and the next one opens:
//! its marks come up on the radar and a line under the glass names it.

use super::*;

/// The campaign as it is flown. A card keeps its number in the save; this is
/// only the order of the briefings. The stage cards sit between the old ones,
/// each after the card that taught what it asks.
pub(crate) const ORDER: [u8; LEVEL_COUNT as usize] = [
    0, 1, 2, 3, 4, 30, 5, 6, 31, 7, 8, 9, 10, 33, 11, 12, 32, 13, 14, 15, 16, 34, 17, 35, 18, 19,
    20, 21, 36, 22, 23, 24, 39, 25, 26, 37, 27, 28, 38, 29,
];

/// The first card number that is a stage card.
pub(crate) const FIRST_CHAIN: u8 = 30;

/// Where a card stands in the campaign.
pub(crate) fn slot_of(level: u8) -> u8 {
    ORDER
        .iter()
        .position(|&card| card == level % LEVEL_COUNT)
        .unwrap_or(0) as u8
}

/// The card at this place in the campaign.
pub(crate) fn card_at(slot: u8) -> u8 {
    ORDER[usize::from(slot.min(LEVEL_COUNT - 1))]
}

/// The last briefing of the campaign. Clearing it rolls the credits.
pub(crate) fn last_card(level: u8) -> bool {
    slot_of(level) + 1 >= LEVEL_COUNT
}

/// The old card a stage card follows. Guns, rifles and runners are as keen
/// as they were on that card.
pub(crate) fn pace(level: u8) -> u8 {
    let level = level % LEVEL_COUNT;
    if level < FIRST_CHAIN {
        return level;
    }
    let slot = usize::from(slot_of(level));
    ORDER[..slot]
        .iter()
        .rev()
        .copied()
        .find(|&card| card < FIRST_CHAIN)
        .unwrap_or(0)
}

/// How the third mark is earned on a stage card.
#[derive(Clone, Copy)]
pub(crate) enum Spare {
    /// No more than this many releases or marks.
    Drops(i16),
    /// At least this many kamikaze airframes still in hand.
    Drones(i32),
}

pub(crate) struct Stage {
    /// What this stage asks, read the way the card of that name reads it.
    pub(crate) task: Task,
    /// Where its marks stand, in tiles: the units, or the points to fly over.
    pub(crate) spots: &'static [(i32, i32)],
    /// Rounds put down for a mine stage.
    pub(crate) caches: &'static [(i32, i32)],
    /// The tile the convoy must not reach, the house's north-west corner, or
    /// the row the infantry runs for.
    pub(crate) at: (i32, i32),
    /// The charge in hand when the stage opens.
    pub(crate) arm: u8,
    /// The line under the glass while the stage is new.
    pub(crate) line: &'static [u8],
}

pub(crate) struct Chain {
    pub(crate) title: (&'static [u8], &'static [u8]),
    /// The two lines of the briefing.
    pub(crate) order: (&'static [u8], &'static [u8]),
    pub(crate) sensor: Sensor,
    pub(crate) mode: Mode,
    pub(crate) pad: (i32, i32),
    pub(crate) alt: i32,
    /// Bombs, shells, mines, grenades.
    pub(crate) rack: (i32, i32, i32, i32),
    pub(crate) drones: i32,
    /// Frames for the quick mark.
    pub(crate) budget: i32,
    pub(crate) spare: Spare,
    pub(crate) guns: &'static [AaGun],
    /// Charge pads for the mine quad.
    pub(crate) charge: &'static [(i32, i32)],
    pub(crate) winter: bool,
    /// Brown islands on a snow card: centre and radius, in tiles.
    pub(crate) mud: &'static [(i32, i32, i32)],
    /// Fuel dumps beside the work, in tiles.
    pub(crate) drums: &'static [(i32, i32)],
    pub(crate) stages: &'static [Stage],
}

const fn stage(task: Task, spots: &'static [(i32, i32)], arm: u8, line: &'static [u8]) -> Stage {
    Stage {
        task,
        spots,
        caches: &[],
        at: (0, 0),
        arm,
        line,
    }
}

const fn gun(tx: i32, ty: i32, radius: i32) -> AaGun {
    AaGun { tx, ty, radius }
}

/// Stage cards, by number from `FIRST_CHAIN`.
pub(crate) static CHAINS: [Chain; 10] = [
    // 31 on the card: the convoy's truck, then the house it was driving for.
    Chain {
        title: (b"CUT", b"OFF"),
        order: (b"1 STOP THE TRUCK", b"2 THEN ITS HOUSE"),
        sensor: Sensor::Day,
        mode: Mode::Quad,
        pad: (8, 48),
        alt: 12,
        rack: (3, 4, 0, 0),
        drones: 0,
        budget: 4200,
        spare: Spare::Drops(4),
        guns: &[],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[(49, 11), (55, 6)],
        stages: &[
            Stage {
                at: (0, 4),
                ..stage(Task::Convoy, &[(0, 40)], ARM_BOMB, b"1 STOP THE TRUCK")
            },
            Stage {
                at: (50, 8),
                ..stage(Task::Hq, &[], ARM_HE, b"2 BREAK THE HOUSE")
            },
        ],
    },
    // Low over three marks, then the trucks they found.
    Chain {
        title: (b"NIGHT", b"SCOUT"),
        order: (b"1 LOW OVER 3 MARKS", b"2 THEN THREE TRUCKS"),
        sensor: Sensor::Night,
        mode: Mode::Quad,
        pad: (24, 90),
        alt: 14,
        rack: (6, 0, 0, 0),
        drones: 0,
        budget: 4400,
        spare: Spare::Drops(4),
        guns: &[],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[(56, 33)],
        stages: &[
            stage(
                Task::Scout,
                &[(12, 72), (34, 64), (20, 54)],
                ARM_BOMB,
                b"1 FLY LOW OVER MARKS",
            ),
            stage(
                Task::Night,
                &[(44, 30), (52, 30), (60, 30)],
                ARM_BOMB,
                b"2 BOMB THREE TRUCKS",
            ),
        ],
    },
    // Three guns over a tank park. From above 20 they cannot see the drop.
    Chain {
        title: (b"SILENT", b"GUNS"),
        order: (b"1 GUNS FROM ABOVE 20", b"2 THEN TWO TANKS"),
        sensor: Sensor::Day,
        mode: Mode::Quad,
        pad: (24, 92),
        alt: 12,
        rack: (6, 0, 4, 0),
        drones: 0,
        budget: 4200,
        spare: Spare::Drops(8),
        guns: &[gun(14, 62, 96), gun(34, 62, 96), gun(24, 42, 120)],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[(24, 27)],
        stages: &[
            stage(Task::Battery, &[], ARM_BOMB, b"1 BOMB THE GUNS"),
            stage(
                Task::Armor,
                &[(18, 30), (30, 30)],
                ARM_MINE,
                b"2 MINES IN THE HATCH",
            ),
        ],
    },
    // Heat: three marks, then the bunkers past them.
    Chain {
        title: (b"HOT", b"TRAIL"),
        order: (b"1 LOW OVER 3 MARKS", b"2 THEN THREE BUNKERS"),
        sensor: Sensor::Heat,
        mode: Mode::Quad,
        pad: (24, 80),
        alt: 14,
        rack: (0, 6, 0, 0),
        drones: 0,
        budget: 4400,
        spare: Spare::Drops(6),
        guns: &[],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[(19, 24)],
        stages: &[
            stage(
                Task::Scout,
                &[(12, 66), (36, 62), (24, 48)],
                ARM_HE,
                b"1 FLY LOW OVER MARKS",
            ),
            stage(
                Task::Pill,
                &[(12, 21), (26, 21), (38, 21)],
                ARM_HE,
                b"2 SHELL THREE BUNKERS",
            ),
        ],
    },
    // Kamikaze at night: the two guns first, then the trucks behind them.
    Chain {
        title: (b"DARK", b"PASS"),
        order: (b"1 RAM BOTH GUNS", b"2 THEN THREE TRUCKS"),
        sensor: Sensor::Night,
        mode: Mode::Fpv,
        pad: (24, 100),
        alt: 10,
        rack: (0, 0, 0, 0),
        drones: 7,
        budget: 4200,
        spare: Spare::Drones(2),
        guns: &[gun(14, 66, 96), gun(34, 66, 96)],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[],
        stages: &[
            stage(Task::Battery, &[], ARM_GUN, b"1 RAM BOTH GUNS"),
            stage(
                Task::Dart,
                &[(14, 40), (24, 38), (32, 40)],
                ARM_GUN,
                b"2 RAM THREE TRUCKS",
            ),
        ],
    },
    // The spotter: parked trucks, then the crews running from the wrecks.
    Chain {
        title: (b"LAST", b"STOP"),
        order: (b"1 MARK THREE TRUCKS", b"2 THEN THE CREWS"),
        sensor: Sensor::Ir,
        mode: Mode::Guide,
        pad: (24, 112),
        alt: 16,
        rack: (0, 0, 0, 0),
        drones: 0,
        budget: 6000,
        spare: Spare::Drops(7),
        guns: &[],
        charge: &[],
        winter: false,
        mud: &[],
        drums: &[],
        stages: &[
            stage(
                Task::Guide,
                &[(10, 74), (26, 74), (42, 74)],
                ARM_BOMB,
                b"1 MARK THREE TRUCKS",
            ),
            Stage {
                at: (0, 46),
                ..stage(
                    Task::Rush,
                    &[(16, 73), (17, 72), (31, 75), (32, 74), (46, 72), (47, 73)],
                    ARM_BOMB,
                    b"2 STOP THE CREWS",
                )
            },
        ],
    },
    // The mine quad: one tank near the pad, then two past a pair of guns.
    Chain {
        title: (b"MINE", b"RUN"),
        order: (b"1 ONE TANK NEAR", b"2 TWO PAST THE GUNS"),
        sensor: Sensor::Day,
        mode: Mode::Quad,
        pad: (24, 96),
        alt: 12,
        rack: (0, 0, 0, 0),
        drones: 0,
        budget: 6400,
        spare: Spare::Drops(5),
        guns: &[gun(12, 54, 96), gun(36, 54, 96)],
        charge: &[(24, 84), (24, 44)],
        winter: false,
        mud: &[],
        drums: &[],
        stages: &[
            Stage {
                caches: &[(20, 104), (24, 104), (28, 104)],
                ..stage(Task::Fetch, &[(8, 76)], ARM_MINE, b"1 MINE INTO THE TANK")
            },
            Stage {
                caches: &[(20, 70), (28, 70), (24, 74)],
                ..stage(
                    Task::Fetch,
                    &[(16, 26), (32, 24)],
                    ARM_MINE,
                    b"2 TWO PAST THE GUNS",
                )
            },
        ],
    },
    // Snow: three marks, then two pairs on the mud.
    Chain {
        title: (b"WHITE", b"SEARCH"),
        order: (b"1 LOW OVER 3 MARKS", b"2 THEN TWO PAIRS"),
        sensor: Sensor::Winter,
        mode: Mode::Quad,
        pad: (24, 70),
        alt: 12,
        rack: (2, 0, 0, 6),
        drones: 0,
        budget: 4000,
        spare: Spare::Drops(4),
        guns: &[],
        charge: &[],
        winter: true,
        mud: &[(41, 30, 3), (47, 22, 3)],
        drums: &[],
        stages: &[
            stage(
                Task::Scout,
                &[(14, 56), (34, 54), (22, 44)],
                ARM_VOG,
                b"1 FLY LOW OVER MARKS",
            ),
            stage(
                Task::Grove,
                &[(40, 30), (41, 30), (46, 22), (47, 22)],
                ARM_VOG,
                b"2 GRENADE TWO PAIRS",
            ),
        ],
    },
    // Snow road: the truck, then the tanks waiting for it on the mud.
    Chain {
        title: (b"COLD", b"CONVOY"),
        order: (b"1 STOP THE TRUCK", b"2 THEN TWO TANKS"),
        sensor: Sensor::Winter,
        mode: Mode::Quad,
        pad: (14, 62),
        alt: 12,
        rack: (4, 0, 4, 0),
        drones: 0,
        budget: 4400,
        spare: Spare::Drops(6),
        guns: &[],
        charge: &[],
        winter: true,
        mud: &[(40, 22, 3), (50, 28, 3)],
        drums: &[(44, 24)],
        stages: &[
            Stage {
                at: (0, 4),
                ..stage(Task::Convoy, &[(0, 46)], ARM_BOMB, b"1 STOP THE TRUCK")
            },
            stage(
                Task::Armor,
                &[(40, 22), (50, 28)],
                ARM_MINE,
                b"2 MINES IN THE HATCH",
            ),
        ],
    },
    // The mine quad at night: two tanks east, then one behind the guns.
    Chain {
        title: (b"NIGHT", b"COURIER"),
        order: (b"1 TWO TANKS EAST", b"2 ONE PAST THE GUNS"),
        sensor: Sensor::Night,
        mode: Mode::Quad,
        pad: (24, 96),
        alt: 12,
        rack: (0, 0, 0, 0),
        drones: 0,
        budget: 6400,
        spare: Spare::Drops(5),
        guns: &[gun(16, 44, 96), gun(32, 44, 96)],
        charge: &[(40, 82), (24, 66)],
        winter: false,
        mud: &[],
        drums: &[],
        stages: &[
            Stage {
                caches: &[(20, 104), (24, 104), (28, 104), (24, 108)],
                ..stage(
                    Task::Fetch,
                    &[(42, 72), (46, 62)],
                    ARM_MINE,
                    b"1 TWO TANKS EAST",
                )
            },
            Stage {
                caches: &[(20, 62), (28, 62)],
                ..stage(Task::Fetch, &[(24, 24)], ARM_MINE, b"2 ONE PAST THE GUNS")
            },
        ],
    },
];

/// The stage card with this number, if it is one.
pub(crate) fn chain(level: u8) -> Option<&'static Chain> {
    let level = level % LEVEL_COUNT;
    if level < FIRST_CHAIN {
        return None;
    }
    CHAINS.get(usize::from(level - FIRST_CHAIN))
}

/// The task a stage asks. An ordinary card has the one.
pub(crate) fn stage_task(level: u8, stage: u8) -> Task {
    match chain(level) {
        Some(card) => card.stages[usize::from(stage).min(card.stages.len() - 1)].task,
        None => task_of(level),
    }
}

/// How many marks a stage opens with.
pub(crate) fn stage_targets(level: u8, stage: &Stage) -> i32 {
    match stage.task {
        // Six roof tiles, as on the house card.
        Task::Hq => 6,
        Task::Battery => aa_guns(level).len() as i32,
        // The goal truck alone.
        Task::Convoy => 1,
        _ => stage.spots.len() as i32,
    }
}

/// A scout mark counts once the drone passes this close, this low.
pub(crate) const SCOUT_REACH: i32 = 24;
pub(crate) const SCOUT_ALT: i32 = 14;

/// Frames a stage's line stays under the glass once it opens.
pub(crate) const STAGE_CUE: i16 = 180;

impl World {
    /// The task in hand: this stage's on a stage card, the card's own otherwise.
    pub(crate) fn task(&self) -> Task {
        stage_task(self.level, self.stage)
    }

    pub(crate) fn stage_now(&self) -> Option<&'static Stage> {
        chain(self.level)
            .map(|card| &card.stages[usize::from(self.stage).min(card.stages.len() - 1)])
    }

    /// The tile the convoy truck must not reach.
    pub(crate) fn convoy_line(&self) -> (i32, i32) {
        self.stage_now().map_or((0, 4), |stage| stage.at)
    }

    /// The north-west roof tile of the house. The roof is three by two.
    pub(crate) fn hq_corner(&self) -> (i32, i32) {
        self.stage_now().map_or((22, 16), |stage| stage.at)
    }

    /// The row a running man or a pushing truck must not reach.
    pub(crate) fn rush_line(&self) -> i32 {
        self.stage_now().map_or(14, |stage| stage.at.1)
    }

    /// This battery still stands.
    pub(crate) fn gun_up(&self, index: usize) -> bool {
        self.guns_down & (1 << index) == 0
    }

    /// A standing battery's circle holds this point.
    pub(crate) fn guns_cover(&self, x: i32, y: i32) -> bool {
        aa_guns(self.level).iter().enumerate().any(|(index, gun)| {
            let (gx, gy) = gun_at(gun);
            self.gun_up(index) && dist2_px(x, y, gx, gy) <= gun.radius * gun.radius
        })
    }

    /// The scout marks still to fly over.
    pub(crate) fn scout_marks(&self) -> impl Iterator<Item = (usize, (i32, i32))> + '_ {
        let spots = match self.stage_now() {
            Some(stage) if matches!(stage.task, Task::Scout) => stage.spots,
            _ => &[],
        };
        spots
            .iter()
            .enumerate()
            .filter(|(index, _)| self.seen & (1 << index) == 0)
            .map(|(index, &(tx, ty))| (index, (tile_mid(tx), tile_mid(ty))))
    }

    /// Seats the marks of the stage in hand. The units of a cleared stage
    /// are wrecks by now, so their slots are free; a stranger loafing in one
    /// is let go first.
    pub(crate) fn seat_stage(&mut self) {
        let Some(stage) = self.stage_now() else {
            return;
        };
        let task = stage.task;
        for (index, &(tx, ty)) in stage.spots.iter().enumerate() {
            let (x, y) = (tile_mid(tx), tile_mid(ty));
            match task {
                Task::Convoy => place(
                    &mut self.trucks,
                    Truck {
                        dir: -32,
                        vertical: true,
                        goal: true,
                        cool: 20,
                        ..seat_truck(tx, ty)
                    },
                ),
                Task::Night | Task::Heat => place(
                    &mut self.trucks,
                    Truck {
                        dir: 150,
                        ..seat_truck(tx, ty)
                    },
                ),
                Task::Lane | Task::Dart | Task::Bend => place(
                    &mut self.trucks,
                    Truck {
                        dir: if index == 1 { -90 } else { 90 },
                        ..seat_truck(tx, ty)
                    },
                ),
                Task::Guide => place(&mut self.trucks, seat_truck(tx, ty)),
                Task::Armor | Task::Flare | Task::Thaw | Task::Fetch => place(
                    &mut self.tanks,
                    Tank {
                        x,
                        y,
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    },
                ),
                Task::Pill | Task::Skirt => place(
                    &mut self.bunkers,
                    Bunker {
                        x,
                        y,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    },
                ),
                Task::Rush | Task::Grove | Task::Pits | Task::Trench => place(
                    &mut self.soldiers,
                    Soldier {
                        x,
                        y,
                        dir: if matches!(task, Task::Rush) { 0 } else { 80 },
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    },
                ),
                _ => {}
            }
        }
        for &(tx, ty) in stage.caches {
            if let Some(slot) = self.caches.iter_mut().find(|cache| !cache.alive) {
                *slot = Cache {
                    x: tile_mid(tx),
                    y: tile_mid(ty),
                    alive: true,
                };
            }
        }
        // The convoy's line is a pale stretch of road between two posts.
        if matches!(task, Task::Convoy) {
            let (lx, ly) = stage.at;
            self.insert_mark(lx, ly, T_FLOOR, false);
            self.insert_mark(lx, ly - 1, T_FLOOR, false);
        }
    }

    /// A low pass over the next scout mark photographs it.
    pub(crate) fn check_scout(&mut self) {
        if !matches!(self.task(), Task::Scout) || self.targets <= 0 || self.falling {
            return;
        }
        if (self.alt >> 8) > SCOUT_ALT {
            return;
        }
        let reach = SCOUT_REACH * SCOUT_REACH;
        let over = self
            .scout_marks()
            .find(|&(_, (x, y))| dist2_px(self.x, self.y, x, y) <= reach)
            .map(|(index, _)| index);
        if let Some(index) = over {
            self.seen |= 1 << index;
            self.targets -= 1;
            self.score += 50;
            self.sfx |= SFX_MARK;
        }
    }

    /// A blast on a battery takes it out. Its circle goes dark with it.
    pub(crate) fn hit_guns(&mut self, x: i32, y: i32, radius: i32) {
        for (index, gun) in aa_guns(self.level).iter().enumerate() {
            if !self.gun_up(index) || chain(self.level).is_none() {
                continue;
            }
            let (gx, gy) = gun_at(gun);
            if dist2_px(x, y, gx, gy) > radius * radius {
                continue;
            }
            self.guns_down |= 1 << index;
            self.leave_husk(gx, gy, 0, false, HUSK_BUNKER);
            self.score += 100;
            if matches!(self.task(), Task::Battery) && self.targets > 0 {
                self.targets -= 1;
                self.sfx |= SFX_MARK;
            }
        }
    }

    /// The stage is clear and another follows: seat it and name it.
    pub(crate) fn next_stage(&mut self) {
        let Some(card) = chain(self.level) else {
            return;
        };
        if self.targets > 0 || self.crashed || usize::from(self.stage) + 1 >= card.stages.len() {
            return;
        }
        self.stage += 1;
        self.seen = 0;
        let stage = &card.stages[usize::from(self.stage)];
        self.targets = stage_targets(self.level, stage);
        self.seat_stage();
        // A battery that is already down does not count again.
        if matches!(stage.task, Task::Battery) {
            self.targets -= self.guns_down.count_ones() as i32;
        }
        if self.mode == Mode::Quad && !matches!(stage.task, Task::Fetch) {
            self.arm = stage.arm;
        }
        self.cue = STAGE_CUE;
        self.score += 100;
        self.sfx |= SFX_MARK;
    }
}

/// A truck of the stage, parked on its tile until the stage says otherwise.
fn seat_truck(tx: i32, ty: i32) -> Truck {
    Truck {
        x: tile_mid(tx),
        y: tile_mid(ty),
        dir: 0,
        vertical: false,
        home_tx: tx,
        home_ty: ty,
        goal: false,
        objective: true,
        hp: HP_TRUCK,
        cool: 30,
        alive: true,
        turn: 0,
        turn_len: 0,
        lane: 1,
        pivot_x: 0,
        pivot_y: 0,
    }
}

/// The first empty slot, or failing that one the card does not need.
fn place<T: Unit>(units: &mut [T], unit: T) {
    let slot = units
        .iter()
        .position(|held| !held.alive())
        .or_else(|| units.iter().position(|held| !held.kept()));
    if let Some(slot) = slot {
        units[slot] = unit;
    }
}
