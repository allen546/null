//! Toy world and fake bots for the control spine (S3). Decision crate: no azalea, no bevy.
//! A fake bot speaks only S2 messages over the bus, so the same squad code can later drive real bots (S6).
//! Bus subjects: jobs in on `bot_jobs`, outcomes out on `bot_up`, `BotState` on the bot's summary subject, halt on `bot_cmd`.

mod bot;
#[cfg(test)]
mod tests;
mod world;

pub use bot::{LEASE_MARGIN, SimBot};
pub use world::{SharedWorld, SimRng, SimWorld};

use null_bus::{Level, Subject, kind};
use null_proto::{BotId, ItemId, ServerId};

/// The one item the skeleton collects.
pub const OAK_LOG: &str = "minecraft:oak_log";

pub fn oak_log() -> ItemId {
    ItemId::new(OAK_LOG)
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot receives `IntentBody::JobBatch`.
pub fn bot_jobs(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::JOBS)
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot publishes `IntentBody::JobOutcome` (its squad consumes it).
pub fn bot_up(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), "up")
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot publishes `SummaryBody::Bot`.
pub fn bot_summary(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::SUMMARY)
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot receives `Command::Halt`.
pub fn bot_cmd(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::CMD)
}
