use null_bot_api::fake::{FakeCommands, FakeView};
use null_bot_api::reflex::Reflex;
use null_bot_api::*;
use null_combat::*;

fn rules() -> FightRules {
    let mut r = FightRules::new(TargetSelector::PlayerNames(vec!["foe".into()]));
    r.never_names = vec!["mate".into()];
    r
}

fn player(id: u32, name: &str, x: f64, z: f64) -> EntitySnapshot {
    let mut e = EntitySnapshot::new(id, "minecraft:player", Vec3::new(x, 64.0, z));
    e.name = Some(name.into());
    e.health = Some(20.0);
    e
}

fn armed(foe_z: f64) -> FakeView {
    let mut v = FakeView::default();
    v.hotbar[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
    v.entities.push(player(1, "foe", 0.0, foe_z));
    v
}

fn tick(v: &mut FakeView, c: &mut FakeCommands, t: u64) {
    v.tick = t;
    c.set_tick(t);
}

fn gotos(c: &FakeCommands, t: u64) -> bool {
    c.at_tick(t).iter().any(|c| matches!(c, Command::Goto { .. }))
}

/// A trunk across the line at head height.
const TRUNK: BlockPos = BlockPos::new(0, 65, 3);

#[test]
fn melee_to_path_after_ten_blocked_ticks_and_back() {
    let mut v = armed(6.0);
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    tick(&mut v, &mut c, 0);
    e.step(&v, &mut c);
    assert!(c.at_tick(0).contains(&Command::StopNav) && !gotos(&c, 0), "melee starts");
    v.solid.push(TRUNK);
    for t in 1..=9 {
        tick(&mut v, &mut c, t);
        e.step(&v, &mut c);
        assert!(!gotos(&c, t), "still in melee at blocked tick {t}");
    }
    tick(&mut v, &mut c, 10);
    e.step(&v, &mut c);
    assert!(gotos(&c, 10), "10th blocked tick hands over to the pathfinder");
    v.solid.clear();
    tick(&mut v, &mut c, 11);
    e.step(&v, &mut c);
    assert!(c.at_tick(11).contains(&Command::StopNav), "clear line: melee again");
    assert!(!gotos(&c, 11));
    assert_eq!(e.fights, 2);
}

#[test]
fn one_blocked_tick_does_not_leave_melee() {
    let mut v = armed(6.0);
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    for t in 0..30 {
        tick(&mut v, &mut c, t);
        if t == 5 {
            v.solid.push(TRUNK);
        } else {
            v.solid.clear();
        }
        e.step(&v, &mut c);
    }
    assert!(!c.all().iter().any(|c| matches!(c, Command::Goto { .. })));
    assert_eq!(e.fights, 1);
}

#[test]
fn paths_to_a_far_enemy_and_holds_without_a_weapon() {
    let mut v = armed(30.0);
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    v.hotbar[0] = None;
    e.step(&v, &mut c);
    assert!(c.all().is_empty());
    v.hotbar[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
    tick(&mut v, &mut c, 1);
    e.step(&v, &mut c);
    assert!(gotos(&c, 1));
}

fn low_hp(foe_z: f64, mate: Option<(f64, f64)>) -> FakeView {
    let mut v = armed(foe_z);
    v.health = 4.0;
    if let Some((x, z)) = mate {
        v.entities.push(player(2, "mate", x, z));
    }
    v
}

#[test]
fn retreat_runs_to_a_teammate() {
    let v = low_hp(6.0, Some((0.0, -20.0)));
    let (mut c, mut r) = (FakeCommands::default(), Retreat::new(rules()));
    assert!(r.trigger(&v));
    r.act(&v, &mut c);
    let goto = c.all().into_iter().find_map(|c| match c {
        Command::Goto { target, .. } => Some(target),
        _ => None,
    });
    assert_eq!(goto.map(|g| g.z), Some(-20));
}

#[test]
fn retreat_skips_a_teammate_behind_the_threat() {
    let mut v = low_hp(6.0, Some((0.0, 20.0)));
    let (mut c, mut r) = (FakeCommands::default(), Retreat::new(rules()));
    assert!(r.trigger(&v));
    r.act(&v, &mut c);
    assert!(!gotos(&c, 0), "no goto toward the far side of the threat");
    assert!(c.all().contains(&Command::Walk { forward: 1, strafe: 0 }), "runs away instead");
    v.health = 20.0;
}

#[test]
fn retreat_hysteresis_and_caught() {
    let mut v = low_hp(6.0, Some((0.0, -20.0)));
    let mut r = Retreat::new(rules());
    assert!(r.trigger(&v));
    v.health = 10.0;
    assert!(r.trigger(&v), "keeps going until resume_health");
    v.entities[0].position.z = 2.0;
    assert!(!r.trigger(&v), "caught: engage fights it out");
    v.entities[0].position.z = 6.0;
    assert!(!r.trigger(&v), "hysteresis needs a firing tick just before; at 10 it stays off");
    v.health = 4.0;
    assert!(r.trigger(&v), "low health fires again");
    v.health = 15.0;
    assert!(!r.trigger(&v), "healed past resume_health");
}

#[test]
fn retreat_gives_up_when_health_never_rises() {
    let mut v = low_hp(6.0, Some((0.0, -20.0)));
    let mut r = Retreat::new(rules());
    let mut last = true;
    for t in 0..=210 {
        v.tick = t;
        last = r.trigger(&v);
        if !last {
            assert!(t >= 200, "hopeless after 200 ticks, not at {t}");
            break;
        }
    }
    assert!(!last);
    v.tick = 300;
    assert!(!r.trigger(&v));
    v.health = 5.0;
    v.tick = 301;
    assert!(r.trigger(&v), "a rise in health clears it");
}

#[test]
fn reflex_preempts_engage_and_engage_resumes() {
    let mut v = low_hp(6.0, Some((0.0, -20.0)));
    let (mut c, mut brain) = (FakeCommands::default(), Brain::new());
    brain.register(Box::new(Retreat::new(rules())));
    let mut e = Engage::new(rules(), (0.0, 0.0), 1);
    for t in 0..3 {
        tick(&mut v, &mut c, t);
        let o = brain.tick(&v, &mut c, Some(&mut e));
        assert_eq!((o.reflex, o.skill), (Some("combat.flee_low_hp"), None));
    }
    v.health = 20.0;
    tick(&mut v, &mut c, 3);
    let o = brain.tick(&v, &mut c, Some(&mut e));
    assert_eq!((o.reflex, o.skill), (None, Some(SkillStatus::Running)));
    assert!(c.at_tick(3).contains(&Command::StopNav) || gotos(&c, 3) || !c.at_tick(3).is_empty());
}

#[test]
fn retreat_never_rallies_to_its_own_copy() {
    // A swarm's shared world can show a bot its own player entity.
    let mut v = low_hp(6.0, None);
    v.entities.push(player(3, "me", 0.0, 0.5));
    let mut rs = rules();
    rs.never_names.push("me".into());
    let (mut c, mut r) = (FakeCommands::default(), Retreat::new(rs).with_self_name("me"));
    assert!(r.trigger(&v));
    r.act(&v, &mut c);
    assert!(!gotos(&c, 0), "no rally to itself");
    assert!(c.all().contains(&Command::Walk { forward: 1, strafe: 0 }), "runs away instead");
}

#[test]
fn retreat_needs_a_threat_or_a_teammate() {
    let mut v = low_hp(6.0, None);
    v.entities.clear();
    let mut r = Retreat::new(rules());
    assert!(!r.trigger(&v), "nothing to run from or to: engage acts");
    v.entities.push(player(2, "mate", 0.0, -20.0));
    assert!(r.trigger(&v), "a teammate to regroup with");
}

#[test]
fn retreat_skips_a_hurt_teammate() {
    let mut v = low_hp(40.0, None);
    let mut mate = player(2, "mate", 0.0, -5.0);
    mate.health = Some(5.0);
    v.entities.push(mate);
    let mut r = Retreat::new(rules());
    assert!(!r.trigger(&v), "a hurt teammate is no rally point, and no threat is near");
}

#[test]
fn stuck_goto_is_force_stopped_and_replanned() {
    // The pathfinder says Following but the bot does not move (stuck on a node).
    let mut v = armed(30.0);
    v.nav_status = NavStatus::Following;
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    for t in 0..=45 {
        tick(&mut v, &mut c, t);
        e.step(&v, &mut c);
    }
    let stop = (1..=45).find(|&t| c.at_tick(t).contains(&Command::Stop)).expect("force stop without progress");
    assert!(gotos(&c, stop), "and a fresh goto the same tick");
}

#[test]
fn drop_worth_needs_survival_and_a_lead() {
    // 5-block fall: 2 damage. 10 HP lands at 8.
    assert!(drop_worth(10.0, Some(5.0), 5));
    assert!(!drop_worth(10.0, Some(9.0), 5), "enemy would be healthier than us after the fall");
    assert!(!drop_worth(2.0, Some(1.0), 5), "the fall kills us");
    assert!(!drop_worth(20.0, None, 5), "unknown enemy health");
    assert!(drop_worth(6.0, Some(5.0), 3), "3 blocks is a free fall");
}

#[test]
fn fighter_drops_onto_a_weaker_enemy_below() {
    // We stand at y 64 on a ledge; the enemy is 5 blocks down, just past the edge.
    let mut v = FakeView::default();
    v.hotbar[0] = Some(ItemStack::new("minecraft:iron_sword", 1));
    v.position = Vec3::new(0.5, 64.0, 0.5);
    v.health = 18.0;
    v.on_ground = true;
    for x in -2..=2 {
        v.solid.push(BlockPos::new(x, 63, 0));
        for z in 1..=4 {
            v.solid.push(BlockPos::new(x, 58, z));
        }
    }
    let mut foe = player(1, "foe", 0.5, 2.5);
    foe.position.y = 59.0;
    foe.health = Some(6.0);
    v.entities.push(foe);
    let mut f = FightSkill::new(rules(), Box::new(MeleeBasic::default()), 1);
    let mut c = FakeCommands::default();
    f.step(&v, &mut c);
    assert!(f.last_drop_in, "worth it: lands at 17 HP against 6");
    assert!(c.all().contains(&Command::Walk { forward: 1, strafe: 0 }));
    v.entities[0].health = Some(18.0);
    let (mut f, mut c) = (FightSkill::new(rules(), Box::new(MeleeBasic::default()), 1), FakeCommands::default());
    f.step(&v, &mut c);
    assert!(!f.last_drop_in, "not worth it: lands at 17 HP against 18");
    assert!(!c.all().contains(&Command::Walk { forward: 1, strafe: 0 }), "does not step toward the edge");
}

#[test]
fn a_path_search_is_never_interrupted() {
    // Azalea searches for up to 5 s; stopping or re-aiming restarts it, so a moving target was never reached.
    let mut v = armed(30.0);
    v.nav_status = NavStatus::Calculating;
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    tick(&mut v, &mut c, 0);
    e.step(&v, &mut c);
    assert!(gotos(&c, 0));
    for t in 1..=105 {
        tick(&mut v, &mut c, t);
        v.entities[0].position.z = 30.0 + t as f64 * 0.2; // the target keeps moving
        e.step(&v, &mut c);
        assert!(!gotos(&c, t) && !c.at_tick(t).contains(&Command::Stop), "search interrupted at tick {t}");
    }
}

#[test]
fn hopeless_clock_runs_while_the_threat_comes_and_goes() {
    // An enemy hovering at the threat edge used to restart the clock each time the reflex stopped.
    let mut v = low_hp(6.0, None);
    let mut r = Retreat::new(rules());
    let mut fired_late = false;
    for t in 0..=260 {
        v.tick = t;
        v.entities[0].position.z = if (t / 15) % 2 == 0 { 6.0 } else { 30.0 };
        let f = r.trigger(&v);
        if t > 205 {
            fired_late |= f;
        }
    }
    assert!(!fired_late, "hopeless after 200 ticks without a health rise, however often it stopped");
}

#[test]
fn heads_for_the_target_while_the_path_search_runs() {
    let mut v = armed(12.0);
    for x in -2..=2 {
        for z in -2..=14 {
            v.solid.push(BlockPos::new(x, 63, z));
        }
    }
    v.position = Vec3::new(0.5, 64.0, 0.5);
    v.on_ground = true;
    v.nav_status = NavStatus::Calculating;
    let (mut c, mut e) = (FakeCommands::default(), Engage::new(rules(), (0.0, 0.0), 1));
    tick(&mut v, &mut c, 1);
    e.step(&v, &mut c);
    assert!(c.at_tick(1).contains(&Command::Walk { forward: 1, strafe: 0 }), "not standing still while searching");
}
