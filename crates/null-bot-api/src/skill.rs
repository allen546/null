use crate::{BotCommands, BotView, Death};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillStatus {
    Running,
    Done,
    Failed(String),
}

// Spec: specs/F1-swarm-runtime.md §4.4 (the skill decides whether it survives a death)
/// What a skill wants when its bot dies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterDeath {
    /// The skill ends with `Failed("died")` at the death tick.
    End,
    /// The skill stays: it is stepped every tick while the bot is dead (its commands are refused)
    /// and carries on, as the same instance, after the respawn.
    Keep,
}

/// A skill is stepped once per tick while no reflex preempts it.
/// Skills never rely on reflexes to stay correct (CONVENTIONS §4.4).
pub trait Skill: Send + Sync {
    fn id(&self) -> &str;
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus;
    // Spec: specs/F1-swarm-runtime.md §4.4
    /// Called once, at the death tick, with the death record. The default ends the job.
    fn on_death(&mut self, _view: &dyn BotView, _death: &Death) -> AfterDeath {
        AfterDeath::End
    }
}
