//! Bot-side ops for A9 (inventory clicks). `RemoteSkill::step` calls `step` for `Op::Inv`;
//! everything goes through `BotView` / `BotCommands` only.

use null_bot_api::{
    BlockPos, BotCommands, BotView, Click, ClickOutcome, Face, ItemStack, OpenOutcome, OpenResult, Window,
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
    /// Click a window slot (`None` = outside the window).
    Click(Option<u16>, Click),
    Status,
}

#[derive(Clone, Debug)]
pub struct InvInfo {
    pub window: Option<Window>,
    pub click: Option<ClickOutcome>,
    pub open: Option<OpenResult>,
    pub open_refusal: Option<&'static str>,
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
