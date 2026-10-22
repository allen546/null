//! Bot-side ops for A20 (client refusals, fork patch 34). `RemoteSkill::step` calls `step` for
//! `Op::Refuse`; everything goes through `BotView` / `BotCommands` only. Entity clicks reuse
//! `InvOp::OpenEntity`.
// Spec: specs/F5-version-layer.md §4.6 (client refusals: elytra, riptide, item use effects)

use null_bot_api::{BotCommands, BotView, Hand, InputRefusal, Vec3};

use crate::skill::Info;

/// The op gives up after this many ticks, before the runner's 60 s wait.
const MAX_TICKS: u64 = 120;
/// Ticks after the input before the op reports: the refusal is decided in the same tick.
const SETTLE_TICKS: u64 = 6;

#[derive(Clone, Debug)]
pub enum RefuseOp {
    /// Wait until the bot is in the air, then press jump once and report.
    AirJump,
    /// Use the main-hand item once and report.
    UseMain,
    /// Report the refusal counts now (a baseline: the log is never reset).
    Status,
}

#[derive(Clone, Debug)]
pub struct RefuseInfo {
    /// `BotView::refused_inputs` when the op ended.
    pub refused: Vec<InputRefusal>,
    pub on_ground: bool,
    pub pos: Vec3,
    /// The input went out (an air jump needs the bot to be in the air first).
    pub issued: bool,
    /// The tick count the bot was in the air when it jumped.
    pub air_ticks: u64,
}

fn info(v: &dyn BotView, issued: bool, air_ticks: u64) -> Info {
    Info::Refuse(RefuseInfo { refused: v.refused_inputs(), on_ground: v.on_ground(), pos: v.position(), issued, air_ticks })
}

/// Per-op state: the age the input went out at.
#[derive(Default)]
pub struct RefuseState {
    issued_at: Option<u64>,
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: elytra, riptide, item use effects)
pub fn step(op: &RefuseOp, age: u64, v: &dyn BotView, c: &mut dyn BotCommands, st: &mut RefuseState) -> Option<Info> {
    if age == 0 {
        *st = RefuseState::default();
    }
    match op {
        RefuseOp::AirJump => {
            if st.issued_at.is_none() && !v.on_ground() && age >= 2 {
                c.jump();
                st.issued_at = Some(age);
            }
            match st.issued_at {
                Some(at) if age >= at + SETTLE_TICKS => Some(info(v, true, at)),
                None if age > MAX_TICKS => Some(info(v, false, 0)),
                _ => None,
            }
        }
        RefuseOp::Status => Some(info(v, false, 0)),
        RefuseOp::UseMain => {
            if age == 0 {
                c.use_item(Hand::Main);
                st.issued_at = Some(0);
            }
            (age >= SETTLE_TICKS).then(|| info(v, true, 0))
        }
    }
}
