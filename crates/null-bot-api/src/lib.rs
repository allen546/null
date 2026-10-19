//! Null's decision API (F5 §3). Zero dependencies, builds on stable.
//! Decision code sees the game only through `BotView` and `BotCommands`.

pub mod commands;
pub mod fake;
pub mod reflex;
pub mod skill;
pub mod types;
pub mod view;

pub use commands::{BotCommands, Command};
pub use reflex::{
    Brain, BotEvent, Group, Param, Reflex, ReflexSpec, Scope, Settings, Target, TickOutcome,
};
pub use skill::{AfterDeath, Skill, SkillStatus};
pub use types::*;
pub use view::BotView;
