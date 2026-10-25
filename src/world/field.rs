use super::*;
use crate::backgrounds;
use crate::draw::{find_haze_tile, haze_tile};
use agb::display::tiled::AffineBackground;
use agb::fixnum::num;
use agb::input::{Button, ButtonController, Tri};
use alloc::vec;

impl super::World {
    /// Seats the sortie without writing the map. The menu fills that a few rows at a time.
    pub(crate) fn preview(bg: &mut AffineBackground, level: u8) -> Self {
        let level = level % LEVEL_COUNT;
        let task = task_of(level);
        let card = chain(level);
        let first = stage_task(level, 0);
        let mode = match task {
            Task::Fpv | Task::Nest | Task::Dart => Mode::Fpv,
            Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Mode::Guide,
            Task::Chain => card.map_or(Mode::Quad, |card| card.mode),
            _ => Mode::Quad,
        };
        let (tx, ty) = start_tile(level);
        let alt = match task {
            Task::Chain => card.map_or(12, |card| card.alt),
            // Above the bar. The lesson is to come down into the opening.
            Task::Gate => 20,
            Task::Fpv => 10,
            Task::Night | Task::Heat | Task::Armor | Task::Pill => 14,
            Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => 11,
            Task::Guide | Task::Column | Task::Rush | Task::Push => 16,
            // Already above the crows, so they are in the glass from the first frame.
            Task::Swarm if defend_card(level) => 22,
            _ => 12,
        };
        let start_x = tile_mid(tx);
        let start_y = tile_mid(ty);
        let start_alt = alt << 8;
        let fetch = matches!(first, Task::Fetch);
        let (bombs, he, mines, vog) = card.map_or(rack(task), |card| card.rack);
        let mut world = Self {
            marks: vec![dead_mark(); MAX_MARKS],
            mark_len: 0,
            view_tx: tx,
            view_ty: ty,
            spawn_row: 0,
            level,
            mode,
            x: start_x,
            y: start_y,
            alt: start_alt,
            pad_x: start_x,
            pad_y: start_y,
            pad_alt: start_alt,
            drones: match task {
                // Pairs share one blast, so six frames is two misses.
                Task::Fpv => 6,
                // Four nests, one spare. The dart keeps a single spare.
                Task::Nest => 5,
                Task::Dart => 4,
                Task::Chain => card.map_or(0, |card| card.drones),
                _ => 0,
            },
            vx: 0,
            vy: 0,
            yaw: num!(0),
            pitch: num!(0),
            bank: num!(0),
            bombs,
            he,
            mines,
            vog,
            stick_hold: 0,
            stick_key: 0,
            arm: card.map_or(opening_arm(task), |card| card.stages[0].arm),
            hull: 100,
            battery: 100,
            blackout: 0,
            age: 0,
            score: 0,
            targets: match task {
                Task::Trench => 6,
                Task::Officer => 1,
                Task::Night | Task::Heat | Task::Armor | Task::Pill | Task::Column => 3,
                Task::Gate => 1,
                Task::Fpv => 8,
                Task::Convoy => 1,
                Task::Hq => 6,
                Task::Guide => 3,
                Task::Rush => 6,
                Task::Push => 9,
                Task::Lane | Task::Flare | Task::Dart | Task::Bend | Task::Skirt => 3,
                Task::Nest => 4,
                Task::Fetch => 3,
                Task::Swarm => {
                    if defend_card(level) {
                        8
                    } else {
                        6
                    }
                }
                // Two quads, two wings, and the truck. Each has to fall.
                Task::Cover => 5,
                // Eight men still in the trenches.
                Task::Pits => 8,
                Task::Grove => 8,
                Task::Drift => 4,
                Task::Thaw => 7,
                Task::Hearth => 5,
                Task::Scout | Task::Battery => 0,
                Task::Chain => card.map_or(0, |card| stage_targets(level, &card.stages[0])),
            },
            fire_cd: 0,
            recover: 0,
            crashed: false,
            why: Why::None,
            again: false,
            falling: false,
            hover: fetch,
            cargo: false,
            caches: [dead_cache(); 6],
            guide: 0,
            salvo: 0,
            salvo_cd: 0,
            lesson: 0,
            stage: 0,
            seen: 0,
            guns_down: 0,
            cue: if chain(level).is_some() { STAGE_CUE } else { 0 },
            mark_x: 0,
            mark_y: 0,
            bloom: 0,
            spent: 0,
            boot: 0,
            sfx: 0,
            shots: [dead_shot(); MAX_SHOTS],
            mine_list: [dead_mine(); MAX_MINES],
            booms: [dead_boom(); MAX_BOOMS],
            burns: [dead_burn(); MAX_BURNS],
            trucks: [dead_truck(); MAX_TRUCKS],
            tanks: [dead_tank(); MAX_TANKS],
            bunkers: [dead_bunker(); MAX_BUNKERS],
            soldiers: [dead_soldier(); MAX_SOLDIERS],
            foes: [dead_foe(); MAX_FOES],
            husk_at: 0,
            husks: [dead_husk(); MAX_HUSKS],
            scrap: [dead_scrap(); MAX_SCRAP],
            drums: [dead_drum(); MAX_DRUMS],
            pops: [dead_pop(); MAX_POPS],
        };
        world.park(bg);
        world.view_tx = tx;
        world.view_ty = ty;
        world.seat_objective();
        world.seat_foes();
        for &(tx, ty) in card.map_or(&[][..], |card| card.drums) {
            world.try_spawn_drum(tx, ty);
        }
        world
    }

    pub(crate) fn boot_ready(&self) -> bool {
        self.boot < 0
    }

    /// Ground window first, then the units that stand on it. `rows` is the budget for this frame.
    pub(crate) fn boot_step(
        &mut self,
        bg: &mut AffineBackground,
        rows: i32,
        pump: &mut dyn FnMut(),
    ) {
        if self.boot < 0 {
            return;
        }
        // The whole ring. A gap of a few tiles shows up as a band once the
        // card lets the floor run to the edge of the glass.
        let ground = VIEW;
        let total = ground + 81;
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        let half = VIEW / 2;
        for _ in 0..rows {
            let row = self.boot as i32;
            if row >= total {
                self.boot = -1;
                return;
            }
            if row < ground {
                let y = ty - half + row;
                for x in (tx - half)..(tx + half) {
                    self.blit(x, y);
                }
            } else {
                if row == ground && matches!(task_of(self.level), Task::Convoy) {
                    self.paint(bg, 0, 4, T_FLOOR);
                    self.paint(bg, 0, 3, T_FLOOR);
                }
                let y = ty - 40 + (row - ground);
                for x in (tx - 40)..=(tx + 40) {
                    self.consider_tile(x, y);
                }
            }
            self.boot += 1;
            // Every row. A pair of rows was long enough to replay a slice of music.
            pump();
        }
        if self.boot as i32 >= total {
            self.boot = -1;
        }
    }

    pub(crate) fn gate_x(&self) -> i32 {
        tile_mid(24)
    }

    pub(crate) fn gate_y(&self) -> i32 {
        tile_mid(6)
    }

    /// South of the posts, on the way from the pad. Hover here, then drop.
    pub(crate) fn drill_mark(&self) -> (i32, i32) {
        (tile_mid(18), tile_mid(78))
    }

    pub(crate) fn seat_foes(&mut self) {
        for (index, (tx, ty)) in foe_spots(self.level).into_iter().enumerate() {
            if tx == i32::MIN {
                continue;
            }
            let x = tile_mid(tx);
            let y = tile_mid(ty);
            let wing = foe_wing(self.level, index);
            self.foes[index] = Foe {
                x,
                y,
                // Wings sit lower, so they are already in the glass when a quad is not.
                z: if wing { 8 << 8 } else { 12 << 8 },
                home_x: x,
                home_y: y,
                cool: 0,
                wing,
                heading: -1,
                alive: true,
            };
        }
    }

    pub(crate) fn seat_objective(&mut self) {
        if chain(self.level).is_some() {
            self.seat_stage();
            return;
        }
        let task = task_of(self.level);
        match task {
            Task::Trench => {
                for (index, tx) in [22, 23, 24, 25, 26, 27].into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(21),
                        dir: 80,
                        step: 0,
                        home_tx: tx,
                        home_ty: 21,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Officer => {
                self.soldiers[0] = Soldier {
                    x: tile_mid(24),
                    y: tile_mid(21),
                    dir: 80,
                    step: 0,
                    home_tx: 24,
                    home_ty: 21,
                    officer: true,
                    objective: true,
                    hp: HP_OFFICER,
                    cool: 40,
                    alive: true,
                };
            }
            Task::Night | Task::Heat => {
                // Night is the first vehicle card: three trucks in a short row.
                // Heat is the same job farther north and spread wider.
                let row = if matches!(task, Task::Heat) { 0 } else { 28 };
                let spots = if matches!(task, Task::Heat) {
                    [6, 24, 42]
                } else {
                    [16, 24, 32]
                };
                for (index, tx) in spots.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: tile_mid(tx),
                        y: tile_mid(row),
                        dir: 150,
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
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
                    };
                }
            }
            Task::Cover => {
                // Northbound between the posts. The quads sit ahead of it.
                // The run to the line is long enough for the new road speed.
                self.trucks[0] = Truck {
                    x: tile_mid(24),
                    y: tile_mid(52),
                    dir: -32,
                    vertical: true,
                    home_tx: 24,
                    home_ty: 52,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                    turn: 0,
                    turn_len: 0,
                    lane: 1,
                    pivot_x: 0,
                    pivot_y: 0,
                };
            }
            Task::Convoy => {
                // Northbound on the west road. Far enough south that the
                // faster truck still takes a while to reach the mark.
                self.trucks[0] = Truck {
                    x: tile_mid(0),
                    y: tile_mid(40),
                    dir: -32,
                    vertical: true,
                    home_tx: 0,
                    home_ty: 40,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                    turn: 0,
                    turn_len: 0,
                    lane: 1,
                    pivot_x: 0,
                    pivot_y: 0,
                };
            }
            Task::Guide | Task::Column => {
                let spots = [10, 26, 42];
                let moving = matches!(task, Task::Column);
                for (index, tx) in spots.into_iter().enumerate() {
                    let row = if moving { 64 } else { 20 };
                    self.trucks[index] = Truck {
                        x: tile_mid(tx),
                        y: tile_mid(row),
                        dir: if moving { 80 } else { 0 },
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
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
                    };
                }
            }
            Task::Armor => {
                // Still hulls. The hatch is the hard part, and a moving
                // six-pixel hole is not a shot you can set up.
                for (index, tx) in [12, 28, 44].into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: tile_mid(tx),
                        y: tile_mid(64),
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: 64,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Pill => {
                for (index, tx) in [16, 28, 40].into_iter().enumerate() {
                    self.bunkers[index] = Bunker {
                        x: tile_mid(tx),
                        y: tile_mid(21),
                        home_tx: tx,
                        home_ty: 21,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    };
                }
            }
            Task::Rush => {
                let spots = [(16, 41), (17, 40), (31, 43), (32, 42), (46, 40), (47, 41)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Push => {
                // Three clumps. A mark on the truck reaches the two men beside it.
                // The east clump is nearest the posts, the centre one is last.
                let trucks = [(14, 52), (30, 56), (46, 48)];
                let men = [(15, 52), (14, 51), (31, 56), (30, 55), (45, 48), (46, 47)];
                for (index, (tx, ty)) in trucks.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: -80,
                        vertical: true,
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
                    };
                }
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Fpv => {
                // Four pairs, far enough apart that one blast cannot clear two.
                let groups = [(16, 36), (32, 36), (16, 24), (32, 24)];
                let mut slot = 0;
                for (gx, gy) in groups {
                    for (ox, oy) in [(0, 0), (12, 0)] {
                        let px = gx * 8 + 4 + ox;
                        let py = gy * 8 + 4 + oy;
                        self.soldiers[slot] = Soldier {
                            x: px << 8,
                            y: py << 8,
                            dir: 0,
                            step: 0,
                            home_tx: px.div_euclid(8),
                            home_ty: py.div_euclid(8),
                            officer: false,
                            objective: true,
                            hp: HP_MAN,
                            cool: 40,
                            alive: true,
                        };
                        slot += 1;
                    }
                }
            }
            Task::Lane | Task::Dart | Task::Bend => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: if index == 1 { -90 } else { 90 },
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
                    };
                }
            }
            Task::Flare => {
                // North of the last guns, with room to descend once the lane
                // is behind. The circles still own the flight in.
                let spots = [(16, 10), (24, 8), (32, 10)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Nest => {
                let spots = [(22, 84), (32, 62), (28, 50), (18, 36)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Fetch => {
                for (index, (tx, ty)) in cache_spots(self.level).into_iter().enumerate() {
                    self.caches[index] = Cache {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        alive: true,
                    };
                }
                for (index, (tx, ty)) in mortar_spots(self.level).into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Skirt => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.bunkers[index] = Bunker {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    };
                }
            }
            Task::Grove => {
                // Two sites. Each pair stands inside one grenade, so the
                // pass stays above the rifles. A lone man was a hover in
                // their range.
                let men = [
                    (20, 32),
                    (21, 32),
                    (24, 34),
                    (25, 34),
                    (19, 22),
                    (20, 22),
                    (23, 24),
                    (24, 24),
                ];
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Drift => {
                self.trucks[0] = Truck {
                    x: tile_mid(0),
                    y: tile_mid(46),
                    dir: -32,
                    vertical: true,
                    home_tx: 0,
                    home_ty: 46,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                    turn: 0,
                    turn_len: 0,
                    lane: 1,
                    pivot_x: 0,
                    pivot_y: 0,
                };
                // Beside the road the truck is on, not a second sortie.
                // Two share a grenade. The third is the miss you can afford.
                let men = [(6, 36), (7, 36), (10, 34)];
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 80,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Thaw => {
                let spots = [(12, 42), (28, 36), (40, 44)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_TANK,
                        cool: 30,
                        alive: true,
                    };
                }
                // Well south of the pad, inside one grenade. The tanks stay on the islands north.
                let men = [(2, 2), (6, 2), (2, 6), (6, 6)];
                for (index, (ox, oy)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (24 * 8 + ox) << 8,
                        y: (110 * 8 + oy) << 8,
                        dir: 80,
                        step: 0,
                        home_tx: 24,
                        home_ty: 110,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Hearth => {
                self.soldiers[0] = Soldier {
                    x: tile_mid(24),
                    y: tile_mid(30),
                    dir: 80,
                    step: 0,
                    home_tx: 24,
                    home_ty: 30,
                    officer: true,
                    objective: true,
                    hp: HP_OFFICER,
                    cool: 40,
                    alive: true,
                };
                // One circle under the watch. The pair beside the officer
                // shares a grenade. The other two are their own drops.
                let men = [(23, 30), (25, 30), (20, 29), (28, 29)];
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index + 1] = Soldier {
                        x: tile_mid(tx),
                        y: tile_mid(ty),
                        dir: -80,
                        step: 0,
                        home_tx: tx,
                        home_ty: ty,
                        officer: false,
                        objective: true,
                        hp: HP_MAN,
                        cool: 40,
                        alive: true,
                    };
                }
            }
            Task::Pits => {
                // Two trench lines. Each pair shares a tile, so one grenade
                // covers it, and the sweep is four drops instead of a tour.
                let pairs = [(22, 29), (25, 29), (22, 21), (25, 21)];
                let offs = [(3, 4), (5, 4)];
                let mut index = 0;
                for (tx, ty) in pairs {
                    for (ox, oy) in offs {
                        self.soldiers[index] = Soldier {
                            x: (tx * 8 + ox) << 8,
                            y: (ty * 8 + oy) << 8,
                            dir: 80,
                            step: 0,
                            home_tx: tx,
                            home_ty: ty,
                            officer: false,
                            objective: true,
                            hp: HP_MAN,
                            cool: 40,
                            alive: true,
                        };
                        index += 1;
                    }
                }
                let wrecks = [
                    (14, 34, HUSK_TANK, false),
                    (34, 34, HUSK_TANK, true),
                    (10, 26, HUSK_TANK, false),
                    (36, 24, HUSK_TRUCK, true),
                    (12, 16, HUSK_TRUCK, false),
                    (30, 16, HUSK_TANK, false),
                    (20, 36, HUSK_TRUCK, true),
                    (38, 30, HUSK_TRUCK, false),
                ];
                for (tx, ty, kind, vertical) in wrecks {
                    let dir = if vertical { 80 } else { 40 };
                    self.leave_husk(tile_mid(tx), tile_mid(ty), dir, vertical, kind);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn hq_goal(&self, tx: i32, ty: i32) -> bool {
        let (hx, hy) = self.hq_corner();
        matches!(self.task(), Task::Hq)
            && (hx..=hx + 2).contains(&tx)
            && (hy..=hy + 1).contains(&ty)
    }

    /// A kill counts only when it is this card's objective.
    pub(crate) fn claim(&mut self, kill: Kill) {
        let Kill {
            officer,
            truck,
            goal,
            objective,
        } = kill;
        let counts = match self.task() {
            Task::Officer => officer,
            Task::Hearth | Task::Trench | Task::Pits | Task::Grove | Task::Thaw => {
                objective && !truck
            }
            Task::Night
            | Task::Heat
            | Task::Guide
            | Task::Column
            | Task::Lane
            | Task::Dart
            | Task::Bend => truck && objective,
            Task::Convoy | Task::Cover => goal,
            Task::Drift => goal || objective && !truck,
            Task::Fpv => objective,
            Task::Rush => objective && !truck,
            Task::Nest => objective && !truck,
            Task::Push => objective,
            Task::Gate
            | Task::Hq
            | Task::Armor
            | Task::Pill
            | Task::Flare
            | Task::Skirt
            | Task::Fetch
            | Task::Swarm
            | Task::Scout
            | Task::Battery
            | Task::Chain => false,
        };
        if counts && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_armor(&mut self) {
        if matches!(
            self.task(),
            Task::Armor | Task::Flare | Task::Fetch | Task::Thaw
        ) && self.targets > 0
        {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_pill(&mut self) {
        if matches!(self.task(), Task::Pill | Task::Skirt) && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    /// Hold time, not the glass, sets how fast the quad slides.
    pub(crate) fn quad_creep(&mut self, pitch_in: Tri, yaw_in: Tri) -> i32 {
        let along = -tri_value(pitch_in);
        let strafe = tri_value(yaw_in);
        let key = (along + strafe * 3) as i8;
        if key == 0 || key != self.stick_key {
            self.stick_hold = 0;
            self.stick_key = key;
        }
        if key != 0 && self.stick_hold < 200 {
            self.stick_hold += 2;
        }
        16 + self.stick_hold as i32
    }

    /// One mortar round at a time. B over a round takes it. B again drops it
    /// into the hatch.
    pub(crate) fn fetch_tick(&mut self, input: &ButtonController) {
        self.arm = ARM_MINE;
        self.mines = if self.cargo { 1 } else { 0 };
        if self.signal_loss() >= 16 || !input.is_just_pressed(Button::B) {
            return;
        }
        if self.cargo {
            // Released from the ground, the round detonates in the claws.
            if (self.alt >> 8) > 8 && self.try_shot(KIND_MINE) {
                self.cargo = false;
                self.spent = self.spent.saturating_add(1);
            }
            return;
        }
        // The claws close on the ground, not from cruise height.
        if (self.alt >> 8) > 1 {
            return;
        }
        if let Some(slot) = self
            .caches
            .iter()
            .position(|cache| cache.alive && dist2_px(self.x, self.y, cache.x, cache.y) <= 16 * 16)
        {
            self.caches[slot].alive = false;
            self.cargo = true;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn ground_at(&self, tx: i32, ty: i32) -> u8 {
        for mark in &self.marks[..self.mark_len] {
            if mark.tx == tx && mark.ty == ty {
                return mark.tile;
            }
        }
        terrain(self.level, tx, ty)
    }

    pub(crate) fn is_quiet(&self, tx: i32, ty: i32) -> bool {
        self.marks[..self.mark_len]
            .iter()
            .any(|mark| mark.quiet && mark.tx == tx && mark.ty == ty)
    }

    pub(crate) fn evict_mark(&self) -> usize {
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        let mut best = 0;
        let mut best_d = -1;
        let mut any_best = 0;
        let mut any_d = -1;
        for (index, mark) in self.marks[..self.mark_len].iter().enumerate() {
            let distance = (mark.tx - cx).abs() + (mark.ty - cy).abs();
            if !mark.quiet && distance > best_d {
                best_d = distance;
                best = index;
            }
            if distance > any_d {
                any_d = distance;
                any_best = index;
            }
        }
        if best_d >= 0 { best } else { any_best }
    }

    pub(crate) fn insert_mark(&mut self, tx: i32, ty: i32, tile: u8, quiet: bool) {
        if let Some(mark) = self.marks[..self.mark_len]
            .iter_mut()
            .find(|mark| mark.tx == tx && mark.ty == ty)
        {
            mark.tile = tile;
            mark.quiet |= quiet;
            return;
        }
        let (slot, restore) = if self.mark_len < MAX_MARKS {
            let slot = self.mark_len;
            self.mark_len += 1;
            (slot, None)
        } else {
            let slot = self.evict_mark();
            (slot, Some((self.marks[slot].tx, self.marks[slot].ty)))
        };
        self.marks[slot] = Mark {
            tx,
            ty,
            tile,
            quiet,
        };
        // The displaced scar must leave the map. Only if that cell is still
        // the one stored in the ring — a cell 128 tiles away shares the slot.
        if let Some((ox, oy)) = restore
            && (ox - self.view_tx).abs() <= SPAN
            && (oy - self.view_ty).abs() <= SPAN
        {
            self.blit(ox, oy);
        }
    }

    pub(crate) fn mark_quiet(&mut self, tx: i32, ty: i32) {
        if let Some(mark) = self.marks[..self.mark_len]
            .iter_mut()
            .find(|mark| mark.tx == tx && mark.ty == ty)
        {
            mark.quiet = true;
            return;
        }
        let tile = terrain(self.level, tx, ty);
        self.insert_mark(tx, ty, tile, true);
    }

    pub(crate) fn blit(&self, tx: i32, ty: i32) {
        poke_ground(tx, ty, self.ground_at(tx, ty));
    }

    /// Keep every ground tile in VRAM. The 64×64 loader is not what is shown.
    pub(crate) fn park(&mut self, bg: &mut AffineBackground) {
        let last = (T_FOG as u16 + 7).max(T_MINE as u16);
        for tile in (T_SKY as u16)..=last {
            bg.set_tile((tile as i32, 0), &backgrounds::TERRAIN.tiles, tile);
        }
        let haze = T_HAZE as u16;
        bg.set_tile((haze as i32, 0), &backgrounds::TERRAIN.tiles, haze);
        find_haze_tile();
    }

    /// The cell of the ring opposite the airframe. The window never reaches
    /// it, so it holds the sky's ramp. The edge of the window writes the
    /// field over it again as the aircraft comes round.
    pub(crate) fn haze_cell(&self) -> (i32, i32) {
        (self.view_tx + VIEW / 2, self.view_ty + VIEW / 2)
    }

    /// The window is thousands of tiles. `pump` keeps the music buffer full
    /// while that write runs, so a mission change does not replay a slice.
    pub(crate) fn refill(&mut self, pump: &mut dyn FnMut()) {
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        self.view_tx = tx;
        self.view_ty = ty;
        pump();
        for y in (ty - SPAN)..=(ty + SPAN) {
            for x in (tx - SPAN)..=(tx + SPAN) {
                self.blit(x, y);
            }
            // One row is already most of a frame. Two rows replayed the music.
            pump();
        }
    }

    /// Slide the window by the columns and rows the plane just entered.
    pub(crate) fn sync_ground(&mut self, pump: &mut dyn FnMut()) {
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        if (tx - self.view_tx).abs() > 8 || (ty - self.view_ty).abs() > 8 {
            self.refill(pump);
            return;
        }
        while self.view_tx != tx {
            let dir = if tx > self.view_tx { 1 } else { -1 };
            self.view_tx += dir;
            let col = self.view_tx + dir * SPAN;
            pump();
            let mut wrote = 0u32;
            for y in (self.view_ty - SPAN)..=(self.view_ty + SPAN) {
                self.blit(col, y);
                wrote += 1;
                if wrote & 7 == 0 {
                    pump();
                }
            }
            pump();
        }
        while self.view_ty != ty {
            let dir = if ty > self.view_ty { 1 } else { -1 };
            self.view_ty += dir;
            let row = self.view_ty + dir * SPAN;
            pump();
            let mut wrote = 0u32;
            for x in (self.view_tx - SPAN)..=(self.view_tx + SPAN) {
                self.blit(x, row);
                wrote += 1;
                if wrote & 7 == 0 {
                    pump();
                }
            }
            pump();
        }
        let hardware = haze_tile();
        if hardware != 0 {
            let (hx, hy) = self.haze_cell();
            poke_cell(hx, hy, hardware);
        }
    }

    pub(crate) fn occupied(&self, tx: i32, ty: i32) -> bool {
        homed(&self.soldiers, tx, ty)
            || homed(&self.trucks, tx, ty)
            || homed(&self.tanks, tx, ty)
            || homed(&self.bunkers, tx, ty)
            || homed(&self.drums, tx, ty)
    }

    pub(crate) fn try_spawn_soldier(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = free_slot(&mut self.soldiers) else {
            return;
        };
        *slot = Soldier {
            x: tile_mid(tx),
            y: tile_mid(ty),
            dir: if hash(tx, ty) & 1 == 0 { 80 } else { -80 },
            step: 0,
            home_tx: tx,
            home_ty: ty,
            officer: false,
            objective: false,
            hp: HP_MAN,
            cool: 50,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_truck(&mut self, tx: i32, ty: i32, vertical: bool) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = free_slot(&mut self.trucks) else {
            return;
        };
        *slot = Truck {
            x: tile_mid(tx),
            y: tile_mid(ty),
            dir: if hash(tx, ty) & 1 == 0 { 150 } else { -150 },
            vertical,
            home_tx: tx,
            home_ty: ty,
            goal: false,
            objective: false,
            hp: HP_TRUCK,
            cool: 40,
            alive: true,
            turn: 0,
            turn_len: 0,
            lane: 1,
            pivot_x: 0,
            pivot_y: 0,
        };
    }

    pub(crate) fn try_spawn_tank(&mut self, tx: i32, ty: i32, vertical: bool) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = free_slot(&mut self.tanks) else {
            return;
        };
        *slot = Tank {
            x: tile_mid(tx),
            y: tile_mid(ty),
            dir: if hash(tx, ty) & 1 == 0 { 120 } else { -120 },
            vertical,
            home_tx: tx,
            home_ty: ty,
            objective: false,
            hp: HP_TANK,
            cool: 30,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_bunker(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = free_slot(&mut self.bunkers) else {
            return;
        };
        *slot = Bunker {
            x: tile_mid(tx),
            y: tile_mid(ty),
            home_tx: tx,
            home_ty: ty,
            objective: false,
            hp: HP_BUNKER,
            cool: 20,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_drum(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = free_slot(&mut self.drums) else {
            return;
        };
        *slot = Drum {
            x: tile_mid(tx),
            y: tile_mid(ty),
            home_tx: tx,
            home_ty: ty,
            alive: true,
        };
    }

    pub(crate) fn consider_tile(&mut self, tx: i32, ty: i32) {
        // The pits are already manned and the armour is already dead.
        // Fetch is the caches and the three tanks. A trench rifle under the
        // hatch makes the sit impossible, and the tanks' own guns still
        // cover the way in.
        if matches!(task_of(self.level), Task::Pits | Task::Gate)
            || matches!(self.task(), Task::Fetch)
            || winter_card(self.level)
        {
            return;
        }
        // Nobody digs in beside the pad: a sortie starts out of rifle range.
        let (pad_x, pad_y) = start_tile(self.level);
        if (tx - pad_x).abs() <= 12 && (ty - pad_y).abs() <= 12 {
            return;
        }
        let seeded = terrain(self.level, tx, ty);
        if seeded == T_TRENCH && hash(tx, ty) % 6 == 1 {
            if self.ground_at(tx, ty) == T_TRENCH && !self.is_quiet(tx, ty) {
                self.try_spawn_soldier(tx, ty);
            }
        } else if seeded == T_TRENCH && hash(tx, ty) % 11 == 2 && !self.is_quiet(tx, ty) {
            self.try_spawn_bunker(tx, ty);
        } else if seeded == T_DIRT && !self.is_quiet(tx, ty) {
            // A fuel dump on the dirt shoulder of a road.
            let lx = tx.rem_euclid(REGION);
            let ly = ty.rem_euclid(REGION);
            if (lx == 2 || ly == 2) && hash(tx, ty.wrapping_add(7)) % 31 == 3 {
                self.try_spawn_drum(tx, ty);
            }
        } else if seeded == T_ROAD && !self.is_quiet(tx, ty) {
            let lx = tx.rem_euclid(REGION);
            let ly = ty.rem_euclid(REGION);
            if ly == 0 && lx == 24 && hash(tx, ty).is_multiple_of(2) {
                self.try_spawn_truck(tx, ty, false);
            } else if lx == 0 && ly == 24 && hash(tx, ty) % 2 == 1 {
                self.try_spawn_truck(tx, ty, true);
            } else if ly == 0 && lx == 8 && hash(tx, ty).is_multiple_of(3) {
                self.try_spawn_tank(tx, ty, false);
            } else if lx == 0 && ly == 8 && hash(tx, ty) % 3 == 1 {
                self.try_spawn_tank(tx, ty, true);
            }
        }
    }

    pub(crate) fn retire_far(&mut self) {
        let home = (tile_of(self.x), tile_of(self.y));
        retire(&mut self.soldiers, home);
        retire(&mut self.trucks, home);
        retire(&mut self.tanks, home);
        retire(&mut self.bunkers, home);
        retire(&mut self.drums, home);
    }

    pub(crate) fn maintain_actors(&mut self) {
        self.retire_far();
        // One row a frame: the whole neighbourhood refreshes about once a second
        // without stalling the picture.
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        let row = cy - 40 + self.spawn_row as i32;
        self.spawn_row = self.spawn_row.wrapping_add(1) % 81;
        for tx in cx - 40..=cx + 40 {
            self.consider_tile(tx, row);
        }
    }
}

/// Write one cell of the 128×128 ground window. The tileset is parked one
/// slot up, so tile 0 is hardware index 1.
fn poke_ground(tx: i32, ty: i32, tile: u8) {
    poke_cell(tx, ty, tile + 1);
}

/// A byte store to background VRAM lands in both halves of its halfword, so
/// it would also overwrite the cell beside it. The pair is read and written whole.
fn poke_cell(tx: i32, ty: i32, hardware: u8) {
    let cell = (ty.rem_euclid(VIEW) * VIEW + tx.rem_euclid(VIEW)) as usize;
    unsafe {
        let pair = (0x0600_C000 as *mut u16).add(cell / 2);
        let old = pair.read_volatile();
        let new = if cell & 1 == 0 {
            (old & 0xFF00) | u16::from(hardware)
        } else {
            (old & 0x00FF) | (u16::from(hardware) << 8)
        };
        pair.write_volatile(new);
    }
}

/// Someone of this kind was seated on the tile and is still alive.
fn homed<T: Unit>(units: &[T], tx: i32, ty: i32) -> bool {
    units
        .iter()
        .any(|unit| unit.alive() && unit.home() == (tx, ty))
}

fn free_slot<T: Unit>(units: &mut [T]) -> Option<&mut T> {
    units.iter_mut().find(|unit| !unit.alive())
}

/// Seventy tiles out, a unit the card does not need is let go.
fn retire<T: Unit>(units: &mut [T], (cx, cy): (i32, i32)) {
    for unit in units {
        if !unit.alive() || unit.kept() {
            continue;
        }
        let (x, y) = unit.at();
        if (tile_of(x) - cx).abs() > 70 || (tile_of(y) - cy).abs() > 70 {
            unit.retire();
        }
    }
}
