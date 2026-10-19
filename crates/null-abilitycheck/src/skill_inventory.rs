//! Bot-side ops for A9 (inventory clicks). `RemoteSkill::step` calls `step` for `Op::Inv`;
//! everything goes through `BotView` / `BotCommands` only.

use null_bot_api::{
    BlockPos, BotCommands, BotView, Click, ClickOutcome, EntityId, Face, ItemStack, OpenOutcome, OpenResult,
    Vec3, Window,
};

use crate::skill::Info;

/// Ticks to wait after a click or a close: the server answers on its next tick, and any slot it
/// corrects comes back within a few.
const SETTLE_TICKS: u64 = 8;

#[derive(Clone, Debug)]
pub enum InvOp {
    Open,
    Close,
    /// Look at a container block, right-click it and wait for the answer (B3 §4.7).
    OpenContainer(BlockPos, Option<Face>),
    /// Wait until the bot sees an entity of this kind (A10E).
    WaitEntity(&'static str),
    /// Right-click the nearest entity of this kind and wait for the answer (A10E, B3 §4.7).
    OpenEntity { kind: &'static str, auto_crouch: bool, aim: Aim },
    /// Click a window slot (`None` = outside the window).
    Click(Option<u16>, Click),
    Status,
}

/// How the bot turns before an entity click.
#[derive(Clone, Copy, Debug)]
pub enum Aim {
    /// `look_at_entity`: a point of the hitbox the crosshair reaches.
    Entity,
    /// Face this yaw and pitch (degrees): a deliberately bad or fixed aim.
    Look(f32, f32),
    /// Leave the look as it is.
    Keep,
}

#[derive(Clone, Debug)]
pub struct InvInfo {
    pub window: Option<Window>,
    pub click: Option<ClickOutcome>,
    pub open: Option<OpenResult>,
    pub open_refusal: Option<&'static str>,
    /// The entity an entity op found: its id and position.
    pub seen: Option<(EntityId, Vec3)>,
    /// `BotView::sneaking` when the info was taken.
    pub sneaking: bool,
    /// `BotView::inventory`: 0-8 hotbar, 9-35 main.
    pub inventory: Vec<Option<ItemStack>>,
    pub offhand: Option<ItemStack>,
}

fn info(v: &dyn BotView) -> InvInfo {
    InvInfo {
        window: v.window().cloned(),
        click: v.last_click(),
        open: v.last_open(),
        open_refusal: v.open_refusal(),
        seen: None,
        sneaking: v.sneaking(),
        inventory: v.inventory().to_vec(),
        offhand: v.offhand().cloned(),
    }
}

// Spec: specs/B3-inventory-crafting.md §4.1 (open, click and close as a vanilla client does)
pub fn step(op: &InvOp, age: u64, v: &dyn BotView, c: &mut dyn BotCommands) -> Option<Info> {
    let wait = match op {
        InvOp::Open => {
            if age == 0 {
                c.open_inventory();
            }
            2
        }
        InvOp::OpenContainer(pos, face) => {
            // The look needs two ticks to land before the click.
            match age {
                0 => c.look_at_block(*pos, *face),
                2 => c.open_container(*pos, *face),
                _ => {}
            }
            if age < 3 {
                return None;
            }
            let settled = v.last_open().is_some_and(|o| o.outcome != OpenOutcome::Pending);
            // Contents arrive with the open; clicks wait a tick after it.
            return (age >= 80 || (settled && age >= 8)).then(|| Info::Inv(info(v)));
        }
        // Spec: specs/B3-inventory-crafting.md §4.7 (entity containers)
        InvOp::WaitEntity(kind) => {
            let seen = nearest(v, kind);
            return (seen.is_some() || age >= 100).then(|| Info::Inv(InvInfo { seen, ..info(v) }));
        }
        // Spec: specs/B3-inventory-crafting.md §4.7 (aim at an entity, right-click it, read the answer)
        InvOp::OpenEntity { kind, auto_crouch, aim } => {
            let seen = nearest(v, kind);
            let Some((id, _)) = seen else {
                return (age >= 20).then(|| Info::Inv(info(v)));
            };
            // The look needs two ticks to land before the click.
            match (age, aim) {
                (0, Aim::Entity) => c.look_at_entity(id),
                (0, Aim::Look(yaw, pitch)) => c.look(*yaw, *pitch),
                (2, _) => c.open_entity(id, *auto_crouch),
                _ => {}
            }
            if age < 3 {
                return None;
            }
            let settled = v.last_open().is_some_and(|o| o.outcome != OpenOutcome::Pending);
            // A crouching open stands up again; contents arrive with the open.
            return (age >= 80 || (settled && age >= 12)).then(|| Info::Inv(InvInfo { seen, ..info(v) }));
        }
        InvOp::Close => {
            if age == 0 {
                c.close_window();
            }
            SETTLE_TICKS
        }
        InvOp::Click(slot, click) => {
            if age == 0 {
                c.click_slot(*slot, *click);
            }
            SETTLE_TICKS
        }
        InvOp::Status => 0,
    };
    (age >= wait).then(|| Info::Inv(info(v)))
}

/// The nearest entity of `kind` the bot sees, with its position.
fn nearest(v: &dyn BotView, kind: &str) -> Option<(EntityId, Vec3)> {
    let me = v.position();
    v.entities()
        .iter()
        .filter(|e| e.kind == kind)
        .min_by(|a, b| dist2(a.position, me).total_cmp(&dist2(b.position, me)))
        .map(|e| (e.id, e.position))
}

fn dist2(a: Vec3, b: Vec3) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)
}
