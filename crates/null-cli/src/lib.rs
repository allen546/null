//! The command line (K4 §4.1) over the K2 command set, and the one-process spine it drives: HQ, operations,
//! squads and simulated bots on an in-process bus. Decision crate: no azalea, no bevy.

mod audit;
mod parse;
mod spine;
#[cfg(test)]
mod tests;

pub use audit::{Audit, AuditEntry};
pub use parse::{HELP, Line, parse_line, render};
pub use spine::{Spine, SpineBot, SpineConfig, TreeSource};
