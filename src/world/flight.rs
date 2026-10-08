use super::*;
use agb::display::tiled::AffineBackground;
use agb::fixnum::{Num, num};
use agb::input::{Button, ButtonController, Tri};

impl super::World {
    pub(crate) fn heading(&self) -> (i32, i32) {
        (self.yaw.sin().to_raw(), self.yaw.cos().to_raw())
    }

    pub(crate) fn horizon(&self) -> i32 {
        // Pitch is in turns (1.0 = full circle). A dive lifts the horizon.
        let pitch_px = (self.pitch * Num::new(220)).to_raw() >> 8;
        let alt_px = self.alt >> 8;
        // Under six the ground rises in the glass, so a low pass looks low
        // and there is still air to hold before the hit.
        let skim = (6 - alt_px).max(0) * 8;
        let level = 58 - pitch_px - skim;
        // Past a shallow nose-down the vanishing point leaves the screen, so a
        // dive shows the ground ahead and not the skyline or the far map.
        if self.pitch <= num!(0.05) {
            return level.clamp(12, 108);
        }
        let alt_px = (self.alt >> 8).max(1);
        let skip = (alt_px * FLOOR_ZOOM * 25 / 48).max(12);
        let buried = -skip;
        let start = 13; // 0.05 turn
        let span = 26; // the next 0.10 turn buries the horizon
        let mix = ((self.pitch.to_raw() - start) * 256 / span).clamp(0, 256);
        (level + (buried - level) * mix / 256).clamp(-140, 108)
    }

    pub(crate) fn fly(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.bloom > 0 {
            self.bloom -= 1;
        }
        if self.recover > 0 {
            self.recover -= 1;
            if self.recover == 0 {
                self.relaunch();
            }
            self.update_booms();
            return;
        }
        if !self.fly_plane(input, bg) {
            return;
        }

        if matches!(task_of(self.level), Task::Fetch) {
            self.fetch_tick(input);
        } else if self.mode != Mode::Gate
            && self.mode != Mode::Fpv
            && self.mode != Mode::Guide
            && self.signal_loss() < 16
        {
            if input.is_just_pressed(Button::A) && !matches!(task_of(self.level), Task::Swarm) {
                self.arm = (self.arm + 1) % 5;
            }
            if self.fire_cd > 0 {
                self.fire_cd -= 1;
            }
            let arm = self.arm;
            let shoot = input.is_pressed(Button::B) && self.fire_cd == 0;
            let once = input.is_just_pressed(Button::B);
            if arm == ARM_GUN && shoot && self.try_gun() {
                self.fire_cd = 14;
            } else if arm == ARM_BOMB && shoot && self.bombs > 0 && self.try_shot(KIND_BOMB) {
                self.bombs -= 1;
                self.spent = self.spent.saturating_add(1);
                self.fire_cd = 16;
            } else if arm == ARM_HE && once && self.he > 0 && self.try_shot(KIND_HE) {
                self.he -= 1;
                self.spent = self.spent.saturating_add(1);
            } else if arm == ARM_MINE && once && self.mines > 0 && self.try_shot(KIND_MINE) {
                self.mines -= 1;
                self.spent = self.spent.saturating_add(1);
            } else if arm == ARM_VOG && once && self.vog > 0 && self.try_shot(KIND_VOG) {
                self.vog -= 1;
                self.spent = self.spent.saturating_add(1);
            }
        }
        if self.mode == Mode::Guide {
            self.guide_tick(input, bg);
        }

        self.update_shots(bg);
        self.update_trucks();
        self.update_tanks();
        self.check_convoy();
        self.check_cover();
        self.check_rush();
        self.update_soldiers();
        self.update_foes();
        self.check_base();
        self.enemy_fire();
        self.aa_tick();
        self.update_booms();
        self.wear();
    }

    /// B under the crosshair calls the salvo. The count runs, then the rounds land.
    pub(crate) fn guide_tick(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.guide == 0
            && self.salvo == 0
            && self.signal_loss() < 16
            && !self.falling
            && input.is_just_pressed(Button::B)
        {
            let (sin_r, cos_r) = self.heading();
            let (nx, ny, _) = self.nose_ground(sin_r, cos_r);
            self.mark_x = nx;
            self.mark_y = ny;
            self.lay_ground_frame(nx, ny);
            self.guide = GUIDE_WAIT;
            self.sfx |= SFX_MARK;
            self.spent = self.spent.saturating_add(1);
        }
        if self.guide > 0 {
            self.guide -= 1;
            if self.guide == 0 {
                self.salvo = 6;
                self.salvo_cd = 0;
            }
        }
        if self.salvo > 0 {
            if self.salvo_cd > 0 {
                self.salvo_cd -= 1;
            } else {
                let n = (6 - self.salvo) as i32;
                let jitter_x = if n == 0 {
                    0
                } else {
                    ((hash(self.age, n) % 33) as i32 - 16) << 8
                };
                let jitter_y = if n == 0 {
                    0
                } else {
                    ((hash(self.age, n + 5) % 33) as i32 - 16) << 8
                };
                self.explode(bg, self.mark_x + jitter_x, self.mark_y + jitter_y, 22, 0, KIND_BOMB);
                self.salvo -= 1;
                self.salvo_cd = 8;
            }
        }
        if self.guide == 0 && self.salvo == 0 && self.signal_loss() < 16 && !self.falling {
            let (sin_r, cos_r) = self.heading();
            let (nx, ny, _) = self.nose_ground(sin_r, cos_r);
            self.lay_ground_frame(nx, ny);
        }
    }

    /// A square of lip tiles on the map, around the ground under the crosshair.
    /// It stays in the world when the glass moves. Camera brackets do not.
    pub(crate) fn lay_ground_frame(&mut self, x: i32, y: i32) {
        let cx = tile_of(x);
        let cy = tile_of(y);
        if self.guide_frame_n > 0 && self.guide_cx == cx && self.guide_cy == cy {
            return;
        }
        self.clear_ground_frame();
        self.guide_cx = cx;
        self.guide_cy = cy;
        let mut n = 0;
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                if dx.abs() != 2 && dy.abs() != 2 {
                    continue;
                }
                let tx = cx + dx;
                let ty = cy + dy;
                if self.ground_at(tx, ty) == T_WATER {
                    continue;
                }
                let was = self.ground_at(tx, ty);
                self.guide_frame[n] = (tx, ty, was);
                n += 1;
                if was != T_LIP {
                    self.insert_mark(tx, ty, T_LIP, false);
                    self.blit(tx, ty);
                }
            }
        }
        self.guide_frame_n = n as u8;
    }

    pub(crate) fn clear_ground_frame(&mut self) {
        let n = self.guide_frame_n as usize;
        self.guide_frame_n = 0;
        self.guide_cx = i32::MIN;
        self.guide_cy = i32::MIN;
        for step in 0..n {
            let (tx, ty, was) = self.guide_frame[step];
            if self.ground_at(tx, ty) == T_LIP && was != T_LIP {
                self.insert_mark(tx, ty, was, false);
                self.blit(tx, ty);
            }
        }
    }

    /// The stick slides the quad. Looking down does not change that speed.
    /// Holding a direction speeds it up. A tap stays slow enough for a hatch.
    /// Forward glass: up and down are height, left and right turn.
    pub(crate) fn slide_motion(
        &mut self,
        sin_r: i32,
        cos_r: i32,
        pitch_in: Tri,
        yaw_in: Tri,
        fetch_fly: bool,
        turning: bool,
    ) {
        let slide_pitch = if turning { Tri::Zero } else { pitch_in };
        let slide_yaw = if turning { Tri::Zero } else { yaw_in };
        let creep = if fetch_fly {
            self.quad_creep(slide_pitch, slide_yaw)
        } else {
            48
        };
        let along = -tri_value(slide_pitch) * creep;
        let strafe = tri_value(slide_yaw) * creep;
        self.vx = (sin_r * along + cos_r * strafe) >> 8;
        self.vy = ((-cos_r) * along + sin_r * strafe) >> 8;
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        if !turning {
            self.bank += (Num::new(0) - self.bank) / Num::new(6);
        }
        // The Fetch quad sits on a round. Only the forward glass climbs and descends.
        if fetch_fly && !self.hover {
            self.alt += -tri_value(pitch_in) * 48;
            if self.alt < 0 {
                self.alt = 0;
            }
        }
    }

    /// A dead battery is not a wing. The quad drops where it is.
    pub(crate) fn drop_motion(&mut self) {
        self.vx = (self.vx * 3) / 4;
        self.vy = (self.vy * 3) / 4;
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        self.alt -= 96;
        if self.alt < 0 {
            self.alt = 0;
        }
    }

    /// Always flies forward. The stick has already tilted the nose.
    pub(crate) fn cruise_motion(&mut self, sin_r: i32, cos_r: i32) {
        let sp = self.pitch.sin().to_raw();
        let cp = self.pitch.cos().to_raw().max(0);
        // The spotter holds one height and loiters. Pitch only aims the glass.
        // Slower than a sortie, fast enough that the nose does not sit on a crow.
        let cruise = if self.mode == Mode::Guide {
            48
        } else if defend_card(self.level) {
            200
        } else {
            400
        };
        let horiz = if self.mode == Mode::Guide {
            cruise
        } else {
            (cruise * cp) >> 8
        };
        self.vx = (sin_r * horiz) >> 8;
        self.vy = -((cos_r * horiz) >> 8);
        self.x = wrap_coord(self.x + self.vx);
        self.y = wrap_coord(self.y + self.vy);
        // 110 subpixels per frame at a vertical nose. Shallower angles sink less.
        // The spotter's stick tilts the camera and does not climb or dive.
        if self.mode != Mode::Guide || self.falling {
            self.alt -= (110 * sp) >> 8;
        }
    }

    /// Always flies forward. The stick tilts the nose; height and speed follow
    /// that angle, and the nose stays where you leave it.
    /// Kamikaze spends one drone and returns to the pad instead of ending the mission.
    pub(crate) fn fly_plane(&mut self, input: &ButtonController, bg: &mut AffineBackground) -> bool {
        self.age = self.age.wrapping_add(1);
        // Past the radio the stick drops frames, then drops out.
        let loss = self.signal_loss();
        // The stick only flutters in the last part of the radio, and dies at the edge.
        let mute = loss >= 16
            || (loss >= 13 && (hash(self.age, 3) % 16) < (loss as u32 - 12));
        let pitch_in = if mute || self.falling {
            Tri::Zero
        } else {
            input.y_tri()
        };
        let yaw_in = if mute || self.falling {
            Tri::Zero
        } else {
            input.x_tri()
        };
        // A dead link or a fall drops the hover. Select on the quad switches it.
        let fetch = matches!(task_of(self.level), Task::Fetch);
        if self.mode != Mode::Quad || self.falling || loss >= 16 {
            self.hover = false;
        } else if input.is_just_pressed(Button::Select) {
            // Select swaps the glass. On the mine quad the forward glass also climbs.
            self.hover = !self.hover;
            self.sfx |= SFX_MARK;
        }

        // About a second from level to a steep dive. Releasing the stick does
        // not snap the nose back. Hover keeps the nose so leaving it is the same attitude.
        let fetch_fly = fetch && !self.falling && loss < 16;
        if !self.hover && !fetch_fly {
            self.pitch += Num::new(tri_value(pitch_in)) / Num::new(240);
        }
        if self.falling {
            self.pitch += Num::new(1) / Num::new(60);
        }
        if self.pitch > num!(0.20) {
            self.pitch = num!(0.20);
        }
        if self.pitch < num!(-0.12) {
            self.pitch = num!(-0.12);
        }
        // Overhead slides. The forward glass of the mine quad yaws like the others.
        let turning = fetch_fly && !self.hover;
        let yaw_step = if (self.hover || fetch_fly) && !turning {
            Num::new(0)
        } else {
            Num::new(tri_value(yaw_in)) / Num::new(160)
        };
        self.yaw += yaw_step;
        wrap_yaw(&mut self.yaw);
        // The airframe does not roll. The indicator leans into the turn and
        // settles back when the stick is released.
        if !self.hover && (!fetch_fly || turning) {
            let want = yaw_step * Num::new(14);
            self.bank += (want - self.bank) / Num::new(6);
            if self.bank > num!(0.06) {
                self.bank = num!(0.06);
            }
            if self.bank < num!(-0.06) {
                self.bank = num!(-0.06);
            }
        }

        let (sin_r, cos_r) = self.heading();
        if self.hover || fetch_fly {
            self.slide_motion(sin_r, cos_r, pitch_in, yaw_in, fetch_fly, turning);
        } else if fetch && (self.falling || loss >= 16) {
            self.drop_motion();
        } else {
            self.cruise_motion(sin_r, cos_r);
        }
        if self.alt > 40 << 8 {
            self.alt = 40 << 8;
        }
        if !self.cross_gate() {
            return false;
        }

        // Contact is the airframe itself, a few pixels out, so you can fly
        // up to a soldier before the charge goes.
        let touching = self.alt <= 4 << 8 && self.point_blank();
        // A live Fetch quad sits on a round. A dead battery still hits the ground.
        let dug_in = self.alt <= 0 && !(fetch && !self.falling && loss < 16);
        if touching || dug_in {
            if dug_in {
                self.alt = 0;
            }
            if self.mode == Mode::Fpv {
                self.explode(bg, self.x, self.y, 48, 0, KIND_BOMB);
            } else if touching {
                let _ = self.strike_at(bg, self.x, self.y);
            } else {
                self.add_boom(self.x, self.y, 0);
            }
            self.falling = false;
            self.vx = 0;
            self.vy = 0;
            // An empty battery ends the sortie. A crash from the radio still returns to the pad.
            if self.battery == 0 {
                self.fail(Why::Cell);
            } else {
                self.recover = 22;
            }
            return true;
        }
        true
    }

    /// True when the airframe is on top of a soldier or a truck.
    pub(crate) fn point_blank(&self) -> bool {
        let reach = 14 * 14;
        self.soldiers.iter().any(|soldier| {
            soldier.alive && dist2_px(self.x, self.y, soldier.x, soldier.y) <= reach
        }) || self.trucks.iter().any(|truck| {
            truck.alive && dist2_px(self.x, self.y, truck.x, truck.y) <= reach
        })
    }

    /// Where the nose meets the ground. Level flight looks far ahead; a dive
    /// pulls that point in toward the plane.
    pub(crate) fn nose(&self, sin_r: i32, cos_r: i32) -> (i32, i32) {
        let sp = self.pitch.sin().to_raw();
        let cp = self.pitch.cos().to_raw().max(32);
        let alt_px = self.alt >> 8;
        let lead = if sp > 12 {
            (alt_px * cp / sp).clamp(6, 180)
        } else {
            (alt_px * 4).clamp(24, 180)
        };
        (
            wrap_coord(self.x + sin_r * lead),
            wrap_coord(self.y - cos_r * lead),
        )
    }

    /// Whoever stands on the nose point is the kamikaze target.
    pub(crate) fn strike_at(&mut self, bg: &mut AffineBackground, hx: i32, hy: i32) -> bool {
        let reach_man = 28 * 28;
        let reach_truck = 32 * 32;
        let mut man: Option<usize> = None;
        let mut man_d = reach_man + 1;
        for (index, soldier) in self.soldiers.iter().enumerate() {
            if !soldier.alive {
                continue;
            }
            let distance = dist2_px(hx, hy, soldier.x, soldier.y);
            if distance <= reach_man && distance < man_d {
                man = Some(index);
                man_d = distance;
            }
        }
        if let Some(index) = man {
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
            return true;
        }
        let mut truck_hit: Option<(usize, i32, i32)> = None;
        for (index, truck) in self.trucks.iter().enumerate() {
            if truck.alive && dist2_px(hx, hy, truck.x, truck.y) <= reach_truck {
                truck_hit = Some((index, truck.x, truck.y));
                break;
            }
        }
        if let Some((index, x, y)) = truck_hit {
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
            return true;
        }
        false
    }

    pub(crate) fn note_kill(
        &mut self,
        bg: &mut AffineBackground,
        x: i32,
        y: i32,
        points: i32,
        officer: bool,
        truck: bool,
        goal: bool,
        objective: bool,
    ) {
        self.add_boom(x, y, 0);
        self.score += points;
        self.claim(officer, truck, goal, objective);
        let tx = tile_of(x);
        let ty = tile_of(y);
        if self.ground_at(tx, ty) != T_WATER {
            self.paint(bg, tx, ty, T_CRATER);
        }
    }

    pub(crate) fn relaunch(&mut self) -> bool {
        if self.targets <= 0 {
            return true;
        }
        if self.mode == Mode::Gate {
            self.spent = self.spent.saturating_add(1);
        }
        if self.mode == Mode::Fpv {
            self.drones -= 1;
            if self.drones <= 0 {
                self.again = true;
                return false;
            }
        }
        self.x = self.pad_x;
        self.y = self.pad_y;
        self.alt = self.pad_alt;
        self.vx = 0;
        self.vy = 0;
        self.pitch = num!(0);
        self.bank = num!(0);
        self.hull = 100;
        self.battery = 100;
        self.blackout = 0;
        self.falling = false;
        true
    }

    /// Pixels from the pad, cheap distance. The radio is a disc around the pad,
    /// so the generated plain can go on without a flight ever using the far part.
    pub(crate) fn signal_loss(&self) -> i32 {
        let dx = ((self.x - self.pad_x) >> 8).abs();
        let dy = ((self.y - self.pad_y) >> 8).abs();
        let dist = dx.max(dy) + dx.min(dy) / 2;
        // Clear well past the objectives. Snow, then a dead radio, much further out.
        let near = 64 * 8;
        let far = 160 * 8;
        if dist <= near {
            return 0;
        }
        ((dist - near) * 16 / (far - near)).min(16)
    }

    pub(crate) fn on_charger(&self) -> bool {
        if !matches!(task_of(self.level), Task::Fetch) || self.falling || (self.alt >> 8) > 1 {
            return false;
        }
        let tx = tile_of(self.x);
        let ty = tile_of(self.y);
        charge_spots(self.level)
            .iter()
            .any(|&(cx, cy)| (tx - cx).abs() <= 1 && (ty - cy).abs() <= 1)
    }

    pub(crate) fn wear(&mut self) {
        let span = 64 - heat(self.level);
        if self.on_charger() {
            if self.battery < 100 && self.age % 3 == 0 {
                self.battery += 1;
            }
            if self.battery < 100 && self.age % 24 == 0 {
                self.sfx |= SFX_MARK;
            }
        } else if self.age % span == 0 && self.battery > 0 {
            self.battery -= 1;
        }
        let loss = self.signal_loss();
        if !self.on_charger() && loss >= 14 && self.age % 40 == 0 && self.battery > 0 {
            self.battery -= 1;
        }
        if loss >= 16 {
            self.blackout += 1;
        } else if self.blackout > 0 {
            self.blackout -= 2;
            if self.blackout < 0 {
                self.blackout = 0;
            }
        }
        for boom in &self.booms {
            if !boom.alive || boom.frame != 1 {
                continue;
            }
            let distance = dist2_px(self.x, self.y, boom.x, boom.y);
            // The burst is on the ground. Cruise height is above it, so a
            // drop under a hover does not wreck the airframe. A low pass does.
            let dz = ((self.alt - boom.z) >> 8).abs();
            if dz > 8 {
                continue;
            }
            if distance < 20 * 20 {
                self.hull -= 40;
            } else if distance < 36 * 36 {
                self.hull -= 16;
            }
        }
        if self.hull < 0 {
            self.hull = 0;
        }
        if self.hull == 0 && self.recover == 0 {
            if self.mode == Mode::Fpv {
                self.recover = 22;
                self.vx = 0;
                self.vy = 0;
            } else {
                self.fail(Why::Hull);
            }
            return;
        }
        // A dead radio or a dead battery drops the nose. The airframe falls
        // until it hits. An empty battery ends the sortie there.
        let was_falling = self.falling;
        let loss = self.signal_loss();
        if self.recover == 0 && (loss >= 16 || self.battery == 0) {
            self.falling = true;
        }
        if self.falling && !was_falling {
            self.sfx |= SFX_FALL;
        }
        // The feed crackles as the radio thins out.
        if loss >= 6 {
            let gap = (20 - loss).clamp(4, 14) * 8;
            if self.age % gap == 0 {
                self.sfx |= SFX_LINK;
            }
        }
    }
}
