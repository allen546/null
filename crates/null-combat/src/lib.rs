//! Null's combat decision crate (B5, AD-B5.1). Depends only on the bot API and the
//! mechanics table. Controllers decide; vanilla clamps and rule filters run after them.

pub mod action;
pub mod clamps;
pub mod controller;
pub mod doctrine;
pub mod engage;
pub mod fight;
pub mod melee;
pub mod obs;
pub mod params;
pub mod retreat;
pub mod shield;
pub mod rng;
pub mod telemetry;
pub mod trace;

pub use action::{ACTION_FIELDS, ActionApplier, DuelAction};
pub use clamps::{ClampReport, Clamps, wrap_deg};
pub use controller::{DoneReason, DuelController, DuelStep};
pub use doctrine::Doctrine;
pub use engage::Engage;
pub use fight::{drop_depth, drop_worth, FightRules, FightSkill, TargetSelector};
pub use melee::{MELEE_BASIC_SPACE, MeleeBasic};
pub use obs::{DuelObs, OBS_FIELDS, TargetObs, observe};
pub use retreat::Retreat;
pub use shield::ShieldDelay;
pub use telemetry::Telemetry;
pub use params::{ParamSpace, ParamSpec, Params};
pub use trace::TraceRecord;
