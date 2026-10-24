//! The operation service (K5, skeleton): claims registry, `Obtain` expansion, scheduler, ledger, summaries and
//! restart from the event log. Decision crate: no azalea, no bevy. The squad and bots sit behind S2 messages only.

mod claims;
mod events;
mod op;
#[cfg(test)]
mod tests;

pub use claims::{ClaimEvent, ClaimRegistry, Entry};
pub use events::{EventLog, OpEvent, decode, load_jsonl, wall_of};
pub use op::{
    JobSt, OpConfig, OpPorts, Operation, claim_id, hq_escalate, op_claims, op_cmd, op_events,
    op_intents, op_summary, squad_jobs,
};
