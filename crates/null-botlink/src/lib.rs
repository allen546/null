//! The bus side of a real bot (S6a): jobs, leases, halt, outcomes and `BotState`, with a `gather` run behind a `Host`.
//! Decision crate: no azalea, no bevy. `null-node` implements `Host` over the Bevy bot; tests use a fake one.
//! It does what `null_simbot::SimBot` does on the bus, so the same squad code drives either.

mod link;
mod run;
#[cfg(test)]
mod tests;

pub use link::{BotLink, Host, HostStatus};
pub use run::{FellRun, FellSpec, RunEnd, RunHandle, RunStatus};
