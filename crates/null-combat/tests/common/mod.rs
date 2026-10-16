#![allow(dead_code)]
use null_bot_api::fake::FakeView;
use null_bot_api::*;
use null_combat::*;

pub const TARGET: EntityId = EntityId(1);

/// Bot at the origin, a player target `dist` blocks away along +z.
pub fn view(dist: f64) -> FakeView {
    let mut v = FakeView::default();
    v.entities.push(EntitySnapshot::new(1, "minecraft:player", Vec3::new(0.0, 64.0, dist)));
    v
}

pub fn obs(v: &FakeView) -> DuelObs {
    observe(v, TARGET).expect("target present")
}

pub fn act(s: DuelStep) -> DuelAction {
    match s {
        DuelStep::Act(a) => a,
        DuelStep::Done(r) => panic!("unexpected done {r:?}"),
    }
}

pub fn melee_with(edit: &[(&str, f64)]) -> MeleeBasic {
    let mut p = MELEE_BASIC_SPACE.defaults();
    for (k, x) in edit {
        assert!(p.set(k, *x), "{k}");
    }
    let mut c = MeleeBasic::new(&p).unwrap();
    c.reset(1);
    c
}

/// Attacks every tick; lets tests check what the clamps and filters do on their own.
pub struct Spam;
impl DuelController for Spam {
    fn id(&self) -> &str {
        "spam"
    }
    fn reset(&mut self, _: u64) {}
    fn step(&mut self, o: &DuelObs) -> DuelStep {
        DuelStep::Act(DuelAction {
            yaw: o.target.face.yaw,
            pitch: o.target.face.pitch,
            attack: true,
            ..Default::default()
        })
    }
}

pub fn player_rules() -> FightRules {
    let mut r = FightRules::new(TargetSelector::Entities(vec![TARGET]));
    r.attack_player_first_allowed = true;
    r
}
