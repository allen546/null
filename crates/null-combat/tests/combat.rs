mod common;
use common::*;
use null_bot_api::fake::{FakeCommands, FakeView};
use null_bot_api::*;
use null_combat::*;

fn attacks(c: &FakeCommands) -> Vec<u64> {
    c.log.iter().filter(|(_, c)| matches!(c, Command::Attack(_))).map(|(t, _)| *t).collect()
}

fn run(skill: &mut FightSkill, v: &mut FakeView, c: &mut FakeCommands, ticks: u64) -> SkillStatus {
    let mut st = SkillStatus::Running;
    for t in 0..ticks {
        v.tick = t;
        c.set_tick(t);
        st = skill.step(v, c);
    }
    st
}

#[test]
fn field_lists_match_vectors() {
    let v = view(3.0);
    assert_eq!(obs(&v).to_vec().len(), OBS_FIELDS.len());
    assert_eq!(DuelAction::default().to_vec().len(), ACTION_FIELDS.len());
    let mut a = DuelAction::default();
    a.select_slot = Some(4);
    assert_eq!(a.to_vec()[ACTION_FIELDS.len() - 1], 4.0);
}

#[test]
fn observe_geometry() {
    let v = view(5.0);
    let o = obs(&v);
    assert!((o.target.face.yaw - 0.0).abs() < 1e-4 && o.target.face.pitch.abs() < 1e-4);
    assert!((o.target.horizontal_distance - 5.0).abs() < 1e-9);
    let mut v = FakeView::default();
    v.entities.push(EntitySnapshot::new(1, "minecraft:player", Vec3::new(4.0, 64.0, 0.0)));
    assert!((obs(&v).target.face.yaw + 90.0).abs() < 1e-4);
    assert!(observe(&v, EntityId(99)).is_none());
}

#[test]
fn approaches_to_spacing() {
    let mut v = view(8.0);
    let mut c = melee_with(&[("crit", 0.0), ("shield_react", 0.0)]);
    let first = act(c.step(&obs(&v)));
    assert_eq!((first.forward, first.sprint), (1, true));
    for _ in 0..80 {
        let a = act(c.step(&obs(&v)));
        v.position.z += a.forward as f64 * 0.25;
        v.attack_cooldown = 1.0;
    }
    let d = obs(&v).target.horizontal_distance;
    assert!((d - 2.6).abs() < 0.5, "settled at {d}");
}

#[test]
fn never_attacks_out_of_reach() {
    let mut v = view(4.5);
    let mut c = FakeCommands::default();
    let mut s = FightSkill::new(player_rules(), Box::new(Spam), 1);
    run(&mut s, &mut v, &mut c, 30);
    assert!(attacks(&c).is_empty());
    assert_eq!(s.clamp_totals().attacks_dropped_reach, 30);
    let mut clamps = Clamps::from_mechanics();
    let mut a = DuelAction::default();
    a.attack = true;
    let (out, rep) = clamps.apply(0, &obs(&v), a);
    assert!(!out.attack && rep.attacks_dropped_reach == 1);
}

#[test]
fn click_rate_holds_over_100_ticks() {
    let mut v = view(2.0);
    let mut c = FakeCommands::default();
    let mut s = FightSkill::new(player_rules(), Box::new(Spam), 1);
    run(&mut s, &mut v, &mut c, 100);
    let a = attacks(&c);
    assert!(a.len() > 50, "should still click a lot: {}", a.len());
    for start in 0..=80u64 {
        let n = a.iter().filter(|&&t| t >= start && t < start + 20).count();
        assert!(n <= 14, "{n} clicks in window at {start}");
    }
    assert!(s.clamp_totals().attacks_dropped_rate > 0);
}

#[test]
fn turn_clamp_and_wraparound() {
    let v = view(2.0);
    let mut o = obs(&v);
    let mut clamps = Clamps::from_mechanics();
    let mut want = |cur: f32, yaw: f32, pitch: f32| {
        o.look = Look { yaw: cur, pitch: 0.0 };
        let a = DuelAction { yaw, pitch, ..Default::default() };
        clamps.apply(0, &o, a)
    };
    // 170 -> -170 is a 20 degree turn the short way: allowed whole.
    let (a, r) = want(170.0, -170.0, 0.0);
    assert!((wrap_deg(a.yaw - -170.0)).abs() < 1e-3 && r.looks_limited == 0);
    // 170 -> 0 is -170: limited to 60 toward it.
    let (a, r) = want(170.0, 0.0, 0.0);
    assert!((a.yaw - 110.0).abs() < 1e-3 && r.looks_limited == 1);
    // 170 -> -100 is +90 across the seam: limited to +60, lands at -130.
    let (a, _) = want(170.0, -100.0, 0.0);
    assert!((wrap_deg(a.yaw - -130.0)).abs() < 1e-3, "{}", a.yaw);
    let (a, r) = want(0.0, 0.0, 80.0);
    assert!((a.pitch - 60.0).abs() < 1e-3 && r.looks_limited == 1);
    assert_eq!(clamps.totals().looks_limited, 3);
    assert!((wrap_deg(540.0) - 180.0).abs() < 1e-4 && (wrap_deg(-190.0) - 170.0).abs() < 1e-4);
}

#[test]
fn mechanics_confidence_is_carried() {
    let c = Clamps::from_mechanics();
    assert_eq!(c.max_clicks_per_second.value, 14.0);
    assert_eq!(c.max_turn_per_tick.value, 60.0);
    assert_eq!(c.reach.value, 3.0);
}

#[test]
fn crit_only_when_valid() {
    let mut c = melee_with(&[("wtap_ticks", 0.0), ("shield_react", 0.0)]);
    let mut v = view(2.0);
    // Ready on the ground: jump, do not attack (a ground hit is no crit).
    let a = act(c.step(&obs(&v)));
    assert!(a.jump && !a.attack);
    // Rising (no fall distance yet): sprint is dropped, still no attack.
    v.on_ground = false;
    let a = act(c.step(&obs(&v)));
    assert!(!a.sprint && !a.attack);
    // Falling: attack.
    v.fall_distance = 0.4;
    assert!(act(c.step(&obs(&v))).attack);
    // Falling but in water: not a crit situation, so the normal rule applies on its own terms.
    let mut c2 = melee_with(&[("shield_react", 0.0)]);
    let mut w = view(2.0);
    w.in_water = true;
    assert!(act(c2.step(&obs(&w))).attack, "water disables the crit requirement");
    // Falling while sprinting is not valid.
    let mut c3 = melee_with(&[("shield_react", 0.0)]);
    let mut f = view(2.0);
    f.on_ground = false;
    f.fall_distance = 0.5;
    let mut o = obs(&f);
    o.sprinting = Some(true);
    assert!(!act(c3.step(&o)).attack);
    // Cooldown not ready: no attack even when falling.
    let mut c4 = melee_with(&[("shield_react", 0.0)]);
    f.attack_cooldown = 0.5;
    assert!(!act(c4.step(&obs(&f))).attack);
}

#[test]
fn wtap_timing() {
    let mut c = melee_with(&[("crit", 0.0), ("shield_react", 0.0), ("wtap_ticks", 2.0), ("spacing", 2.0)]);
    let mut v = view(2.0);
    let a = act(c.step(&obs(&v)));
    assert!(a.attack && a.sprint);
    v.attack_cooldown = 0.0; // the swing landed
    let sprints: Vec<bool> = (0..4)
        .map(|_| {
            let a = act(c.step(&obs(&v)));
            assert!(!a.attack);
            a.sprint
        })
        .collect();
    assert_eq!(sprints, vec![false, false, true, true]);
}

#[test]
fn shield_logic() {
    let mut v = view(2.0);
    v.offhand = Some(ItemStack::new("minecraft:shield", 1));
    v.attack_cooldown = 0.2;
    let mut c = melee_with(&[("crit", 0.0)]);
    assert!(act(c.step(&obs(&v))).block_with_shield);
    v.entities[0].blocking_with_shield = true;
    assert!(!act(c.step(&obs(&v))).block_with_shield, "target already blocking");
    v.entities[0].blocking_with_shield = false;
    v.entities[0].position.z = 6.0;
    assert!(!act(c.step(&obs(&v))).block_with_shield, "target out of reach");
    v.entities[0].position.z = 2.0;
    v.attack_cooldown = 0.7;
    assert!(!act(c.step(&obs(&v))).block_with_shield, "own cooldown high");
    v.attack_cooldown = 0.2;
    let mut off = melee_with(&[("crit", 0.0), ("shield_react", 0.0)]);
    assert!(!act(off.step(&obs(&v))).block_with_shield);
    v.offhand = None;
    assert!(!act(c.step(&obs(&v))).block_with_shield, "no shield");

    // The applier uses the shield once, then releases once.
    let mut ap = ActionApplier::default();
    let mut cmds = FakeCommands::default();
    let up = DuelAction { block_with_shield: true, ..Default::default() };
    for _ in 0..3 {
        ap.apply(&mut cmds, &up, TARGET);
    }
    ap.apply(&mut cmds, &DuelAction::default(), TARGET);
    ap.apply(&mut cmds, &DuelAction::default(), TARGET);
    let all = cmds.all();
    assert_eq!(all.iter().filter(|c| **c == Command::UseItem(Hand::Off)).count(), 1);
    assert_eq!(all.iter().filter(|c| **c == Command::ReleaseUse).count(), 1);
}

#[test]
fn done_reasons() {
    let mut c = melee_with(&[]);
    let mut v = view(2.0);
    v.health = 4.0;
    assert_eq!(c.step(&obs(&v)), DuelStep::Done(DoneReason::Retreat));
    v.health = 20.0;
    v.entities[0].health = Some(0.0);
    assert_eq!(c.step(&obs(&v)), DuelStep::Done(DoneReason::TargetDead));
    v.entities[0].health = Some(10.0);
    v.entities[0].position.z = 40.0;
    assert_eq!(c.step(&obs(&v)), DuelStep::Done(DoneReason::TargetLost));

    // Through the skill: retreat stops the bot and reports it.
    let mut v = view(2.0);
    let mut cm = FakeCommands::default();
    let mut s = FightSkill::new(player_rules(), Box::new(melee_with(&[])), 1);
    assert_eq!(run(&mut s, &mut v, &mut cm, 2), SkillStatus::Running);
    v.health = 3.0;
    assert_eq!(s.step(&v, &mut cm), SkillStatus::Failed("retreat".into()));
    assert_eq!(s.last_done, Some(DoneReason::Retreat));
    assert_eq!(cm.all().last(), Some(&Command::Stop));
    // Target's health hits 0 mid-fight: dead, not lost. A dead body is never picked.
    let mut v = view(2.0);
    let mut s = FightSkill::new(player_rules(), Box::new(melee_with(&[])), 1);
    let mut cm = FakeCommands::default();
    v.entities[0].health = Some(10.0);
    s.step(&v, &mut cm);
    v.entities[0].health = Some(0.0);
    s.step(&v, &mut cm);
    assert_eq!(s.last_done, Some(DoneReason::TargetDead));
    let mut s = FightSkill::new(player_rules(), Box::new(melee_with(&[])), 1);
    assert_eq!(s.step(&v, &mut cm), SkillStatus::Failed("no valid target".into()));
}

fn zombie(id: u32, z: f64) -> EntitySnapshot {
    EntitySnapshot::new(id, "minecraft:zombie", Vec3::new(0.0, 64.0, z))
}

#[test]
fn never_targets_cargo_or_allies() {
    let mut v = FakeView::default();
    v.entities = vec![zombie(1, 2.0), zombie(2, 3.0), zombie(3, 10.0)];
    let mut rules = FightRules::new(TargetSelector::Kinds(vec!["minecraft:zombie".into()]));
    rules.never_cargo = vec![EntityId(1)];
    rules.never_allies = vec![EntityId(2)];
    let mut s = FightSkill::new(rules.clone(), Box::new(melee_with(&[])), 1);
    let mut c = FakeCommands::default();
    s.step(&v, &mut c);
    assert_eq!(s.target(), Some(EntityId(3)));
    v.entities.truncate(2);
    let mut s = FightSkill::new(rules, Box::new(Spam), 1);
    assert!(matches!(s.step(&v, &mut c), SkillStatus::Failed(_)));
    // Naming cargo explicitly does not help.
    let mut r = FightRules::new(TargetSelector::Entities(vec![EntityId(1)]));
    r.never_cargo = vec![EntityId(1)];
    let mut s = FightSkill::new(r, Box::new(Spam), 1);
    assert!(matches!(s.step(&v, &mut c), SkillStatus::Failed(_)));
    assert!(attacks(&c).is_empty());
}

#[test]
fn player_first_rule_both_ways() {
    let mut v = view(2.0);
    v.tick = 10;
    let mut c = FakeCommands::default();
    let mk = |allow: bool| {
        let mut r = FightRules::new(TargetSelector::Entities(vec![TARGET]));
        r.attack_player_first_allowed = allow;
        FightSkill::new(r, Box::new(Spam), 1)
    };
    assert!(matches!(mk(false).step(&v, &mut c), SkillStatus::Failed(_)), "never first");
    assert_eq!(mk(true).step(&v, &mut c), SkillStatus::Running, "permission enabled");
    v.attackers.push(Attacker { entity: TARGET, tick: 5 });
    assert_eq!(mk(false).step(&v, &mut c), SkillStatus::Running, "it hit us recently");
    v.tick = 5000;
    assert!(matches!(mk(false).step(&v, &mut c), SkillStatus::Failed(_)), "too long ago");
    // Mid-fight: attacker window runs out, fight ends.
    v.tick = 10;
    let mut s = mk(false);
    assert_eq!(s.step(&v, &mut c), SkillStatus::Running);
    v.tick = 5000;
    assert!(matches!(s.step(&v, &mut c), SkillStatus::Failed(_)));
    // Non-players are not covered by the rule.
    let mut z = FakeView::default();
    z.entities.push(zombie(7, 2.0));
    let r = FightRules::new(TargetSelector::Entities(vec![EntityId(7)]));
    assert_eq!(FightSkill::new(r, Box::new(Spam), 1).step(&z, &mut c), SkillStatus::Running);
}

#[test]
fn cargo_proximity_rule() {
    let attack_count = |held: &str, cargo_z: f64| {
        let mut v = view(2.0);
        v.hotbar[0] = Some(ItemStack::new(held, 1));
        v.entities.push(zombie(50, cargo_z));
        let mut r = player_rules();
        r.never_cargo = vec![EntityId(50)];
        r.cargo_radius = 5.0;
        let mut s = FightSkill::new(r, Box::new(melee_with(&[("crit", 0.0)])), 1);
        let mut c = FakeCommands::default();
        run(&mut s, &mut v, &mut c, 1);
        (attacks(&c).len(), s.trace()[0].clamped.attacks_dropped_cargo)
    };
    assert_eq!(attack_count("minecraft:iron_sword", 4.0), (0, 1), "sword near cargo");
    assert_eq!(attack_count("minecraft:iron_axe", 4.0), (1, 0), "axe does not sweep");
    assert_eq!(attack_count("minecraft:iron_sword", 30.0), (1, 0), "cargo far away");
}

#[test]
fn deterministic_by_seed() {
    let v = view(2.6);
    let o = obs(&v);
    let run_seed = |seed: u64| -> Vec<DuelAction> {
        let mut c = melee_with(&[("strafe_jitter", 0.6), ("strafe_period", 8.0), ("crit", 0.0)]);
        c.reset(seed);
        (0..200).map(|_| act(c.step(&o))).collect()
    };
    assert_eq!(run_seed(42), run_seed(42));
    let strafes = |s| run_seed(s).iter().map(|a| a.strafe).collect::<Vec<_>>();
    assert_ne!(strafes(1), strafes(2));
    assert_ne!(strafes(42), strafes(43));
    // Reset mid-run replays the same sequence.
    let mut c = melee_with(&[("strafe_jitter", 0.6)]);
    c.reset(9);
    let a: Vec<_> = (0..50).map(|_| act(c.step(&o))).collect();
    c.reset(9);
    let b: Vec<_> = (0..50).map(|_| act(c.step(&o))).collect();
    assert_eq!(a, b);
}

#[test]
fn trace_lengths_and_json() {
    let mut v = view(2.0);
    let mut c = FakeCommands::default();
    let mut s = FightSkill::new(player_rules(), Box::new(melee_with(&[])), 3);
    run(&mut s, &mut v, &mut c, 5);
    let t = s.take_trace();
    assert_eq!(t.len(), 5);
    assert!(s.trace().is_empty());
    for r in &t {
        assert_eq!(r.obs.len(), OBS_FIELDS.len());
        assert_eq!(r.action.len(), ACTION_FIELDS.len());
        assert!(r.to_json_line().starts_with("{\"tick\":"));
    }
}

#[test]
fn doctrine_round_trip_and_rejections() {
    let text = "technique = melee_basic\nversion = 3\nparent = 2\n[params]\nspacing = 2.6\nstrafe_period = 12\nstrafe_jitter = 0.25\nattack_at_cooldown = 0.95\ncrit = 1\nwtap_ticks = 2\nshield_react = 1\nretreat_hp = 6\ncrit_wait_ticks = 6\n";
    let d = Doctrine::parse(text).unwrap();
    assert_eq!((d.version, d.parent), (3, Some(2)));
    assert_eq!(d.to_string(), text);
    assert_eq!(Doctrine::parse(&d.to_string()).unwrap(), d);
    assert!(MeleeBasic::from_doctrine(&d).is_ok());

    let file = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/doctrine/melee_basic.v1.txt")).unwrap();
    let d1 = Doctrine::parse(&file).unwrap();
    assert_eq!(d1.to_string(), file);
    assert_eq!(d1.params, MELEE_BASIC_SPACE.defaults());
    assert_eq!(d1.parent, None);

    // Comments and a partial param list are accepted; missing params take defaults.
    let loose = "# note\ntechnique = melee_basic\nversion = 4\n\n[params]\nspacing = 3\n";
    let d = Doctrine::parse(loose).unwrap();
    assert_eq!(d.params.get("spacing"), Some(3.0));
    assert_eq!(d.params.get("retreat_hp"), Some(6.0));

    let head = "technique = melee_basic\nversion = 1\n[params]\n";
    for (bad, why) in [
        (format!("{head}speed = 1\n"), "unknown param"),
        (format!("{head}spacing = 9\n"), "out of bounds"),
        (format!("{head}spacing = abc\n"), "bad number"),
        (format!("{head}spacing = 2\nspacing = 3\n"), "duplicate"),
        ("technique = zzz\nversion = 1\n".to_string(), "unknown technique"),
        ("technique = melee_basic\n".to_string(), "no version"),
        ("version = 1\n".to_string(), "no technique"),
        ("technique = melee_basic\nversion = 1\ncolour = red\n".to_string(), "unknown key"),
        ("technique = melee_basic\nversion = x\n".to_string(), "bad version"),
        ("technique = melee_basic\nversion = 1\nnonsense\n".to_string(), "no ="),
    ] {
        assert!(Doctrine::parse(&bad).is_err(), "{why} should be rejected");
    }
}

#[test]
fn melee_params_unit_round_trip() {
    let d = MELEE_BASIC_SPACE.defaults();
    let u = MELEE_BASIC_SPACE.to_unit(&d);
    assert_eq!(u.len(), 9);
    assert!(u.iter().all(|x| (0.0..=1.0).contains(x)));
    let back = MELEE_BASIC_SPACE.from_unit(&u);
    for ((_, a), (_, b)) in d.0.iter().zip(&back.0) {
        assert!((a - b).abs() < 1e-9);
    }
    assert!(MELEE_BASIC_SPACE.validate(&MELEE_BASIC_SPACE.from_unit(&[2.0; 8])).is_ok());
    assert!(MeleeBasic::new(&MELEE_BASIC_SPACE.from_unit(&[0.0; 8])).is_ok());
}

fn named(id: u32, name: &str, z: f64) -> EntitySnapshot {
    let mut e = EntitySnapshot::new(id, "minecraft:player", Vec3::new(0.0, 64.0, z));
    e.name = Some(name.into());
    e
}

#[test]
fn player_names_target_only_named() {
    let mut v = FakeView::default();
    let watcher = named(1, "allen546", 1.0);
    let mut nameless = EntitySnapshot::new(2, "minecraft:player", Vec3::new(0.0, 64.0, 1.5));
    nameless.name = None;
    let enemy = named(3, "null_b1", 8.0);
    v.entities = vec![watcher, nameless, enemy];
    let rules = FightRules::new(TargetSelector::PlayerNames(vec!["null_b1".into(), "null_b2".into()]));
    let ids: Vec<u32> = v.entities.iter().filter(|e| rules.is_valid_target(&v, e)).map(|e| e.id.0).collect();
    // Named enemy is attackable first; the nearer watcher and the nameless player are not.
    assert_eq!(ids, vec![3]);
    let mut s = FightSkill::new(rules, Box::new(Spam), 1);
    let mut c = FakeCommands::default();
    s.step(&v, &mut c);
    assert_eq!(s.target(), Some(EntityId(3)));
}

#[test]
fn ally_name_never_targeted_even_if_listed() {
    let mut v = FakeView::default();
    v.entities = vec![named(1, "null_a1", 1.0), named(2, "null_b1", 5.0)];
    let mut rules = FightRules::new(TargetSelector::PlayerNames(vec!["null_a1".into(), "null_b1".into()]));
    rules.never_names = vec!["null_a1".into()];
    assert!(!rules.is_valid_target(&v, &v.entities[0]));
    assert!(rules.is_valid_target(&v, &v.entities[1]));
}

#[test]
fn kind_selector_still_protects_unattacking_players() {
    let mut v = FakeView::default();
    v.entities = vec![named(1, "allen546", 1.0)];
    let rules = FightRules::new(TargetSelector::Kinds(vec!["minecraft:player".into()]));
    assert!(!rules.is_valid_target(&v, &v.entities[0]));
}

#[test]
fn engage_skips_dead_enemy_and_goes_to_next() {
    let mut v = FakeView::default();
    v.hotbar[0] = Some(null_bot_api::ItemStack::new("minecraft:iron_sword", 1));
    let mut dead = named(1, "null_b1", 3.0);
    dead.health = Some(0.0);
    v.entities = vec![dead, named(2, "null_b2", 12.0)];
    let mut rules = FightRules::new(TargetSelector::PlayerNames(vec!["null_b1".into(), "null_b2".into()]));
    rules.never_names = vec!["null_a1".into()];
    let mut d = Engage::new(rules, (0.0, 0.0), 1);
    let mut c = FakeCommands::default();
    v.tick = 20;
    d.step(&v, &mut c);
    assert_eq!(d.fights, 0);
    // Heads for the live enemy at z=12.
    assert!(c.all().iter().any(|x| matches!(x, Command::Goto { target, .. } if target.z == 12)));
}

#[test]
fn flat_ground_bot_without_crit_still_swings() {
    let mut v = view(2.4);
    v.on_ground = true;
    v.attack_cooldown = 1.0;
    let mut s = FightSkill::new(player_rules(), Box::new(melee_with(&[])), 1);
    let mut c = FakeCommands::default();
    run(&mut s, &mut v, &mut c, 30);
    assert!(!attacks(&c).is_empty());
}

#[test]
fn step_ahead_sees_a_full_block_but_not_open_ground() {
    use null_bot_api::{BlockPos, Vec3, fake::FakeView};
    use null_combat::fight::step_ahead;
    let mut v = FakeView::default();
    v.floor_y = 0;
    v.position = Vec3::new(0.5, 0.0, 0.5);
    // Yaw 0 faces +z in Minecraft.
    assert!(!step_ahead(&v, 1, 0, 0.0));
    v.solid.push(BlockPos::new(0, 0, 1));
    assert!(step_ahead(&v, 1, 0, 0.0));
    assert!(!step_ahead(&v, -1, 0, 0.0), "the block is behind when walking backwards");
    v.solid.push(BlockPos::new(0, 2, 1));
    assert!(!step_ahead(&v, 1, 0, 0.0), "no room to stand on it");
}

#[test]
fn engage_waits_for_a_weapon() {
    let mut v = FakeView::default();
    v.entities = vec![named(2, "null_b2", 12.0)];
    let rules = FightRules::new(TargetSelector::PlayerNames(vec!["null_b2".into()]));
    let mut d = Engage::new(rules, (0.0, 0.0), 1);
    let mut c = FakeCommands::default();
    v.tick = 20;
    d.step(&v, &mut c);
    assert!(c.all().is_empty(), "no kit yet, so no movement");
    v.hotbar[0] = Some(null_bot_api::ItemStack::new("minecraft:iron_sword", 1));
    v.tick = 21;
    d.step(&v, &mut c);
    assert!(c.all().iter().any(|x| matches!(x, Command::Goto { .. })));
}

#[test]
fn obstacle_checks() {
    use null_bot_api::{BlockPos, Vec3, fake::FakeView};
    use null_combat::fight::{drop_ahead, line_blocked, wall_ahead};
    let mut v = FakeView::default();
    v.floor_y = 0;
    v.position = Vec3::new(0.5, 0.0, 0.5);
    assert!(!drop_ahead(&v, 1, 0, 0.0) && !wall_ahead(&v, 0.0));
    assert!(!line_blocked(&v, v.position, Vec3::new(0.5, 0.0, 6.5)));
    // A tree trunk 3 blocks ahead blocks the straight line, not the next step.
    v.solid.push(BlockPos::new(0, 1, 3));
    assert!(line_blocked(&v, v.position, Vec3::new(0.5, 0.0, 6.5)));
    assert!(!wall_ahead(&v, 0.0));
    v.solid.push(BlockPos::new(0, 1, 1));
    assert!(wall_ahead(&v, 0.0));
    // A hole: no ground within 4 blocks below the next block.
    let mut h = FakeView::default();
    h.floor_y = -100;
    h.position = Vec3::new(0.5, 0.0, 0.5);
    h.solid.push(BlockPos::new(0, -1, 0));
    assert!(drop_ahead(&h, 1, 0, 0.0));
    h.solid.push(BlockPos::new(0, -3, 1));
    h.solid.push(BlockPos::new(0, -3, 2));
    assert!(!drop_ahead(&h, 1, 0, 0.0), "a 2-block step down is fine");
}
