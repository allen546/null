//! Toy world and fake bots for the control spine (S3). Decision crate: no azalea, no bevy.
//! A fake bot speaks only S2 messages over the bus, so the same squad code can later drive real bots (S6).
//! Bus subjects: jobs in on `bot_jobs`, outcomes out on `bot_up`, `BotState` on the bot's summary subject, halt on `bot_cmd`.

mod bot;
#[cfg(test)]
mod tests;
mod world;

pub use bot::SimBot;
pub use world::{SharedWorld, SimRng, SimWorld};

pub use null_bus::{bot_cmd, bot_jobs, bot_summary, bot_up};
pub use null_proto::LEASE_MARGIN;
use null_proto::ItemId;

/// The one item the skeleton collects.
pub const OAK_LOG: &str = "minecraft:oak_log";

pub fn oak_log() -> ItemId {
    ItemId::new(OAK_LOG)
}
