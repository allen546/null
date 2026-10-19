//! Mechanism crate: the only place (with null-node) that imports Azalea and Bevy (F5 §4.1).
//! Implements `BotView` over Azalea components and applies `BotCommands` as component writes.

pub mod actions;
pub mod blocks;
pub mod host;
pub mod lifecycle;
pub mod view;

use azalea::app::{PluginGroup, PluginGroupBuilder};

pub use host::{BotBrain, BotDied, HostPlugin, HostTick, ReflexDisabled, SKILL_DIED, SKILL_LEAK};
pub use lifecycle::{
    Backoff, BotState, BotStateChange, BrainFactory, HostEvent, KickClass, NodePlugin,
    classify_kick,
};

/// Azalea's default bot plugins minus the ones that act on their own (CONVENTIONS §4.5).
///
/// Removed: `AutoRespawnPlugin` and `AutoReconnectPlugin` (replaced by `lifecycle`, which
/// reports `BotState` and backs off). Kept and therefore registered here: `BotPlugin` (jump and
/// look-at message handlers; inert unless something sends those messages), `PathfinderPlugin`
/// (acts only on `GotoEvent`, which only the host sends, with mining off), `ContainerPlugin`,
/// `TickBroadcastPlugin`, `EventsPlugin` (plumbing), and `AcceptResourcePacksPlugin` (answers
/// resource-pack prompts with "accepted"; the skeleton server sends none).
pub fn null_bot_plugins() -> PluginGroupBuilder {
    azalea::bot::DefaultBotPlugins
        .build()
        .disable::<azalea::auto_respawn::AutoRespawnPlugin>()
        .disable::<azalea::auto_reconnect::AutoReconnectPlugin>()
}
