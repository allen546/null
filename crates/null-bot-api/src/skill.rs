use crate::{BotCommands, BotView};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkillStatus {
    Running,
    Done,
    Failed(String),
}

/// A skill is stepped once per tick while no reflex preempts it.
/// Skills never rely on reflexes to stay correct (CONVENTIONS §4.4).
pub trait Skill: Send + Sync {
    fn id(&self) -> &str;
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus;
}
