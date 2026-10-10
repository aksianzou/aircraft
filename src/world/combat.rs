use super::*;
use agb::display::tiled::AffineBackground;
use crate::draw::{isqrt64, project};

impl super::World {
    pub(crate) fn try_shot(&mut self, kind: u8) -> bool {
        // One release. The round leaves the airframe with the airframe's
        // speed. The glass does not move the spawn.
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        *slot = Shot {
            x: self.x,
            y: self.y,
            z: self.alt,
            vx: self.vx,
            vy: self.vy,
            vz: -((110 * self.pitch.sin().to_raw()) >> 8),
            kind,
            life: 0,
            alive: true,
        };
        if kind == KIND_BOMB {
            self.sfx |= SFX_WHISTLE;
        }
        true
    }

    /// Where the nose meets the ground. Pitch and height decide it. The glass
    /// is drawn so this point sits on the crosshair, and the overhead picture
    /// draws the crosshair on this point.
    pub(crate) fn nose_ground(&self, sin_r: i32, cos_r: i32) -> (i32, i32, i32) {
        let view_cy = VIEW_Y + VIEW_H / 2;
        let drop = (view_cy - self.horizon()).max(4);
        let fwd = (self.alt as i64) * (FOCAL as i64) * (FLOOR_ZOOM as i64) / drop as i64;
        // Ninety tiles. That covers a group by the posts and the ground ahead of it.
        let lead = (fwd >> 8).clamp(8, 720) as i32;
        (
            wrap_coord(self.x + sin_r * lead),
            wrap_coord(self.y - cos_r * lead),
            0,
        )
    }

    /// The drone in the glass, if the crosshair is on its picture.
    /// One above the airframe is not drawn, so it cannot be the target either.
    pub(crate) fn foe_under_sight(&self) -> Option<usize> {
        let (sin_r, cos_r) = self.heading();
        let horizon = self.horizon();
        let alt_px = self.alt >> 8;
        let aim_y = VIEW_Y + VIEW_H / 2;
        let mut best: Option<(usize, i32)> = None;
        for (index, foe) in self.foes.iter().enumerate() {
            if !foe.alive || (foe.z >> 8) + 2 >= alt_px {
                continue;
            }
            let Some((sx, sy)) = project(
                self.x,
                self.y,
                self.alt,
                sin_r,
                cos_r,
                horizon,
                foe.x,
                foe.y,
                foe.z,
            ) else {
                continue;
            };
            let dx = sx - WIDTH / 2;
            let dy = sy - aim_y;
            let dist = dx * dx + dy * dy;
            // On the picture, not a wide halo around it.
            if dist > 6 * 6 {
                continue;
            }
            if best.is_none_or(|(_, near)| dist < near) {
                best = Some((index, dist));
            }
        }
        best.map(|(index, _)| index)
    }

    /// The crosshair on the body, not the feet. The figure is drawn above the
    /// ground point, so a round aimed at the picture has to go to the man.
    pub(crate) fn man_under_sight(&self) -> Option<usize> {
        let (sin_r, cos_r) = self.heading();
        let horizon = self.horizon();
        let aim_x = WIDTH / 2;
        let aim_y = VIEW_Y + VIEW_H / 2;
        let alt_px = (self.alt >> 8).max(1);
        let mut best: Option<(usize, i32)> = None;
        for (index, soldier) in self.soldiers.iter().enumerate() {
            if !soldier.alive {
                continue;
            }
            let Some((sx, sy)) = project(
                self.x,
                self.y,
                self.alt,
                sin_r,
                cos_r,
                horizon,
                soldier.x,
                soldier.y,
                0,
            ) else {
                continue;
            };
            let drop = (sy - horizon).max(1);
            let span = (16 * drop / (alt_px * FLOOR_ZOOM)).max(6);
            let half = (span / 2).clamp(4, 14);
            let dx = (sx - aim_x).abs();
            let above = sy - aim_y;
            if dx > half || above < -4 || above > span {
                continue;
            }
            let dist = dx * dx + (above - span / 2) * (above - span / 2);
            if best.is_none_or(|(_, near)| dist < near) {
                best = Some((index, dist));
            }
        }
        best.map(|(index, _)| index)
    }

    /// A burst along the nose to the ground. It hurts people and trucks, not the ground.
    /// A hostile quad in the crosshair is the target instead, so the round meets that picture.
    pub(crate) fn try_gun(&mut self) -> bool {
        let (sin_r, cos_r) = self.heading();
        let (nx, ny, nz) = if let Some(index) = self.foe_under_sight() {
            let foe = &self.foes[index];
            (foe.x, foe.y, foe.z)
        } else if let Some(index) = self.man_under_sight() {
            let man = &self.soldiers[index];
            (man.x, man.y, 0)
        } else {
            self.nose_ground(sin_r, cos_r)
        };
        let span = (nx as i64 - self.x as i64)
            .abs()
            .max((ny as i64 - self.y as i64).abs())
            >> 8;
        let steps = (span / 6).clamp(4, 18) as i32;
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        *slot = Shot {
            x: self.x,
            y: self.y,
            z: self.alt,
            vx: (nx - self.x) / steps,
            vy: (ny - self.y) / steps,
            vz: (nz - self.alt) / steps,
            kind: KIND_GUN,
            life: 0,
            alive: true,
        };
        self.sfx |= SFX_GUN;
        true
    }

    /// A round at the aircraft, not at the crosshair. `slope` is the steepest
    /// the barrel will lift (rise/run in eighths). Past that the shell keeps
    /// the low arc and passes under the machine.
    pub(crate) fn launch_at_player(&mut self, x: i32, y: i32, kind: u8, slope: i32) -> bool {
        let Some(slot) = self.shots.iter_mut().find(|shot| !shot.alive) else {
            return false;
        };
        // A few pixels of scatter. Flak throws a wider burst: a machine that
        // holds still would otherwise sit inside every shell, because the
        // round is aimed at where it is now and does not lead.
        let spread: u32 = if kind == KIND_FLAK { 41 } else { 9 };
        let half = (spread as i32) / 2;
        let scatter_x = (hash(self.age, x) % spread) as i32 - half;
        let scatter_y = (hash(self.age.wrapping_add(9), y) % spread) as i32 - half;
        let tx = self.x.wrapping_add(scatter_x << 8);
        let ty = self.y.wrapping_add(scatter_y << 8);
        let dx = delta(tx, x);
        let dy = delta(ty, y);
        let adx = (dx >> 8).abs();
        let ady = (dy >> 8).abs();
        let dist = adx.max(ady) + adx.min(ady) / 2;
        let muzzle = 1 << 8;
        let rise = self.alt - muzzle;
        // A flat barrel cannot crane over its own hull. The old ten-pixel
        // pin was smaller than the hatch, so hovering to aim ate the shells.
        // Flak still points up.
        let overhead = if slope < 8 { dist < TANK_NEAR } else { dist < 10 };
        let steps = if overhead {
            if slope >= 8 { 16 } else { 10 }
        } else {
            (dist / 3).clamp(8, 26)
        };
        let vx = dx / steps;
        let vy = dy / steps;
        let horiz = (vx.abs().max(vy.abs()) + vx.abs().min(vy.abs()) / 2).max(1);
        let mut vz = rise / steps + GRAVITY * (steps + 1) / 2;
        let cap = if overhead && slope < 8 {
            48
        } else if overhead {
            vz.max(48)
        } else {
            (horiz * slope / 8).max(48)
        };
        if vz > cap {
            vz = cap;
        }
        if vz < 0 {
            vz = 0;
        }
        *slot = Shot {
            x,
            y,
            z: muzzle,
            vx,
            vy,
            vz,
            kind,
            life: 0,
            alive: true,
        };
        true
    }

    /// Under `AA_HIDE` a battery sees the drone inside its circle. At that
    /// height and above, the drone is gone and the guns do not fire.
    pub(crate) fn aa_tick(&mut self) {
        let guns = aa_guns(task_of(self.level));
        if guns.is_empty() || self.recover > 0 {
            return;
        }
        if !guns_see(self.alt >> 8) {
            return;
        }
        if !aa_covers(guns, self.x, self.y) {
            return;
        }
        let heat = heat(self.level);
        let chip = 1 + heat / 10;
        // The circle burns while you stay in it. Twice as slow as the old
        // tick, so crossing it and stopping to aim are not the same death.
        if self.age % ((44 - heat) * 2) == 0 {
            self.hull -= chip;
            if self.hull < 0 {
                self.hull = 0;
            }
        }
        if self.age % (52 - heat) == 0 {
            let mut best = 0;
            let mut best_d = i32::MAX;
            for (index, gun) in guns.iter().enumerate() {
                let (gx, gy) = gun_at(gun);
                let distance = dist2_px(self.x, self.y, gx, gy);
                if distance < best_d {
                    best_d = distance;
                    best = index;
                }
            }
            let (gx, gy) = gun_at(&guns[best]);
            let _ = self.launch_at_player(gx, gy, KIND_FLAK, FLAK_SLOPE);
        }
    }

    /// Whether this gunner commits. Low cards miss often, and a high pass is
    /// harder to hit than a deck-level one. The last card fires in range.
    pub(crate) fn will_shoot(&self, salt: i32, keen: bool) -> bool {
        let heat = heat(self.level);
        let roll = (hash(self.age, salt) % 12) as i32;
        let need = if keen {
            6 + heat * 6 / 16
        } else {
            2 + heat * 5 / 16
        };
        roll < need
    }

    pub(crate) fn enemy_fire(&mut self) {
        if self.mode == Mode::Gate {
            return;
        }
        let alt_px = self.alt >> 8;
        // Same line as the batteries. Above it the drone is not there to shoot.
        if !guns_see(alt_px) {
            return;
        }
        let keen = alt_px <= 7;
        let pause = (16 - heat(self.level)) * 6;
        let cool = |base: i32| (base + pause).clamp(1, 255) as u8;
        for index in 0..MAX_SOLDIERS {
            if !self.soldiers[index].alive {
                continue;
            }
            if self.soldiers[index].cool > 0 {
                self.soldiers[index].cool -= 1;
                continue;
            }
            let sx = self.soldiers[index].x;
            let sy = self.soldiers[index].y;
            if dist2_px(self.x, self.y, sx, sy) > 40 * 40 {
                continue;
            }
            if !self.will_shoot(index as i32, keen) {
                self.soldiers[index].cool = cool(50);
                continue;
            }
            self.soldiers[index].cool = cool(200);
            let _ = self.launch_at_player(sx, sy, KIND_FLAK, FLAK_SLOPE);
        }
        for index in 0..MAX_TRUCKS {
            if !self.trucks[index].alive {
                continue;
            }
            if self.trucks[index].cool > 0 {
                self.trucks[index].cool -= 1;
                continue;
            }
            let tx = self.trucks[index].x;
            let ty = self.trucks[index].y;
            if dist2_px(self.x, self.y, tx, ty) > 48 * 48 {
                continue;
            }
            if !self.will_shoot(index as i32 + 19, keen) {
                self.trucks[index].cool = cool(40);
                continue;
            }
            self.trucks[index].cool = cool(170);
            let _ = self.launch_at_player(tx, ty, KIND_SHELL, TANK_SLOPE);
        }
        for index in 0..MAX_TANKS {
            if !self.tanks[index].alive {
                continue;
            }
            if self.tanks[index].cool > 0 {
                self.tanks[index].cool -= 1;
                continue;
            }
            let tx = self.tanks[index].x;
            let ty = self.tanks[index].y;
            if dist2_px(self.x, self.y, tx, ty) > 56 * 56 {
                continue;
            }
            if !self.will_shoot(index as i32 + 31, keen) {
                self.tanks[index].cool = cool(40);
                continue;
            }
            self.tanks[index].cool = cool(150);
            let _ = self.launch_at_player(tx, ty, KIND_SHELL, TANK_SLOPE);
        }
        for index in 0..MAX_BUNKERS {
            if !self.bunkers[index].alive {
                continue;
            }
            if self.bunkers[index].cool > 0 {
                self.bunkers[index].cool -= 1;
                continue;
            }
            let bx = self.bunkers[index].x;
            let by = self.bunkers[index].y;
            if dist2_px(self.x, self.y, bx, by) > 64 * 64 {
                continue;
            }
            if !self.will_shoot(index as i32 + 47, keen) {
                self.bunkers[index].cool = cool(36);
                continue;
            }
            self.bunkers[index].cool = cool(110);
            let _ = self.launch_at_player(bx, by, KIND_FLAK, FLAK_SLOPE);
        }
    }

    pub(crate) fn step_gun(&mut self, bg: &mut AffineBackground, index: usize) {
        self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
        self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
        self.shots[index].z += self.shots[index].vz;
        self.shots[index].life += 1;
        let x = self.shots[index].x;
        let y = self.shots[index].y;
        let z = self.shots[index].z;
        let life = self.shots[index].life;
        let prev_x = x - self.shots[index].vx;
        let prev_y = y - self.shots[index].vy;
        if z > 16 << 8 && life < 10 {
            return;
        }
        let mut bird = None;
        for (slot, foe) in self.foes.iter().enumerate() {
            if foe.alive
                && dist2_px(x, y, foe.x, foe.y) <= 8 * 8
                && ((z - foe.z) >> 8).abs() <= 5
            {
                bird = Some(slot);
                break;
            }
        }
        if let Some(slot) = bird {
            self.shots[index].alive = false;
            self.hurt_foe(slot);
            return;
        }
        let mut man = None;
        // Only once the round is down among them. A high pass is not a hit,
        // and the step is long enough that a single sample can jump a man.
        if z <= 12 << 8 {
            for (slot, soldier) in self.soldiers.iter().enumerate() {
                if soldier.alive && near_step(soldier.x, soldier.y, prev_x, prev_y, x, y, 10) {
                    man = Some(slot);
                    break;
                }
            }
        }
        if let Some(slot) = man {
            self.shots[index].alive = false;
            self.hurt_soldier(bg, slot);
            return;
        }
        let mut rig = None;
        for (slot, truck) in self.trucks.iter().enumerate() {
            if truck.alive && dist2_px(x, y, truck.x, truck.y) <= 10 * 10 {
                rig = Some(slot);
                break;
            }
        }
        if let Some(slot) = rig {
            self.shots[index].alive = false;
            self.hurt_truck(bg, slot);
            return;
        }
        let mut armour = None;
        for (slot, tank) in self.tanks.iter().enumerate() {
            if tank.alive && dist2_px(x, y, tank.x, tank.y) <= 12 * 12 {
                armour = Some(slot);
                break;
            }
        }
        if let Some(slot) = armour {
            self.shots[index].alive = false;
            self.hurt_tank(bg, slot);
            return;
        }
        let mut nest = None;
        for (slot, bunker) in self.bunkers.iter().enumerate() {
            if bunker.alive && dist2_px(x, y, bunker.x, bunker.y) <= 12 * 12 {
                nest = Some(slot);
                break;
            }
        }
        if let Some(slot) = nest {
            self.shots[index].alive = false;
            self.hurt_bunker(bg, slot);
            return;
        }
        if z <= 1 << 8 || life > 22 {
            let tx = tile_of(x);
            let ty = tile_of(y);
            if self.hq_goal(tx, ty) {
                self.paint(bg, tx, ty, T_CRATER);
            }
            self.shots[index].alive = false;
        }
    }

    pub(crate) fn hurt_foe(&mut self, index: usize) {
        let x = self.foes[index].x;
        let y = self.foes[index].y;
        let z = self.foes[index].z;
        self.foes[index].alive = false;
        self.add_boom(x, y, z, 0);
        self.score += 50;
        self.sfx |= SFX_BOOM;
        if matches!(task_of(self.level), Task::Swarm | Task::Cover) && self.targets > 0 {
            self.targets -= 1;
            self.sfx |= SFX_MARK;
        }
    }

    /// Holds one height. Chases under `AA_HIDE` and goes home above it.
    /// The bullet is the only thing that brings one down. The same height is a bump.
    pub(crate) fn update_foes(&mut self) {
        let see = guns_see(self.alt >> 8) && !self.falling;
        let px = self.x;
        let py = self.y;
        let pz = self.alt;
        let defend = defend_card(self.level);
        let cover = matches!(task_of(self.level), Task::Cover);
        let escort = if cover {
            self.trucks
                .iter()
                .find(|truck| truck.alive && truck.goal)
                .map(|truck| (truck.x, truck.y))
        } else {
            None
        };
        let base_x = self.pad_x;
        let base_y = self.pad_y;
        let mut bump = false;
        for foe in &mut self.foes {
            if !foe.alive {
                continue;
            }
            if foe.cool > 0 {
                foe.cool -= 1;
            }
            // A wing keeps its cruise. It does not park on the truck or the player.
            if foe.wing {
                let speed = if defend {
                    40
                } else if cover {
                    80
                } else {
                    64
                };
                if defend {
                    let dx = base_x.wrapping_sub(foe.x);
                    let step_x = dx.signum() * 20;
                    foe.x = foe.x.wrapping_add(if step_x.abs() > dx.abs() { dx } else { step_x });
                    foe.y = foe.y.wrapping_add(speed);
                } else {
                    if foe.heading == 0 {
                        foe.heading = -1;
                    }
                    let ty = tile_of(foe.y);
                    if ty <= 8 {
                        foe.heading = 1;
                    } else if ty >= 58 {
                        foe.heading = -1;
                    }
                    foe.y = foe.y.wrapping_add(i32::from(foe.heading) * speed);
                    if see && !cover {
                        let dx = px.wrapping_sub(foe.x);
                        foe.x = foe.x.wrapping_add(dx.signum() * 16);
                    }
                }
            } else {
                // Crows fly at the pad. Cover rides above the truck. The swarm flies at you.
                let (tx, ty) = if defend {
                    (base_x, base_y)
                } else if let Some((ex, ey)) = escort {
                    let side = if foe.home_x < ex { -(24 << 8) } else { 24 << 8 };
                    (ex.wrapping_add(side), ey.wrapping_sub(20 << 8))
                } else if see {
                    (px, py)
                } else {
                    (foe.home_x, foe.home_y)
                };
                let dx = tx.wrapping_sub(foe.x) as i64;
                let dy = ty.wrapping_sub(foe.y) as i64;
                let dist = isqrt64(dx * dx + dy * dy);
                let step = if defend {
                    56
                } else if cover {
                    40
                } else {
                    72
                };
                if dist > 256 {
                    foe.x = foe.x.wrapping_add((dx * step / dist) as i32);
                    foe.y = foe.y.wrapping_add((dy * step / dist) as i32);
                }
            }
            let dz = ((foe.z - pz) >> 8).abs();
            if dz <= 2 && foe.cool == 0 && dist2_px(foe.x, foe.y, px, py) <= 12 * 12 {
                foe.cool = 45;
                let ox = foe.x.wrapping_sub(px) as i64;
                let oy = foe.y.wrapping_sub(py) as i64;
                let push = isqrt64(ox * ox + oy * oy).max(1);
                foe.x = px.wrapping_add((ox * (18 << 8) / push) as i32);
                foe.y = py.wrapping_add((oy * (18 << 8) / push) as i32);
                if ox == 0 && oy == 0 {
                    foe.y = py.wrapping_sub(18 << 8);
                }
                bump = true;
            }
        }
        if bump {
            self.hull = (self.hull - 16).max(0);
            self.sfx |= SFX_BOOM;
        }
    }

    pub(crate) fn hurt_soldier(&mut self, bg: &mut AffineBackground, index: usize) {
        // A rifleman drops in a short burst. An officer takes the longer one.
        let bite = if self.soldiers[index].officer { 3 } else { 4 };
        if self.soldiers[index].hp > bite {
            self.soldiers[index].hp -= bite;
            return;
        }
        self.soldiers[index].hp = 0;
        let x = self.soldiers[index].x;
        let y = self.soldiers[index].y;
        let hx = self.soldiers[index].home_tx;
        let hy = self.soldiers[index].home_ty;
        let officer = self.soldiers[index].officer;
        let objective = self.soldiers[index].objective;
        self.soldiers[index].alive = false;
        self.mark_quiet(hx, hy);
        self.note_kill(
            bg,
            x,
            y,
            if officer { 200 } else { 50 },
            officer,
            false,
            false,
            objective,
        );
    }

    pub(crate) fn hurt_truck(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.trucks[index].hp > 0 {
            self.trucks[index].hp -= 1;
        }
        if self.trucks[index].hp > 0 {
            return;
        }
        let x = self.trucks[index].x;
        let y = self.trucks[index].y;
        let hx = self.trucks[index].home_tx;
        let hy = self.trucks[index].home_ty;
        let goal = self.trucks[index].goal;
        let objective = self.trucks[index].objective;
        self.trucks[index].alive = false;
        self.mark_quiet(hx, hy);
        self.note_kill(
            bg,
            x,
            y,
            if goal { 200 } else { 100 },
            false,
            !goal,
            goal,
            objective,
        );
    }

    pub(crate) fn hurt_tank(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.tanks[index].hp > 0 {
            self.tanks[index].hp -= 1;
        }
        if self.tanks[index].hp > 0 {
            return;
        }
        let x = self.tanks[index].x;
        let y = self.tanks[index].y;
        let hx = self.tanks[index].home_tx;
        let hy = self.tanks[index].home_ty;
        let objective = self.tanks[index].objective;
        self.tanks[index].alive = false;
        self.mark_quiet(hx, hy);
        if objective {
            self.claim_armor();
        }
        self.note_kill(bg, x, y, 150, false, false, false, false);
    }

    pub(crate) fn hurt_bunker(&mut self, bg: &mut AffineBackground, index: usize) {
        if self.bunkers[index].hp > 0 {
            self.bunkers[index].hp -= 1;
        }
        if self.bunkers[index].hp > 0 {
            return;
        }
        let x = self.bunkers[index].x;
        let y = self.bunkers[index].y;
        let hx = self.bunkers[index].home_tx;
        let hy = self.bunkers[index].home_ty;
        let objective = self.bunkers[index].objective;
        self.bunkers[index].alive = false;
        self.mark_quiet(hx, hy);
        if objective {
            self.claim_pill();
        }
        self.note_kill(bg, x, y, 100, false, false, false, false);
    }

    pub(crate) fn step_flak(&mut self, index: usize) {
        let kind = self.shots[index].kind;
        self.shots[index].vz -= GRAVITY;
        self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
        self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
        self.shots[index].z += self.shots[index].vz;
        self.shots[index].life += 1;
        let x = self.shots[index].x;
        let y = self.shots[index].y;
        let z = self.shots[index].z;
        let life = self.shots[index].life;
        let near = dist2_px(x, y, self.x, self.y) <= 7 * 7;
        let level = (z - self.alt).abs() <= 4 << 8;
        if near && level {
            self.shots[index].alive = false;
            let heat = heat(self.level);
            let hit = if kind == KIND_SHELL {
                4 + heat / 4
            } else {
                2 + heat / 8
            };
            self.hull -= hit;
            if self.hull < 0 {
                self.hull = 0;
            }
            self.add_boom(self.x, self.y, z, 0);
            return;
        }
        if life > 28 || z > 48 << 8 || z < 0 {
            self.shots[index].alive = false;
        }
    }

    pub(crate) fn update_shots(&mut self, bg: &mut AffineBackground) {
        for index in 0..MAX_SHOTS {
            if !self.shots[index].alive {
                continue;
            }
            let kind = self.shots[index].kind;
            if kind == KIND_GUN {
                self.step_gun(bg, index);
                continue;
            }
            if kind == KIND_FLAK || kind == KIND_SHELL {
                self.step_flak(index);
                continue;
            }
            self.shots[index].vz -= GRAVITY;
            self.shots[index].z += self.shots[index].vz;
            self.shots[index].x = wrap_coord(self.shots[index].x + self.shots[index].vx);
            self.shots[index].y = wrap_coord(self.shots[index].y + self.shots[index].vy);
            self.shots[index].life += 1;
            if self.shots[index].z > 0 && self.shots[index].life < 400 {
                continue;
            }
            let shot = self.shots[index];
            self.shots[index].alive = false;
            let radius = match shot.kind {
                KIND_HE => 22,
                KIND_VOG => 12,
                KIND_MINE => 10,
                _ => 14,
            };
            self.explode(bg, shot.x, shot.y, radius, 0, shot.kind);
        }
    }

    /// A rig follows a road. Off the road it keeps the facing it has, instead of
    /// reversing every frame and spinning in place.
    fn keep_on_road(&mut self, index: usize, truck: bool, step: i32) {
        let (x, y, dir, vertical) = if truck {
            let rig = &self.trucks[index];
            (rig.x, rig.y, rig.dir, rig.vertical)
        } else {
            let rig = &self.tanks[index];
            (rig.x, rig.y, rig.dir, rig.vertical)
        };
        if dir == 0 {
            return;
        }
        let probe = dir.signum() * (10 << 8);
        let (ax, ay) = if vertical { (x, y + probe) } else { (x + probe, y) };
        if self.ground_at(tile_of(ax), tile_of(ay)) == T_ROAD {
            return;
        }
        if truck {
            if vertical {
                self.trucks[index].y -= step;
            } else {
                self.trucks[index].x -= step;
            }
        } else if vertical {
            self.tanks[index].y -= step;
        } else {
            self.tanks[index].x -= step;
        }
        let (x, y) = if truck {
            (self.trucks[index].x, self.trucks[index].y)
        } else {
            (self.tanks[index].x, self.tanks[index].y)
        };
        let back = -dir.signum() * (10 << 8);
        let (bx, by) = if vertical { (x, y + back) } else { (x + back, y) };
        let here = self.ground_at(tile_of(x), tile_of(y)) == T_ROAD;
        let behind = self.ground_at(tile_of(bx), tile_of(by)) == T_ROAD;
        if here || behind {
            if truck {
                self.trucks[index].dir = -self.trucks[index].dir;
            } else {
                self.tanks[index].dir = -self.tanks[index].dir;
            }
        }
    }

    pub(crate) fn update_trucks(&mut self) {
        for index in 0..MAX_TRUCKS {
            if !self.trucks[index].alive {
                continue;
            }
            if self.trucks[index].objective && matches!(task_of(self.level), Task::Push) {
                self.trucks[index].y -= rush_step(self.level);
                continue;
            }
            // Faster than a road crawl, slow enough to climb, shoot, then dive.
            if self.trucks[index].goal && matches!(task_of(self.level), Task::Cover) {
                self.trucks[index].y -= 24;
                continue;
            }
            let step = ground_step(self.trucks[index].dir);
            if self.trucks[index].vertical {
                self.trucks[index].y += step;
            } else {
                self.trucks[index].x += step;
            }
            if self.trucks[index].goal {
                continue;
            }
            self.keep_on_road(index, true, step);
        }
    }

    pub(crate) fn update_tanks(&mut self) {
        for index in 0..MAX_TANKS {
            if !self.tanks[index].alive {
                continue;
            }
            let step = ground_step(self.tanks[index].dir);
            if self.tanks[index].vertical {
                self.tanks[index].y += step;
            } else {
                self.tanks[index].x += step;
            }
            self.keep_on_road(index, false, step);
        }
    }

    pub(crate) fn update_soldiers(&mut self) {
        for index in 0..MAX_SOLDIERS {
            if !self.soldiers[index].alive {
                continue;
            }
            if self.soldiers[index].objective && matches!(task_of(self.level), Task::Fpv | Task::Nest)
            {
                continue;
            }
            if self.soldiers[index].objective
                && matches!(task_of(self.level), Task::Rush | Task::Push)
            {
                self.soldiers[index].y -= rush_step(self.level);
                self.soldiers[index].step = self.soldiers[index].step.wrapping_add(1);
                continue;
            }
            let sign = if self.soldiers[index].dir < 0 { -1 } else { 1 };
            let ahead = self.soldiers[index].x + sign * (10 << 8);
            let tile = self.ground_at(tile_of(ahead), tile_of(self.soldiers[index].y));
            let sign = if tile == T_TRENCH || tile == T_LIP {
                sign
            } else {
                -sign
            };
            self.soldiers[index].dir = sign * 80;
            self.soldiers[index].x += ground_step(self.soldiers[index].dir);
            self.soldiers[index].step = self.soldiers[index].step.wrapping_add(1);
        }
    }

    pub(crate) fn update_booms(&mut self) {
        for boom in &mut self.booms {
            if !boom.alive {
                continue;
            }
            boom.frame += 1;
            if boom.frame > 40 {
                boom.alive = false;
            }
        }
    }

    pub(crate) fn explode(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32, depth: i32, kind: u8) {
        self.add_boom(x, y, 0, radius);
        self.damage_tiles(bg, x, y, radius);
        let reach = radius * radius;
        let mut graves = [(0i32, 0i32); MAX_SOLDIERS + MAX_TRUCKS];
        let mut graves_n = 0;
        let mut trucks_hit = 0;
        let mut goals_hit = 0;
        let mut obj_trucks = 0;
        let mut officers_hit = 0;
        let mut obj_men = 0;
        let mut men_hit = 0;
        for truck in &mut self.trucks {
            if kind == KIND_BOMB && truck.alive && dist2_px(truck.x, truck.y, x, y) <= reach {
                graves[graves_n] = (truck.home_tx, truck.home_ty);
                graves_n += 1;
                if truck.goal {
                    goals_hit += 1;
                } else if truck.objective {
                    obj_trucks += 1;
                } else {
                    trucks_hit += 1;
                }
                truck.alive = false;
            }
        }
        for soldier in &mut self.soldiers {
            if matches!(kind, KIND_VOG | KIND_BOMB | KIND_HE | KIND_MINE)
                && soldier.alive
                && dist2_px(soldier.x, soldier.y, x, y) <= reach
            {
                graves[graves_n] = (soldier.home_tx, soldier.home_ty);
                graves_n += 1;
                if soldier.officer {
                    officers_hit += 1;
                } else if soldier.objective {
                    obj_men += 1;
                } else {
                    men_hit += 1;
                }
                soldier.alive = false;
            }
        }
        self.score += (trucks_hit + goals_hit + obj_trucks) * 100
            + goals_hit * 100
            + (officers_hit + obj_men) * 50
            + officers_hit * 150
            + men_hit * 50;
        for _ in 0..trucks_hit {
            self.claim(false, true, false, false);
        }
        for _ in 0..obj_trucks {
            self.claim(false, true, false, true);
        }
        for _ in 0..goals_hit {
            self.claim(false, false, true, true);
        }
        for _ in 0..officers_hit {
            self.claim(true, false, false, true);
        }
        for _ in 0..obj_men {
            self.claim(false, false, false, true);
        }
        for _ in 0..men_hit {
            self.claim(false, false, false, false);
        }
        for step in 0..graves_n {
            self.mark_quiet(graves[step].0, graves[step].1);
        }
        let mut wrecks = [(0i32, 0i32); MAX_TANKS + MAX_BUNKERS];
        let mut wrecks_n = 0;
        let mut armor = 0;
        for tank in &mut self.tanks {
            if !tank.alive || kind == KIND_GUN {
                continue;
            }
            let dist = dist2_px(tank.x, tank.y, x, y);
            let hatch = dist <= HATCH * HATCH;
            let hits = hull_hits(kind);
            let near = hits > 0 && dist <= TANK_NEAR * TANK_NEAR;
            if !hatch && !near {
                continue;
            }
            if hatch {
                tank.hp = 0;
            } else {
                tank.hp -= HP_TANK / hits;
            }
            if tank.hp > 0 {
                continue;
            }
            wrecks[wrecks_n] = (tank.home_tx, tank.home_ty);
            wrecks_n += 1;
            if tank.objective {
                armor += 1;
            }
            tank.alive = false;
            self.score += 150;
        }
        let mut pills = 0;
        for bunker in &mut self.bunkers {
            if kind == KIND_HE && bunker.alive && dist2_px(bunker.x, bunker.y, x, y) <= reach {
                wrecks[wrecks_n] = (bunker.home_tx, bunker.home_ty);
                wrecks_n += 1;
                if bunker.objective {
                    pills += 1;
                }
                bunker.alive = false;
                self.score += 100;
            }
        }
        for _ in 0..armor {
            self.claim_armor();
        }
        for _ in 0..pills {
            self.claim_pill();
        }
        for step in 0..wrecks_n {
            self.mark_quiet(wrecks[step].0, wrecks[step].1);
        }
        if depth >= 4 {
            return;
        }
        let mut chain = [(0i32, 0i32); MAX_MINES];
        let mut count = 0;
        for mine in &mut self.mine_list {
            if mine.alive && dist2_px(mine.x, mine.y, x, y) <= reach {
                mine.alive = false;
                chain[count] = (mine.x, mine.y);
                count += 1;
            }
        }
        for step in 0..count {
            self.explode(bg, chain[step].0, chain[step].1, 24, depth + 1, KIND_BOMB);
        }
    }

    pub(crate) fn damage_tiles(&mut self, bg: &mut AffineBackground, x: i32, y: i32, radius: i32) {
        let cx = x >> 8;
        let cy = y >> 8;
        let reach = radius * radius;
        let x0 = (cx - radius).div_euclid(8);
        let x1 = (cx + radius).div_euclid(8);
        let y0 = (cy - radius).div_euclid(8);
        let y1 = (cy + radius).div_euclid(8);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let dx = tx * 8 + 4 - cx;
                let dy = ty * 8 + 4 - cy;
                if dx * dx + dy * dy > reach {
                    continue;
                }
                let tile = self.ground_at(tx, ty);
                if tile == T_WATER {
                    continue;
                }
                // On the trunk the tree is gone. Further out it catches, then falls.
                let gone = radius * radius / 4;
                if dx * dx + dy * dy > gone && matches!(tile, T_TREE | T_BUSH) {
                    self.light_tree(bg, tx, ty);
                    continue;
                }
                self.paint(bg, tx, ty, T_CRATER);
            }
        }
    }

    pub(crate) fn light_tree(&mut self, bg: &mut AffineBackground, tx: i32, ty: i32) {
        if self
            .burns
            .iter()
            .any(|burn| burn.alive && burn.tx == tx && burn.ty == ty)
        {
            return;
        }
        let Some(slot) = self.burns.iter_mut().find(|burn| !burn.alive) else {
            self.paint(bg, tx, ty, T_CRATER);
            return;
        };
        *slot = Burn {
            tx,
            ty,
            life: 80,
            alive: true,
        };
    }

    pub(crate) fn update_burns(&mut self, bg: &mut AffineBackground) {
        for index in 0..MAX_BURNS {
            if !self.burns[index].alive {
                continue;
            }
            if self.burns[index].life > 0 {
                self.burns[index].life -= 1;
                continue;
            }
            let tx = self.burns[index].tx;
            let ty = self.burns[index].ty;
            self.burns[index].alive = false;
            if matches!(self.ground_at(tx, ty), T_TREE | T_BUSH) {
                self.paint(bg, tx, ty, T_CRATER);
            }
        }
    }

    pub(crate) fn paint(&mut self, bg: &mut AffineBackground, tx: i32, ty: i32, tile: u8) {
        let previous = self.ground_at(tx, ty);
        if tile == T_CRATER {
            for burn in &mut self.burns {
                if burn.alive && burn.tx == tx && burn.ty == ty {
                    burn.alive = false;
                }
            }
        }
        self.insert_mark(tx, ty, tile, false);
        let _ = bg;
        self.blit(tx, ty);
        if tile == T_CRATER && previous != T_CRATER && self.hq_goal(tx, ty) && self.targets > 0 {
            self.targets -= 1;
            self.score += 50;
            self.sfx |= SFX_MARK;
        }
    }

    /// The first cause sticks. A later hit in the same frame does not replace it.
    pub(crate) fn fail(&mut self, why: Why) {
        if self.crashed {
            return;
        }
        self.crashed = true;
        self.why = why;
    }

    /// The convoy truck reached the mark. The sortie is lost.
    pub(crate) fn check_convoy(&mut self) {
        if !matches!(task_of(self.level), Task::Convoy) || self.targets <= 0 {
            return;
        }
        let arrived = self.trucks.iter().any(|truck| {
            truck.alive && truck.goal && tile_of(truck.y) <= 4
        });
        if arrived {
            self.fail(Why::Truck);
        }
    }

    /// The escort truck reached the posts. The quads do not stop that.
    pub(crate) fn check_cover(&mut self) {
        if !matches!(task_of(self.level), Task::Cover) || self.targets <= 0 {
            return;
        }
        let arrived = self
            .trucks
            .iter()
            .any(|truck| truck.alive && truck.goal && tile_of(truck.y) <= 14);
        if arrived {
            self.fail(Why::Truck);
        }
    }

    /// The base line is north. A living mark that reaches it takes the place.
    pub(crate) fn check_rush(&mut self) {
        let task = task_of(self.level);
        if !matches!(task, Task::Rush | Task::Push) || self.targets <= 0 {
            return;
        }
        let men = self.soldiers.iter().any(|soldier| {
            soldier.alive && soldier.objective && tile_of(soldier.y) <= 14
        });
        let rigs = self.trucks.iter().any(|truck| {
            truck.alive && truck.objective && tile_of(truck.y) <= 14
        });
        if men || rigs {
            self.fail(Why::Line);
        }
    }

    /// The last card. A hostile quad that reaches the pad takes the base.
    pub(crate) fn check_base(&mut self) {
        if !defend_card(self.level) || self.targets <= 0 || self.crashed {
            return;
        }
        let reached = self.foes.iter().any(|foe| {
            foe.alive && dist2_px(foe.x, foe.y, self.pad_x, self.pad_y) <= 24 * 24
        });
        if reached {
            self.add_boom(self.pad_x, self.pad_y, 0, 0);
            self.fail(Why::Base);
        }
    }

    /// Crossing the gate line either clears the card or sends the plane back.
    pub(crate) fn cross_gate(&mut self) -> bool {
        if self.mode != Mode::Gate || self.targets <= 0 {
            return true;
        }
        let gy = self.gate_y();
        let prev = self.y - self.vy;
        if !(prev > gy && self.y <= gy) {
            return true;
        }
        let dx = ((self.x - self.gate_x()) >> 8).abs();
        let alt = self.alt >> 8;
        if dx <= 16 && (8..=15).contains(&alt) {
            self.targets = 0;
            self.score += 200;
            self.sfx |= SFX_MARK;
            true
        } else {
            self.recover = 22;
            self.vx = 0;
            self.vy = 0;
            false
        }
    }

    pub(crate) fn add_boom(&mut self, x: i32, y: i32, z: i32, radius: i32) {
        let mut slot = 0;
        for (index, boom) in self.booms.iter().enumerate() {
            if !boom.alive {
                slot = index;
                break;
            }
        }
        self.booms[slot] = Boom {
            x: wrap_coord(x),
            y: wrap_coord(y),
            z,
            radius: radius.clamp(0, 120) as i16,
            frame: 0,
            alive: true,
        };
        // A near blast clips the sensor, then the picture comes back.
        // The white disc on the ground is separate and stays.
        let near = dist2_px(self.x, self.y, x, y);
        if near <= 96 * 96 {
            let punch: i16 = if near <= 40 * 40 {
                22
            } else if near <= 64 * 64 {
                16
            } else {
                10
            };
            if self.bloom < punch {
                self.bloom = punch;
            }
        }
        self.sfx |= SFX_BOOM;
    }
}

/// True when `point` comes within `radius` pixels of the segment.
fn near_step(px: i32, py: i32, ax: i32, ay: i32, bx: i32, by: i32, radius: i32) -> bool {
    let abx = bx.wrapping_sub(ax);
    let aby = by.wrapping_sub(ay);
    let apx = px.wrapping_sub(ax);
    let apy = py.wrapping_sub(ay);
    let ab2 = abx as i64 * abx as i64 + aby as i64 * aby as i64;
    let t = if ab2 <= 0 {
        0
    } else {
        ((apx as i64 * abx as i64 + apy as i64 * aby as i64) * 256 / ab2).clamp(0, 256)
    };
    let cx = ax.wrapping_add(((abx as i64 * t) >> 8) as i32);
    let cy = ay.wrapping_add(((aby as i64 * t) >> 8) as i32);
    dist2_px(px, py, cx, cy) <= radius * radius
}
