//! Message types every level shares (S2, specs/plans/BUILD_PLAN.md). Decision crate: no azalea, no bevy.
//! The wire format is JSON via serde; every message crosses a bus inside an [`Envelope`].

pub mod bot;
pub mod claim;
pub mod command;
pub mod goal;
pub mod ids;
pub mod job;
pub mod loc;
pub mod time;
pub mod upward;

pub use bot::*;
pub use claim::*;
pub use command::*;
pub use goal::*;
pub use ids::*;
pub use job::*;
pub use loc::*;
pub use time::*;
pub use upward::*;

use serde::{Deserialize, Serialize};

/// Schema version. A process rejects a message with another major (F6 §4.2).
// Spec: specs/F6-bus-deployment.md §4.2
pub const PROTO_VERSION: ProtoVersion = ProtoVersion { major: 1, minor: 0 };

// Spec: specs/F6-bus-deployment.md §4.2
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtoVersion {
    pub major: u16,
    pub minor: u16,
}

/// Wire wrapper: the version travels with every message.
// Spec: specs/F6-bus-deployment.md §4.2
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub proto_version: ProtoVersion,
    pub body: T,
}

// Spec: specs/F6-bus-deployment.md §4.2
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionMismatch {
    pub got: ProtoVersion,
}

impl<T> Envelope<T> {
    pub fn new(body: T) -> Self {
        Self { proto_version: PROTO_VERSION, body }
    }
    /// The body, or an error when the major version differs from ours.
    pub fn open(self) -> Result<T, VersionMismatch> {
        if self.proto_version.major == PROTO_VERSION.major {
            Ok(self.body)
        } else {
            Err(VersionMismatch { got: self.proto_version })
        }
    }
}

#[cfg(test)]
mod tests;
