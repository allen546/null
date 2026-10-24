//! Id newtypes (CONVENTIONS §3.1, F6 §4.2). All are plain strings so HQ and tests can name them.
// Spec: specs/F6-bus-deployment.md §4.2

use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! id {
    ($($name:ident),*) => {$(
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
        impl $name {
            pub fn new(s: impl Into<String>) -> Self { Self(s.into()) }
            pub fn as_str(&self) -> &str { &self.0 }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(&self.0) }
        }
    )*};
}

// `BotId` is the account name. `ItemId` is the namespaced item name, e.g. "minecraft:oak_log".
id!(ServerId, OperationId, SquadId, BotId, JobId, ClaimId, GoalId, ItemId);
