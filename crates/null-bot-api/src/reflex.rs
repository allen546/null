//! Reflex registry entries and per-bot arbitration (CONVENTIONS §4, F1 §4.5).

use crate::{BotCommands, BotView, Owner, Skill, SkillStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Movement,
    Survival,
    Combat,
    Mob,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub value: f64,
    pub unit: &'static str,
}

/// Registry entry (CONVENTIONS §4.1). Trigger and action are the trait methods.
#[derive(Clone, Debug)]
pub struct ReflexSpec {
    pub id: &'static str,
    pub group: Group,
    /// Higher wins when two enabled reflexes fire in one tick.
    pub priority: i32,
    pub default_on: bool,
    pub params: Vec<Param>,
}

pub trait Reflex: Send + Sync {
    fn spec(&self) -> &ReflexSpec;
    /// Evaluated every tick, even when the reflex is off (CONVENTIONS §4.3).
    fn trigger(&mut self, view: &dyn BotView) -> bool;
    /// Only called when the reflex is enabled and wins arbitration.
    fn act(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands);
}

/// What a setting names. Reflex beats group beats all within one scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    All,
    Group(Group),
    Reflex(String),
}

impl Target {
    fn rank(&self) -> u8 {
        match self {
            Target::All => 0,
            Target::Group(_) => 1,
            Target::Reflex(_) => 2,
        }
    }
    fn matches(&self, spec: &ReflexSpec) -> bool {
        match self {
            Target::All => true,
            Target::Group(g) => *g == spec.group,
            Target::Reflex(id) => id == spec.id,
        }
    }
}

/// Settings made at one scope. Setting the same target again replaces it.
#[derive(Clone, Debug, Default)]
pub struct Settings(Vec<(Target, bool)>);

impl Settings {
    pub fn set(&mut self, target: Target, on: bool) {
        match self.0.iter_mut().find(|(t, _)| *t == target) {
            Some(e) => e.1 = on,
            None => self.0.push((target, on)),
        }
    }
    pub fn clear(&mut self) {
        self.0.clear();
    }
    fn resolve(&self, spec: &ReflexSpec) -> Option<bool> {
        self.0.iter().filter(|(t, _)| t.matches(spec)).max_by_key(|(t, _)| t.rank()).map(|e| e.1)
    }
}

/// Only bot and job scope exist for now; policy, operation and squad come later.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Job,
    Bot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotEvent {
    /// A disabled reflex's trigger fired and was not acted on.
    ReflexSuppressed { id: &'static str, tick: u64 },
}

#[derive(Debug, PartialEq)]
pub struct TickOutcome {
    /// Id of the reflex that preempted the skill this tick.
    pub reflex: Option<&'static str>,
    /// Skill status when the skill ran; None when preempted or absent.
    pub skill: Option<SkillStatus>,
    pub events: Vec<BotEvent>,
}

/// One bot's reflexes and switches.
#[derive(Default)]
pub struct Brain {
    reflexes: Vec<Box<dyn Reflex>>,
    bot: Settings,
    job: Settings,
}

impl Brain {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, reflex: Box<dyn Reflex>) {
        self.reflexes.push(reflex);
    }

    pub fn set(&mut self, scope: Scope, target: Target, on: bool) {
        match scope {
            Scope::Bot => self.bot.set(target, on),
            Scope::Job => self.job.set(target, on),
        }
    }

    /// Job-scope settings last only while the job runs.
    pub fn end_job(&mut self) {
        self.job.clear();
    }

    /// Bot scope beats job scope; with neither set, the registry default applies.
    pub fn is_enabled(&self, spec: &ReflexSpec) -> bool {
        self.bot.resolve(spec).or_else(|| self.job.resolve(spec)).unwrap_or(spec.default_on)
    }

    /// Runs one tick: the best enabled firing reflex acts, else the skill steps.
    pub fn tick(
        &mut self,
        view: &dyn BotView,
        cmds: &mut dyn BotCommands,
        skill: Option<&mut dyn Skill>,
    ) -> TickOutcome {
        let tick = view.tick();
        let mut events = Vec::new();
        let mut winner: Option<usize> = None;
        for i in 0..self.reflexes.len() {
            if !self.reflexes[i].trigger(view) {
                continue;
            }
            let spec = self.reflexes[i].spec();
            if !self.is_enabled(spec) {
                events.push(BotEvent::ReflexSuppressed { id: spec.id, tick });
            } else if winner.is_none_or(|w| spec.priority > self.reflexes[w].spec().priority) {
                winner = Some(i); // strict `>` keeps registration order on ties
            }
        }
        if let Some(w) = winner {
            let id = self.reflexes[w].spec().id;
            // Spec: specs/B6-placement-breaking.md §4.6 (every command is stamped with its owner)
            cmds.set_owner(Owner::Reflex(id));
            self.reflexes[w].act(view, cmds);
            return TickOutcome { reflex: Some(id), skill: None, events };
        }
        cmds.set_owner(Owner::Skill);
        let skill = skill.map(|s| s.step(view, cmds));
        TickOutcome { reflex: None, skill, events }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{FakeCommands, FakeView};
    use crate::Command;

    struct R {
        spec: ReflexSpec,
        fires: bool,
    }
    fn r(id: &'static str, group: Group, priority: i32, default_on: bool, fires: bool) -> Box<R> {
        Box::new(R { spec: ReflexSpec { id, group, priority, default_on, params: vec![] }, fires })
    }
    impl Reflex for R {
        fn spec(&self) -> &ReflexSpec {
            &self.spec
        }
        fn trigger(&mut self, _: &dyn BotView) -> bool {
            self.fires
        }
        fn act(&mut self, _: &dyn BotView, cmds: &mut dyn BotCommands) {
            cmds.sprint(self.spec.priority > 0); // marks which reflex acted
            cmds.select_slot(self.spec.priority as u8);
        }
    }
    struct S;
    impl Skill for S {
        fn id(&self) -> &str {
            "s"
        }
        fn step(&mut self, _: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
            cmds.jump();
            SkillStatus::Running
        }
    }

    fn run(b: &mut Brain) -> (TickOutcome, FakeCommands) {
        let v = FakeView::default();
        let mut c = FakeCommands::default();
        let o = b.tick(&v, &mut c, Some(&mut S));
        (o, c)
    }

    #[test]
    fn commands_are_stamped_with_the_stepping_owner() {
        use crate::Owner;
        let mut b = Brain::new();
        b.register(r("flee", Group::Survival, 5, true, false));
        assert_eq!(run(&mut b).1.owner, Some(Owner::Skill));
        let mut b = Brain::new();
        b.register(r("flee", Group::Survival, 5, true, true));
        assert_eq!(run(&mut b).1.owner, Some(Owner::Reflex("flee")));
    }

    #[test]
    fn skill_runs_when_nothing_fires() {
        let mut b = Brain::new();
        b.register(r("a", Group::Combat, 1, true, false));
        let (o, c) = run(&mut b);
        assert_eq!(o.skill, Some(SkillStatus::Running));
        assert_eq!(c.all(), vec![Command::Jump]);
    }

    #[test]
    fn highest_priority_enabled_reflex_preempts_skill() {
        let mut b = Brain::new();
        b.register(r("low", Group::Combat, 1, true, true));
        b.register(r("high", Group::Survival, 5, true, true));
        b.register(r("off", Group::Mob, 9, false, true));
        let (o, c) = run(&mut b);
        assert_eq!(o.reflex, Some("high"));
        assert_eq!(o.skill, None);
        assert!(!c.all().contains(&Command::Jump));
        assert!(c.all().contains(&Command::SelectSlot(5)));
        // The disabled higher-priority reflex is reported, not acted on.
        assert_eq!(o.events, vec![BotEvent::ReflexSuppressed { id: "off", tick: 0 }]);
    }

    #[test]
    fn disabled_reflex_only_emits_suppressed_and_skill_runs() {
        let mut b = Brain::new();
        b.register(r("x", Group::Combat, 1, false, true));
        let (o, _) = run(&mut b);
        assert_eq!(o.reflex, None);
        assert_eq!(o.skill, Some(SkillStatus::Running));
        assert_eq!(o.events.len(), 1);
    }

    #[test]
    fn untriggered_disabled_reflex_is_silent() {
        let mut b = Brain::new();
        b.register(r("x", Group::Combat, 1, false, false));
        assert!(run(&mut b).0.events.is_empty());
    }

    #[test]
    fn scope_precedence() {
        let mut b = Brain::new();
        b.register(r("x", Group::Combat, 1, true, true));
        let spec = b.reflexes[0].spec().clone();
        assert!(b.is_enabled(&spec));
        // Job group off beats the default.
        b.set(Scope::Job, Target::Group(Group::Combat), false);
        assert!(!b.is_enabled(&spec));
        // Within one scope the reflex setting beats the group setting.
        b.set(Scope::Job, Target::Reflex("x".into()), true);
        assert!(b.is_enabled(&spec));
        // Bot scope beats job scope even with a less specific target.
        b.set(Scope::Bot, Target::All, false);
        assert!(!b.is_enabled(&spec));
        b.set(Scope::Bot, Target::Reflex("x".into()), true);
        assert!(b.is_enabled(&spec));
        // Setting the same target again replaces it.
        b.set(Scope::Bot, Target::Reflex("x".into()), false);
        assert!(!b.is_enabled(&spec));
    }

    #[test]
    fn job_settings_clear_when_job_ends_bot_settings_stay() {
        let mut b = Brain::new();
        b.register(r("x", Group::Survival, 1, true, true));
        let spec = b.reflexes[0].spec().clone();
        b.set(Scope::Job, Target::All, false);
        assert!(!b.is_enabled(&spec));
        assert_eq!(run(&mut b).0.events.len(), 1);
        b.end_job();
        assert!(b.is_enabled(&spec));
        assert_eq!(run(&mut b).0.reflex, Some("x"));

        b.set(Scope::Bot, Target::All, false);
        b.end_job();
        assert!(!b.is_enabled(&spec));
    }
}
