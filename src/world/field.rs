use super::*;
use agb::display::tiled::AffineBackground;
use agb::fixnum::num;
use agb::input::{Button, ButtonController, Tri};
use alloc::vec;
use crate::backgrounds;

impl super::World {
    /// Seats the sortie without writing the map. The menu fills that a few rows at a time.
    pub(crate) fn preview(bg: &mut AffineBackground, level: u8) -> Self {
        let level = level % LEVEL_COUNT;
        let task = task_of(level);
        let mode = match task {
            Task::Fpv | Task::Nest | Task::Dart => Mode::Fpv,
            Task::Gate => Mode::Gate,
            Task::Guide | Task::Column | Task::Rush | Task::Push | Task::Bend => Mode::Guide,
            _ => Mode::Quad,
        };
        let (tx, ty) = start_tile(level);
        let alt = match task {
            Task::Gate => 12,
            Task::Fpv => 10,
            Task::Night | Task::Heat | Task::Armor | Task::Pill => 14,
            Task::Lane | Task::Flare | Task::Nest | Task::Dart | Task::Bend | Task::Skirt => 11,
            Task::Guide | Task::Column | Task::Rush | Task::Push => 16,
            // Already above the crows, so they are in the glass from the first frame.
            Task::Swarm if defend_card(level) => 22,
            _ => 12,
        };
        let start_x = (tx * 8 + 4) << 8;
        let start_y = (ty * 8 + 4) << 8;
        let start_alt = alt << 8;
        let fetch = matches!(task, Task::Fetch);
        let bare = mode == Mode::Fpv || mode == Mode::Guide || fetch;
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
                _ => 0,
            },
            vx: 0,
            vy: 0,
            yaw: num!(0),
            pitch: num!(0),
            bank: num!(0),
            bombs: if bare { 0 } else { 40 },
            he: if bare { 0 } else { 8 },
            mines: if bare { 0 } else { 8 },
            vog: if bare { 0 } else { 16 },
            stick_hold: 0,
            stick_key: 0,
            arm: if mode == Mode::Fpv || matches!(task, Task::Swarm | Task::Cover) {
                ARM_GUN
            } else if fetch {
                ARM_MINE
            } else {
                ARM_BOMB
            },
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
        };
        world.park(bg);
        world.view_tx = tx;
        world.view_ty = ty;
        world.seat_objective();
        world.seat_foes();
        world
    }

    pub(crate) fn boot_ready(&self) -> bool {
        self.boot < 0
    }

    /// Ground window first, then the units that stand on it. `rows` is the budget for this frame.
    pub(crate) fn boot_step(&mut self, bg: &mut AffineBackground, rows: i32, pump: &mut dyn FnMut()) {
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
            if self.boot & 1 == 0 {
                pump();
            }
        }
        if self.boot as i32 >= total {
            self.boot = -1;
        }
    }

    pub(crate) fn gate_x(&self) -> i32 {
        (24 * 8 + 4) << 8
    }

    pub(crate) fn gate_y(&self) -> i32 {
        (6 * 8 + 4) << 8
    }

    pub(crate) fn seat_foes(&mut self) {
        for (index, (tx, ty)) in foe_spots(self.level).into_iter().enumerate() {
            if tx == i32::MIN {
                continue;
            }
            let x = (tx * 8 + 4) << 8;
            let y = (ty * 8 + 4) << 8;
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
        let task = task_of(self.level);
        match task {
            Task::Trench => {
                for (index, tx) in [22, 23, 24, 25, 26, 27].into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (21 * 8 + 4) << 8,
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
                    x: (24 * 8 + 4) << 8,
                    y: (21 * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (row * 8 + 4) << 8,
                        dir: 150,
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Cover => {
                // Northbound between the posts. The quads ride above this truck.
                self.trucks[0] = Truck {
                    x: (24 * 8 + 4) << 8,
                    y: (40 * 8 + 4) << 8,
                    dir: -32,
                    vertical: true,
                    home_tx: 24,
                    home_ty: 40,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                };
            }
            Task::Convoy => {
                // Northbound on the west road, toward the pale mark.
                self.trucks[0] = Truck {
                    x: (0 * 8 + 4) << 8,
                    y: (14 * 8 + 4) << 8,
                    dir: -32,
                    vertical: true,
                    home_tx: 0,
                    home_ty: 14,
                    goal: true,
                    objective: true,
                    hp: HP_TRUCK,
                    cool: 20,
                    alive: true,
                };
            }
            Task::Guide | Task::Column => {
                let spots = [10, 26, 42];
                let moving = matches!(task, Task::Column);
                for (index, tx) in spots.into_iter().enumerate() {
                    let row = if moving { 64 } else { 20 };
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (row * 8 + 4) << 8,
                        dir: if moving { 80 } else { 0 },
                        vertical: false,
                        home_tx: tx,
                        home_ty: row,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Armor => {
                for (index, tx) in [12, 28, 44].into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (64 * 8 + 4) << 8,
                        dir: 80,
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
                        x: (tx * 8 + 4) << 8,
                        y: (21 * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
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
                let men = [
                    (15, 52),
                    (14, 51),
                    (31, 56),
                    (30, 55),
                    (45, 48),
                    (46, 47),
                ];
                for (index, (tx, ty)) in trucks.into_iter().enumerate() {
                    self.trucks[index] = Truck {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: -80,
                        vertical: true,
                        home_tx: tx,
                        home_ty: ty,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
                for (index, (tx, ty)) in men.into_iter().enumerate() {
                    self.soldiers[index] = Soldier {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        dir: 0,
                        vertical: false,
                        home_tx: tx,
                        home_ty: ty,
                        goal: false,
                        objective: true,
                        hp: HP_TRUCK,
                        cool: 30,
                        alive: true,
                    };
                }
            }
            Task::Flare => {
                let spots = [(14, 24), (24, 22), (32, 24)];
                for (index, (tx, ty)) in spots.into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        alive: true,
                    };
                }
                for (index, (tx, ty)) in mortar_spots(self.level).into_iter().enumerate() {
                    self.tanks[index] = Tank {
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
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
                        x: (tx * 8 + 4) << 8,
                        y: (ty * 8 + 4) << 8,
                        home_tx: tx,
                        home_ty: ty,
                        objective: true,
                        hp: HP_BUNKER,
                        cool: 20,
                        alive: true,
                    };
                }
            }
            _ => {}
        }
    }

    pub(crate) fn hq_goal(&self, tx: i32, ty: i32) -> bool {
        matches!(task_of(self.level), Task::Hq) && (22..=24).contains(&tx) && (16..=17).contains(&ty)
    }

    /// A kill counts only when it is this card's objective.
    pub(crate) fn claim(&mut self, officer: bool, truck: bool, goal: bool, objective: bool) {
        let counts = match task_of(self.level) {
            Task::Officer => officer,
            Task::Trench => objective && !truck,
            Task::Night | Task::Heat | Task::Guide | Task::Column | Task::Lane | Task::Dart
            | Task::Bend => truck && objective,
            Task::Convoy | Task::Cover => goal,
            Task::Fpv => objective,
            Task::Rush => objective && !truck,
            Task::Nest => objective && !truck,
            Task::Push => objective,
            Task::Gate | Task::Hq | Task::Armor | Task::Pill | Task::Flare | Task::Skirt
            | Task::Fetch | Task::Swarm => false,
        };
        if counts && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_armor(&mut self) {
        if matches!(task_of(self.level), Task::Armor | Task::Flare | Task::Fetch)
            && self.targets > 0
        {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    pub(crate) fn claim_pill(&mut self) {
        if matches!(task_of(self.level), Task::Pill | Task::Skirt) && self.targets > 0 {
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
        if let Some((ox, oy)) = restore {
            if (ox - self.view_tx).abs() <= SPAN && (oy - self.view_ty).abs() <= SPAN {
                self.blit(ox, oy);
            }
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
        let hx = tx.rem_euclid(VIEW) as usize;
        let hy = ty.rem_euclid(VIEW) as usize;
        // Park loaded tileset slot 0 into hardware index 1, and so on.
        let hw = self.ground_at(tx, ty) + 1;
        unsafe {
            (0x0600_C000 as *mut u8)
                .add(hy * 128 + hx)
                .write_volatile(hw);
        }
    }

    /// Keep every ground tile in VRAM. The 64×64 loader is not what is shown.
    pub(crate) fn park(&mut self, bg: &mut AffineBackground) {
        let last = (T_FOG as u16 + 7).max(T_MINE as u16);
        for tile in (T_SKY as u16)..=last {
            bg.set_tile((tile as i32, 0), &backgrounds::TERRAIN.tiles, tile);
        }
    }

    /// The window is thousands of tiles. `pump` keeps the music buffer full
    /// while that write runs, so a mission change does not replay a slice.
    pub(crate) fn refill(&mut self, pump: &mut dyn FnMut()) {
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        self.view_tx = tx;
        self.view_ty = ty;
        pump();
        let mut rows = 0u32;
        for y in (ty - SPAN)..=(ty + SPAN) {
            for x in (tx - SPAN)..=(tx + SPAN) {
                self.blit(x, y);
            }
            rows += 1;
            if rows & 1 == 0 {
                pump();
            }
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
            for y in (self.view_ty - SPAN)..=(self.view_ty + SPAN) {
                self.blit(col, y);
            }
        }
        while self.view_ty != ty {
            let dir = if ty > self.view_ty { 1 } else { -1 };
            self.view_ty += dir;
            let row = self.view_ty + dir * SPAN;
            for x in (self.view_tx - SPAN)..=(self.view_tx + SPAN) {
                self.blit(x, row);
            }
        }
    }

    pub(crate) fn occupied(&self, tx: i32, ty: i32) -> bool {
        self.soldiers
            .iter()
            .any(|soldier| soldier.alive && soldier.home_tx == tx && soldier.home_ty == ty)
            || self
                .trucks
                .iter()
                .any(|truck| truck.alive && truck.home_tx == tx && truck.home_ty == ty)
            || self
                .tanks
                .iter()
                .any(|tank| tank.alive && tank.home_tx == tx && tank.home_ty == ty)
            || self
                .bunkers
                .iter()
                .any(|bunker| bunker.alive && bunker.home_tx == tx && bunker.home_ty == ty)
    }

    pub(crate) fn try_spawn_soldier(&mut self, tx: i32, ty: i32) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.soldiers.iter_mut().find(|soldier| !soldier.alive) else {
            return;
        };
        *slot = Soldier {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
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
        let Some(slot) = self.trucks.iter_mut().find(|truck| !truck.alive) else {
            return;
        };
        *slot = Truck {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            dir: if hash(tx, ty) & 1 == 0 { 150 } else { -150 },
            vertical,
            home_tx: tx,
            home_ty: ty,
            goal: false,
            objective: false,
            hp: HP_TRUCK,
            cool: 40,
            alive: true,
        };
    }

    pub(crate) fn try_spawn_tank(&mut self, tx: i32, ty: i32, vertical: bool) {
        if self.occupied(tx, ty) {
            return;
        }
        let Some(slot) = self.tanks.iter_mut().find(|tank| !tank.alive) else {
            return;
        };
        *slot = Tank {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
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
        let Some(slot) = self.bunkers.iter_mut().find(|bunker| !bunker.alive) else {
            return;
        };
        *slot = Bunker {
            x: (tx * 8 + 4) << 8,
            y: (ty * 8 + 4) << 8,
            home_tx: tx,
            home_ty: ty,
            objective: false,
            hp: HP_BUNKER,
            cool: 20,
            alive: true,
        };
    }

    pub(crate) fn consider_tile(&mut self, tx: i32, ty: i32) {
        let seeded = terrain(self.level, tx, ty);
        if seeded == T_TRENCH && hash(tx, ty) % 6 == 1 {
            if self.ground_at(tx, ty) == T_TRENCH && !self.is_quiet(tx, ty) {
                self.try_spawn_soldier(tx, ty);
            }
        } else if seeded == T_TRENCH && hash(tx, ty) % 11 == 2 && !self.is_quiet(tx, ty) {
            self.try_spawn_bunker(tx, ty);
        } else if seeded == T_ROAD && !self.is_quiet(tx, ty) {
            let lx = tx.rem_euclid(REGION);
            let ly = ty.rem_euclid(REGION);
            if ly == 0 && lx == 24 && hash(tx, ty) % 2 == 0 {
                self.try_spawn_truck(tx, ty, false);
            } else if lx == 0 && ly == 24 && hash(tx, ty) % 2 == 1 {
                self.try_spawn_truck(tx, ty, true);
            } else if ly == 0 && lx == 8 && hash(tx, ty) % 3 == 0 {
                self.try_spawn_tank(tx, ty, false);
            } else if lx == 0 && ly == 8 && hash(tx, ty) % 3 == 1 {
                self.try_spawn_tank(tx, ty, true);
            }
        }
    }

    pub(crate) fn retire_far(&mut self) {
        let cx = tile_of(self.x);
        let cy = tile_of(self.y);
        for soldier in &mut self.soldiers {
            if !soldier.alive {
                continue;
            }
            let tx = tile_of(soldier.x);
            let ty = tile_of(soldier.y);
            if soldier.officer || soldier.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                soldier.alive = false;
            }
        }
        for truck in &mut self.trucks {
            if !truck.alive {
                continue;
            }
            let tx = tile_of(truck.x);
            let ty = tile_of(truck.y);
            if truck.goal || truck.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                truck.alive = false;
            }
        }
        for tank in &mut self.tanks {
            if !tank.alive {
                continue;
            }
            let tx = tile_of(tank.x);
            let ty = tile_of(tank.y);
            if tank.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                tank.alive = false;
            }
        }
        for bunker in &mut self.bunkers {
            if !bunker.alive {
                continue;
            }
            let tx = tile_of(bunker.x);
            let ty = tile_of(bunker.y);
            if bunker.objective {
                continue;
            }
            if (tx - cx).abs() > 70 || (ty - cy).abs() > 70 {
                bunker.alive = false;
            }
        }
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
