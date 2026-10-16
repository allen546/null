//! Demo decision code for the S4 skeleton: walk out to an assigned point and back, twice.
//! Written against `BotView` and `BotCommands` only, so it builds on stable.

use null_bot_api::{BlockPos, BotCommands, BotView, NavStatus, Skill, SkillStatus, Vec3};

pub struct PatrolConfig {
    /// This bot's slot on the ring and the number of slots; angle = 2pi * index / count.
    pub index: usize,
    pub count: usize,
    /// Ring centre (x, z). None means "where this bot first stands".
    pub center: Option<(f64, f64)>,
    pub radius_min: f64,
    pub radius_max: f64,
    pub round_trips: u32,
    pub tolerance: f32,
    /// Give up on one leg after this many ticks.
    pub leg_timeout_ticks: u64,
    /// No progress toward the target for this long counts as stuck.
    pub stuck_ticks: u64,
}

impl PatrolConfig {
    pub fn new(index: usize, count: usize) -> Self {
        Self {
            index,
            count: count.max(1),
            center: None,
            radius_min: 20.0,
            radius_max: 30.0,
            round_trips: 2,
            tolerance: 1.5,
            leg_timeout_ticks: 20 * 120,
            stuck_ticks: 20 * 12,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Phase {
    /// Let chunks load before looking at blocks.
    Warmup(u32),
    /// Heading to `legs[leg]`; `issued` is the tick of the last goto.
    Moving,
}

pub struct Patrol {
    cfg: PatrolConfig,
    phase: Phase,
    home: Option<BlockPos>,
    /// Alternating out, home, out, home...; filled once the surface heights are known.
    legs: Vec<BlockPos>,
    leg: usize,
    /// Ring centre once known, and how many times the outbound point was moved aside.
    centre: (f64, f64),
    shifts: u32,
    /// Tick of the last goto, tick the current leg began, retries on this leg.
    issued: u64,
    started: u64,
    retries: u32,
    /// Closest the bot has got to the current target and when; a stalled bot is "stuck".
    best_dist: f64,
    best_tick: u64,
}

impl Patrol {
    pub fn new(cfg: PatrolConfig) -> Self {
        Self { cfg, phase: Phase::Warmup(40), home: None, legs: Vec::new(), leg: 0, centre: (0.0, 0.0), shifts: 0, issued: 0, started: 0, retries: 0, best_dist: f64::MAX, best_tick: 0 }
    }

    /// The ring point for this bot (x, z), unique per index.
    pub fn ring_point(&self, center: (f64, f64)) -> (f64, f64) {
        self.ring_point_shifted(center, 0)
    }

    /// Each shift moves the point 0.15 rad along the ring (about 3-4 blocks).
    fn ring_point_shifted(&self, center: (f64, f64), shift: u32) -> (f64, f64) {
        let n = self.cfg.count as f64;
        let angle = std::f64::consts::TAU * self.cfg.index as f64 / n + 0.15 * shift as f64;
        // Three radius bands so neighbours on the ring are not shoulder to shoulder.
        let band = (self.cfg.index % 3) as f64 / 2.0;
        let r = self.cfg.radius_min + (self.cfg.radius_max - self.cfg.radius_min) * band;
        (center.0 + r * angle.cos(), center.1 + r * angle.sin())
    }

    /// Standing position above the highest solid block with two free blocks over it. The block
    /// must sit on four more solid ones: that rejects tree canopies and overhangs, which the
    /// view cannot tell from ground (it only knows "solid").
    fn surface(view: &dyn BotView, x: i32, z: i32, around_y: i32) -> Option<BlockPos> {
        for y in ((around_y - 40)..=(around_y + 40)).rev() {
            if view.is_solid(BlockPos::new(x, y, z))
                && !view.is_solid(BlockPos::new(x, y + 1, z))
                && !view.is_solid(BlockPos::new(x, y + 2, z))
            {
                if (1..=4).any(|d| !view.is_solid(BlockPos::new(x, y - d, z))) {
                    continue;
                }
                return Some(BlockPos::new(x, y + 1, z));
            }
        }
        None
    }

    fn center_of(p: BlockPos) -> Vec3 {
        Vec3::new(p.x as f64 + 0.5, p.y as f64, p.z as f64 + 0.5)
    }
}

impl Skill for Patrol {
    fn id(&self) -> &str {
        "patrol"
    }

    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        let now = view.tick();
        let pos = view.position();
        if let Phase::Warmup(n) = &mut self.phase {
            if *n > 0 {
                *n -= 1;
                return SkillStatus::Running;
            }
        }
        match self.phase {
            Phase::Warmup(_) => {
                let home = pos.block_pos();
                let c = self.cfg.center.unwrap_or((pos.x, pos.z));
                self.centre = c;
                // A bad column (open water, a tree trunk) moves the point along the ring.
                let found = (0..=8).find_map(|shift| {
                    let (x, z) = self.ring_point_shifted(c, shift);
                    Self::surface(view, x.floor() as i32, z.floor() as i32, home.y).map(|o| (shift, o))
                });
                let Some((shift, out)) = found else {
                    return SkillStatus::Failed("no standable surface near the assigned point".into());
                };
                self.shifts = shift;
                self.home = Some(home);
                for _ in 0..self.cfg.round_trips {
                    self.legs.push(out);
                    self.legs.push(home);
                }
                cmds.goto(self.legs[0], self.cfg.tolerance);
                self.phase = Phase::Moving;
                self.issued = now;
                self.started = now;
                self.retries = 0;
                self.best_dist = f64::MAX;
                self.best_tick = now;
                SkillStatus::Running
            }
            Phase::Moving => {
                let target = self.legs[self.leg];
                let arrived = pos.distance(Self::center_of(target)) <= self.cfg.tolerance as f64 + 1.0
                    && view.on_ground();
                if arrived {
                    self.leg += 1;
                    if self.leg == self.legs.len() {
                        return SkillStatus::Done;
                    }
                    cmds.goto(self.legs[self.leg], self.cfg.tolerance);
                    self.issued = now;
                    self.started = now;
                    self.retries = 0;
                    self.best_dist = f64::MAX;
                    self.best_tick = now;
                } else if now - self.started > self.cfg.leg_timeout_ticks {
                    return SkillStatus::Failed(format!("leg {} timed out", self.leg));
                } else if {
                    let d = pos.distance(Self::center_of(target));
                    if d < self.best_dist - 1.0 {
                        self.best_dist = d;
                        self.best_tick = now;
                    }
                    let stuck = now - self.best_tick > self.cfg.stuck_ticks;
                    if stuck {
                        cmds.stop_nav();
                        self.best_tick = now;
                    }
                    stuck || (view.nav_status() == NavStatus::Idle && now - self.issued > 10)
                } {
                    // The pathfinder gave up, stopped short or made no progress; ask again, and
                    // after a few tries move the outbound point aside (water, a cliff, a canopy).
                    if self.retries >= 3 && self.leg % 2 == 0 && self.shifts < 8 {
                        self.shifts += 1;
                        let (x, z) = self.ring_point_shifted(self.centre, self.shifts);
                        let home = self.home.unwrap_or_default();
                        if let Some(out) = Self::surface(view, x.floor() as i32, z.floor() as i32, home.y) {
                            for i in (0..self.legs.len()).step_by(2) {
                                self.legs[i] = out;
                            }
                            self.retries = 0;
                            cmds.goto(out, self.cfg.tolerance);
                            self.issued = now;
                        }
                        return SkillStatus::Running;
                    }
                    if self.retries >= 5 {
                        return SkillStatus::Failed(format!("leg {} not reachable", self.leg));
                    }
                    self.retries += 1;
                    cmds.goto(target, self.cfg.tolerance);
                    self.issued = now;
                }
                SkillStatus::Running
            }
        }
    }
}

/// Walk to one block and finish; the S4 "walk to a given point" check.
pub struct GotoPoint {
    target: BlockPos,
    tolerance: f32,
    issued: Option<u64>,
    started: u64,
    timeout_ticks: u64,
}

impl GotoPoint {
    pub fn new(target: BlockPos) -> Self {
        Self { target, tolerance: 1.5, issued: None, started: 0, timeout_ticks: 20 * 120 }
    }
}

impl Skill for GotoPoint {
    fn id(&self) -> &str {
        "goto_point"
    }
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        let now = view.tick();
        let here = view.position();
        let goal = Patrol::center_of(self.target);
        if here.distance(goal) <= self.tolerance as f64 + 1.0 && view.on_ground() {
            return SkillStatus::Done;
        }
        match self.issued {
            None => {
                self.started = now;
                self.issued = Some(now);
                cmds.goto(self.target, self.tolerance);
            }
            Some(_) if now - self.started > self.timeout_ticks => {
                return SkillStatus::Failed("goto timed out".into());
            }
            Some(t) if view.nav_status() == NavStatus::Idle && now - t > 10 => {
                self.issued = Some(now);
                cmds.goto(self.target, self.tolerance);
            }
            _ => {}
        }
        SkillStatus::Running
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use null_bot_api::fake::{FakeCommands, FakeView};
    use null_bot_api::Command;

    fn gotos(c: &FakeCommands) -> Vec<BlockPos> {
        c.all()
            .into_iter()
            .filter_map(|c| if let Command::Goto { target, .. } = c { Some(target) } else { None })
            .collect()
    }

    #[test]
    fn goto_point_finishes_on_arrival() {
        let mut g = GotoPoint::new(BlockPos::new(5, 64, 5));
        let mut v = FakeView::default();
        let mut c = FakeCommands::default();
        assert_eq!(g.step(&v, &mut c), SkillStatus::Running);
        assert_eq!(c.all(), vec![Command::Goto { target: BlockPos::new(5, 64, 5), tolerance: 1.5 }]);
        v.position = Vec3::new(5.5, 64.0, 5.5);
        assert_eq!(g.step(&v, &mut c), SkillStatus::Done);
    }

    #[test]
    fn ring_points_are_unique_and_in_band() {
        let ps: Vec<_> = (0..20)
            .map(|i| Patrol::new(PatrolConfig::new(i, 20)).ring_point((0.0, 0.0)))
            .collect();
        for (i, a) in ps.iter().enumerate() {
            let r = (a.0 * a.0 + a.1 * a.1).sqrt();
            assert!((19.99..=30.01).contains(&r));
            for b in &ps[i + 1..] {
                assert!((a.0 - b.0).hypot(a.1 - b.1) > 5.0, "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn walks_out_and_back_twice_then_done() {
        let mut p = Patrol::new(PatrolConfig::new(0, 4));
        let mut v = FakeView::default(); // flat floor at y=64, bot stands at y=64
        let mut c = FakeCommands::default();
        let mut status = SkillStatus::Running;
        for t in 0..400 {
            v.tick = t;
            c.set_tick(t);
            status = p.step(&v, &mut c);
            // Teleport to the latest goto target to stand in for the pathfinder.
            if let Some(g) = gotos(&c).last() {
                v.position = Vec3::new(g.x as f64 + 0.5, g.y as f64, g.z as f64 + 0.5);
            }
            if status != SkillStatus::Running {
                break;
            }
        }
        assert_eq!(status, SkillStatus::Done);
        let g = gotos(&c);
        assert_eq!(g.len(), 4);
        assert_eq!(g[0], g[2]);
        assert_eq!(g[1], g[3]);
        assert_eq!(g[1], BlockPos::new(0, 64, 0));
        assert_eq!(g[0], BlockPos::new(20, 64, 0));
    }

    #[test]
    fn reissues_goto_when_navigation_stops_short() {
        let mut p = Patrol::new(PatrolConfig::new(0, 4));
        let v0 = FakeView::default();
        let mut c = FakeCommands::default();
        let mut v = v0;
        for t in 0..200 {
            v.tick = t;
            c.set_tick(t);
            p.step(&v, &mut c); // never moves, nav idle
        }
        assert!(gotos(&c).len() > 1);
    }

    #[test]
    fn fails_after_retries() {
        let mut p = Patrol::new(PatrolConfig::new(0, 4));
        let mut v = FakeView::default();
        let mut c = FakeCommands::default();
        let mut last = SkillStatus::Running;
        for t in 0..3000 {
            v.tick = t;
            c.set_tick(t);
            last = p.step(&v, &mut c);
        }
        assert!(matches!(last, SkillStatus::Failed(_)));
        // The outbound point was moved aside before giving up.
        let mut outs = gotos(&c);
        outs.dedup();
        assert!(outs.len() > 2, "{outs:?}");
    }

    #[test]
    fn stalled_navigation_moves_the_target_aside() {
        let mut p = Patrol::new(PatrolConfig::new(0, 4));
        let mut v = FakeView::default();
        v.nav_status = NavStatus::Following; // pathfinder keeps "working" and never arrives
        let mut c = FakeCommands::default();
        for t in 0..1500 {
            v.tick = t;
            c.set_tick(t);
            p.step(&v, &mut c);
        }
        let mut outs = gotos(&c);
        outs.dedup();
        assert!(outs.len() > 1, "{outs:?}");
        assert!(c.all().contains(&Command::StopNav));
    }

    #[test]
    fn no_surface_fails() {
        let mut p = Patrol::new(PatrolConfig::new(0, 4));
        let mut v = FakeView::default();
        v.floor_y = -1000; // nothing solid in scan range
        let mut c = FakeCommands::default();
        let mut last = SkillStatus::Running;
        for t in 0..60 {
            v.tick = t;
            last = p.step(&v, &mut c);
        }
        assert!(matches!(last, SkillStatus::Failed(_)));
    }
}
