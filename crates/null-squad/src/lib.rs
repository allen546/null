//! The squad leader (K6, skeleton): hands jobs to idle bots, keeps each job's claim alive, retries once,
//! applies reflex settings, merges observations and reports lost bots. Decision crate: no azalea, no bevy.
//! It speaks only S2 messages. The bot subjects come from `null-simbot` (a real bot in S6 uses the same names).

mod squad;
#[cfg(test)]
mod tests;

pub use squad::{BotPorts, Squad, SquadConfig, squad_cmd};
