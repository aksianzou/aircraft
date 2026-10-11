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
        let skip = (alt_px * FLOOR_ZOOM * 25 / REACH).max(12);
        let buried = -skip;
        let start = 13; // 0.05 turn
        let span = 26; // the next 0.10 turn buries the horizon
        let mix = ((self.pitch.to_raw() - start) * 256 / span).clamp(0, 256);
        (level + (buried - level) * mix / 256).clamp(-140, 108)
    }

    pub(crate) fn fly(&mut self, input: &ButtonController, bg: &mut AffineBackground) {
        if self.bloom > 16 {
            if self.age & 1 == 0 {
                self.bloom -= 1;
            }
        } else if self.bloom > 0 {
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
                self.note_drill_drop();
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
        self.update_burns(bg);
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
        self.advance_drill();
        self.wear();
    }

    /// Hovering over the practice mark is the first step. The drop is the second.
    fn advance_drill(&mut self) {
        if !matches!(task_of(self.level), Task::Gate) || self.lesson != 0 || !self.hover {
            return;
        }
        let (mx, my) = self.drill_mark();
        if dist2_px(self.x, self.y, mx, my) <= 24 * 24 {
            self.lesson = 1;
        }
    }

    fn note_drill_drop(&mut self) {
        if !matches!(task_of(self.level), Task::Gate) || self.lesson >= 2 || !self.hover {
            return;
        }
        let (mx, my) = self.drill_mark();
        if dist2_px(self.x, self.y, mx, my) <= 24 * 24 {
            self.lesson = 2;
        }
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
        let defend_sit = defend_card(self.level) && self.hover;
        let slide_pitch = if turning || defend_sit { Tri::Zero } else { pitch_in };
        let slide_yaw = if turning || defend_sit { Tri::Zero } else { yaw_in };
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
        let defend_sit = defend_card(self.level) && self.hover;
        if (!self.hover && !fetch_fly) || defend_sit {
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
        // On the pad the gun turns and nods without sliding off it.
        let turning = fetch_fly && !self.hover;
        let yaw_step = if (self.hover || fetch_fly) && !turning && !defend_sit {
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
        // up to a soldier before the charge goes. A pine is solid up to its crown.
        let touching = self.alt <= 4 << 8 && self.point_blank();
        let crowned = self.in_crown();
        // A live Fetch quad sits on a round. A dead battery still hits the ground.
        let dug_in = self.alt <= 0 && !(fetch && !self.falling && loss < 16);
        if touching || dug_in || crowned {
            if dug_in {
                self.alt = 0;
            }
            self.falling = false;
            self.vx = 0;
            self.vy = 0;
            if self.mode == Mode::Fpv {
                self.explode(bg, self.x, self.y, 48, 0, KIND_BOMB);
                // The next airframe leaves the pad, until the last one is spent.
                if self.battery == 0 {
                    self.fail(Why::Cell);
                } else {
                    self.recover = 22;
                }
                return true;
            }
            if touching {
                let _ = self.strike_at(bg, self.x, self.y);
            } else {
                self.add_boom(self.x, self.y, 0, 0);
            }
            // One airframe. The ram is a win only when that kill clears the card.
            if self.targets > 0 {
                if self.battery == 0 {
                    self.fail(Why::Cell);
                } else {
                    self.fail(Why::Hull);
                }
            }
            return true;
        }
        true
    }

    /// True when the airframe is down in a pine. Cruise height clears the crown.
    fn in_crown(&self) -> bool {
        let alt = self.alt >> 8;
        if alt <= 0 || alt > TREE_TOP {
            return false;
        }
        self.ground_at(tile_of(self.x), tile_of(self.y)) == T_TREE
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
            let dir = self.trucks[index].dir;
            let vertical = self.trucks[index].vertical;
            self.trucks[index].alive = false;
            self.leave_husk(x, y, dir, vertical, HUSK_TRUCK);
            self.fling(x, y, false);
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
        self.add_boom(x, y, 0, 0);
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
        // Forward flight spends the pack. A hover, and the mine quad sitting
        // on a pad, spend less. The span is taken from this card's clock, so a
        // full sortie in the air still finishes with the battery star in reach.
        let flight_span = (time_budget(self.level) / 65).max(16);
        let span = if self.hover {
            flight_span * 3 / 2
        } else {
            flight_span
        };
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
            // The burst is on the ground. A bombing pass stays above it.
            // Only a drone sitting in the fire, ALT 03 and under, is hit,
            // and one blast does not finish a full airframe.
            let dz = ((self.alt - boom.z) >> 8).abs();
            if dz > 3 {
                continue;
            }
            if distance < 16 * 16 {
                self.hull -= 28;
            } else if distance < 28 * 28 {
                self.hull -= 12;
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
