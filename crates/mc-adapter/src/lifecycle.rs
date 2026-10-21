//! Per-bot lifecycle (F1 §4.3-4.4): BotState events, our own reconnect with exponential backoff,
//! respawn on death. Azalea's own auto-reconnect and auto-respawn plugins are NOT installed
//! (CONVENTIONS §4.5); these systems replace them and are the only place that does either.

use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Sender},
    time::{Duration, Instant},
};

use azalea::{
    account::Account,
    app::{App, Plugin, Update},
    disconnect::DisconnectEvent,
    entity::HasClientLoaded,
    join::{
        ConnectOpts, ConnectionFailedEvent, StartJoinServerEvent, handle_start_join_server_event,
        poll_create_connection_task,
    },
    packet::{death_event_on_0_health, game::DeathEvent},
    respawn::{PerformRespawnEvent, perform_respawn},
};
use azalea::ecs::prelude::*;
use null_bot_api::SkillStatus;

use azalea::unmodelled_disconnect::{DisconnectOnUnmodelledMovement, PacketsAfterUnmodelledDisconnect, UnmodelledDisconnect};
use crate::host::{BotBrain, BotDied, ReflexDisabled, SKILL_DIED, SkillEnded};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BotState {
    Connecting,
    Spawned,
    Idle,
    OnJob(String),
    Dead,
    Respawning,
    /// Disconnected; `retry_in_ms` is None when the bot will not come back on its own.
    Kicked { class: KickClass, reason: String, retry_in_ms: Option<u64> },
    /// The login itself failed (network); retried with backoff.
    Offline { reason: String, retry_in_ms: u64 },
    Banned,
    /// The fork closed the connection for a `Movement` entry (AD-F5.5). The bot stays off: nothing
    /// above the fork rejoins it unless it decides to (F5 §4.6).
    UnmodelledOff { tick: u64, inputs: Vec<String>, packets_after: u32 },
}

impl BotState {
    pub fn label(&self) -> &'static str {
        match self {
            BotState::Connecting => "connecting",
            BotState::Spawned => "spawned",
            BotState::Idle => "idle",
            BotState::OnJob(_) => "on_job",
            BotState::Dead => "dead",
            BotState::Respawning => "respawning",
            BotState::Kicked { .. } => "kicked",
            BotState::Offline { .. } => "offline",
            BotState::Banned => "banned",
            BotState::UnmodelledOff { .. } => "unmodelled_off",
        }
    }
}

/// F1 §4.10, reduced to what a vanilla server shows us. The raw reason stays a string here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KickClass {
    Banned,
    /// Same account logged in elsewhere; never reconnect (F1 §4.10).
    Superseded,
    Other,
}

pub fn classify_kick(reason: &str) -> KickClass {
    let r = reason.to_lowercase();
    if r.contains("banned") {
        KickClass::Banned
    } else if r.contains("logged in from another location") || r.contains("already online") {
        KickClass::Superseded
    } else {
        KickClass::Other
    }
}

#[derive(Clone, Debug)]
pub struct BotStateChange {
    pub bot: String,
    pub state: BotState,
    /// Milliseconds since the node started.
    pub at_ms: u64,
}

#[derive(Clone, Debug)]
pub enum HostEvent {
    State(BotStateChange),
    SkillEnded { bot: String, skill: String, status: SkillStatus, tick: u64 },
    /// The first host tick that saw the death, with the record (position, dimension, message,
    /// inventory snapshot). A running skill ends the same tick with `Failed("died")`.
    Died { bot: String, tick: u64, death: Option<null_bot_api::Death> },
    /// The leak guard turned a reflex off at bot scope: it held 4,096 unreleased results (AD-F5.4).
    ReflexDisabled { bot: String, reflex: &'static str, tick: u64 },
    /// The fork closed the connection because a `Movement` input was in force (AD-F5.5). The bot
    /// stays off; no rejoin is tried.
    UnmodelledDisconnect { bot: String, tick: u64, inputs: Vec<String>, packets_after: u32 },
}

#[derive(Clone, Copy, Debug)]
pub struct Backoff {
    pub base: Duration,
    pub cap: Duration,
    /// Fraction of the delay added or removed at random.
    pub jitter: f64,
    /// A bot that stayed up this long starts over at the base delay.
    pub stable_after: Duration,
}

impl Default for Backoff {
    fn default() -> Self {
        Self {
            base: Duration::from_secs(2),
            cap: Duration::from_secs(60),
            jitter: 0.25,
            stable_after: Duration::from_secs(30),
        }
    }
}

impl Backoff {
    /// Delay before retry number `attempt` (0 = first retry); `unit` in [0,1) picks the jitter.
    pub fn delay(&self, attempt: u32, unit: f64) -> Duration {
        let raw = self.base.as_secs_f64() * 2f64.powi(attempt.min(30) as i32);
        let capped = raw.min(self.cap.as_secs_f64());
        let j = 1.0 + self.jitter * (2.0 * unit - 1.0);
        Duration::from_secs_f64(capped * j)
    }
}

/// Builds a bot's brain (reflexes and first skill) by account name; called at every spawn.
pub type BrainFactory = Arc<dyn Fn(&str) -> BotBrain + Send + Sync>;

pub struct NodePlugin {
    pub factory: BrainFactory,
    pub events: Sender<HostEvent>,
    pub backoff: Backoff,
    /// false: a dead bot stays on the death screen (last-team-standing runs).
    pub respawn: bool,
}

struct Rec {
    name: String,
    attempt: u32,
    retry_at: Option<Instant>,
    spawned_at: Option<Instant>,
}

#[derive(Resource)]
struct Lifecycle {
    start: Instant,
    bots: HashMap<Entity, Rec>,
    factory: BrainFactory,
    tx: Sender<HostEvent>,
    backoff: Backoff,
    respawn: bool,
    rng: u64,
}

impl Lifecycle {
    fn emit(&self, e: Entity, state: BotState) {
        if let Some(r) = self.bots.get(&e) {
            let _ = self.tx.send(HostEvent::State(BotStateChange {
                bot: r.name.clone(),
                state,
                at_ms: self.start.elapsed().as_millis() as u64,
            }));
        }
    }

    fn unit(&mut self) -> f64 {
        // xorshift64: jitter only needs to decorrelate bots, not be good randomness.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Records the end of a connection and schedules the retry. Returns the delay if any.
    fn schedule_retry(&mut self, e: Entity) -> Duration {
        let now = Instant::now();
        let unit = self.unit();
        let (b, attempt) = {
            let b = self.backoff;
            let r = self.bots.get_mut(&e).expect("known bot");
            if r.spawned_at.is_some_and(|t| now - t >= b.stable_after) {
                r.attempt = 0;
            }
            r.spawned_at = None;
            (b, r.attempt)
        };
        let d = b.delay(attempt, unit);
        let r = self.bots.get_mut(&e).expect("known bot");
        r.attempt += 1;
        r.retry_at = Some(now + d);
        d
    }
}

impl Plugin for NodePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Lifecycle {
            start: Instant::now(),
            bots: HashMap::new(),
            factory: self.factory.clone(),
            tx: self.events.clone(),
            backoff: self.backoff,
            respawn: self.respawn,
            rng: 0x9E37_79B9_7F4A_7C15 ^ std::process::id() as u64,
        })
        // Ordered so a death is seen the same frame and our respawn/rejoin messages are
        // handled the same frame (the ambiguity logger flagged these).
        .add_systems(
            Update,
            lifecycle
                .after(death_event_on_0_health)
                // Azalea chains handle_start_join -> poll_create_connection, so we cannot sit
                // between them; a ConnectionFailedEvent is read this frame or the next.
                .ambiguous_with(poll_create_connection_task)
                .before(perform_respawn)
                .before(handle_start_join_server_event),
        );
    }
}

#[allow(clippy::type_complexity)]
fn lifecycle(
    mut commands: Commands,
    mut lc: ResMut<Lifecycle>,
    new: Query<(Entity, &Account), Added<Account>>,
    loaded: Query<Entity, Added<HasClientLoaded>>,
    mut old_brains: Query<&mut BotBrain>,
    mut disconnects: MessageReader<DisconnectEvent>,
    mut failures: MessageReader<ConnectionFailedEvent>,
    mut deaths: MessageReader<DeathEvent>,
    mut ended: MessageReader<SkillEnded>,
    mut died: MessageReader<BotDied>,
    mut reflex_off: MessageReader<ReflexDisabled>,
    retry: Query<(&Account, &ConnectOpts)>,
    unmodelled: Query<(&UnmodelledDisconnect, Option<&PacketsAfterUnmodelledDisconnect>)>,
    mut join: MessageWriter<StartJoinServerEvent>,
    mut respawn: MessageWriter<PerformRespawnEvent>,
) {
    for (e, acc) in &new {
        lc.bots.insert(
            e,
            Rec { name: acc.username().to_string(), attempt: 0, retry_at: None, spawned_at: None },
        );
        lc.emit(e, BotState::Connecting);
        // Spec: specs/F5-version-layer.md §4.6 (every bot may be disconnected by the fork)
        commands.entity(e).insert(DisconnectOnUnmodelledMovement);
    }

    for e in &loaded {
        if !lc.bots.contains_key(&e) {
            continue;
        }
        let name = lc.bots[&e].name.clone();
        lc.bots.get_mut(&e).unwrap().spawned_at = Some(Instant::now());
        lc.emit(e, BotState::Spawned);
        let mut brain = (lc.factory)(&name);
        // A skill that chose `Keep` at the death (F1 §4.4) is still in the old brain: it takes the
        // factory's place, as the same instance.
        if let Ok(mut old) = old_brains.get_mut(e) {
            brain.inherit_skill(&mut old);
        }
        let job = brain.skill.as_ref().map(|s| s.id().to_string());
        // Fresh brain on every spawn: a job does not survive a disconnect, nor a death unless its
        // skill kept it (F1 §4.4).
        commands.entity(e).insert(brain);
        lc.emit(e, job.map_or(BotState::Idle, BotState::OnJob));
    }

    for ev in ended.read() {
        if lc.bots.contains_key(&ev.entity) {
            let name = lc.bots[&ev.entity].name.clone();
            let _ = lc.tx.send(HostEvent::SkillEnded {
                bot: name,
                skill: ev.skill.clone(),
                status: ev.status.clone(),
                tick: ev.tick,
            });
            // A death ends the job but the bot is dead, not idle.
            if !matches!(&ev.status, SkillStatus::Failed(why) if why == SKILL_DIED) {
                lc.emit(ev.entity, BotState::Idle);
            }
        }
    }

    for ev in died.read() {
        if let Some(r) = lc.bots.get(&ev.entity) {
            let _ = lc.tx.send(HostEvent::Died { bot: r.name.clone(), tick: ev.tick, death: ev.death.clone() });
        }
    }

    for ev in reflex_off.read() {
        if let Some(r) = lc.bots.get(&ev.entity) {
            let _ = lc.tx.send(HostEvent::ReflexDisabled { bot: r.name.clone(), reflex: ev.reflex, tick: ev.tick });
        }
    }

    for ev in deaths.read() {
        if lc.bots.contains_key(&ev.entity) {
            // The brain stays: its reflexes see the death and the wait for the respawn (F1 §4.4).
            commands.entity(ev.entity).remove::<azalea::ItemInUse>();
            lc.emit(ev.entity, BotState::Dead);
            if lc.respawn {
                respawn.write(PerformRespawnEvent { entity: ev.entity });
                lc.emit(ev.entity, BotState::Respawning);
            }
        }
    }

    for ev in disconnects.read() {
        if !lc.bots.contains_key(&ev.entity) {
            continue;
        }
        commands.entity(ev.entity).remove::<BotBrain>();
        // Spec: specs/F5-version-layer.md §4.6 (the disconnect for a Movement entry: stay off)
        if let Ok((d, after)) = unmodelled.get(ev.entity) {
            lc.bots.get_mut(&ev.entity).unwrap().retry_at = None;
            let inputs: Vec<String> = d.inputs.iter().map(|i| format!("{i:?}")).collect();
            let packets_after = after.map_or(0, |n| n.0);
            tracing::warn!(entity = ?ev.entity, tick = d.tick, inputs = %inputs.join(","), packets_after, "disconnected for an unmodelled input; staying off");
            if let Some(r) = lc.bots.get(&ev.entity) {
                let _ = lc.tx.send(HostEvent::UnmodelledDisconnect { bot: r.name.clone(), tick: d.tick, inputs: inputs.clone(), packets_after });
            }
            lc.emit(ev.entity, BotState::UnmodelledOff { tick: d.tick, inputs, packets_after });
            continue;
        }
        let reason = ev.reason.as_ref().map(|r| r.to_string()).unwrap_or_default();
        let class = classify_kick(&reason);
        let state = match class {
            KickClass::Banned => {
                lc.bots.get_mut(&ev.entity).unwrap().retry_at = None;
                BotState::Banned
            }
            KickClass::Superseded => {
                lc.bots.get_mut(&ev.entity).unwrap().retry_at = None;
                BotState::Kicked { class, reason, retry_in_ms: None }
            }
            KickClass::Other => {
                let d = lc.schedule_retry(ev.entity);
                BotState::Kicked { class, reason, retry_in_ms: Some(d.as_millis() as u64) }
            }
        };
        lc.emit(ev.entity, state);
    }

    for ev in failures.read() {
        if !lc.bots.contains_key(&ev.entity) {
            continue;
        }
        let d = lc.schedule_retry(ev.entity);
        lc.emit(
            ev.entity,
            BotState::Offline { reason: ev.error.to_string(), retry_in_ms: d.as_millis() as u64 },
        );
    }

    let now = Instant::now();
    let due: Vec<Entity> = lc
        .bots
        .iter()
        .filter(|(_, r)| r.retry_at.is_some_and(|t| now >= t))
        .map(|(e, _)| *e)
        .collect();
    for e in due {
        lc.bots.get_mut(&e).unwrap().retry_at = None;
        if let Ok((account, opts)) = retry.get(e) {
            join.write(StartJoinServerEvent {
                account: account.clone(),
                connect_opts: opts.clone(),
                start_join_callback_tx: None,
            });
            lc.emit(e, BotState::Connecting);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_then_caps() {
        let b = Backoff { jitter: 0.0, ..Backoff::default() };
        let secs: Vec<u64> = (0..7).map(|a| b.delay(a, 0.5).as_secs()).collect();
        assert_eq!(secs, vec![2, 4, 8, 16, 32, 60, 60]);
    }

    #[test]
    fn jitter_stays_in_band() {
        let b = Backoff::default();
        let lo = b.delay(2, 0.0).as_secs_f64();
        let hi = b.delay(2, 0.999).as_secs_f64();
        assert!((lo - 6.0).abs() < 1e-6 && (hi - 10.0).abs() < 0.05, "{lo} {hi}");
    }

    #[test]
    fn kicks_are_classified() {
        assert_eq!(classify_kick("You are banned from this server"), KickClass::Banned);
        assert_eq!(
            classify_kick("You logged in from another location"),
            KickClass::Superseded
        );
        assert_eq!(classify_kick("Timed out"), KickClass::Other);
    }
}
