//! Typed time (CONVENTIONS §3.2): game ticks for game things, wall milliseconds for network things.
// Spec: specs/CONVENTIONS.md §3.2

use serde::{Deserialize, Serialize};
use std::ops::{Add, Sub};

macro_rules! time {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub u64);
        impl Add for $name {
            type Output = $name;
            fn add(self, o: $name) -> $name { $name(self.0.saturating_add(o.0)) }
        }
        impl Sub for $name {
            type Output = $name;
            fn sub(self, o: $name) -> $name { $name(self.0.saturating_sub(o.0)) }
        }
    };
}

// Game ticks (`gt`).
// Spec: specs/CONVENTIONS.md §3.2
time!(Gt);
// Wall-clock milliseconds (`ms`).
// Spec: specs/CONVENTIONS.md §3.2
time!(WallMs);

impl WallMs {
    pub fn as_duration(self) -> std::time::Duration {
        std::time::Duration::from_millis(self.0)
    }
}

/// Ticks before a lease ends at which a bot stops by itself (lease rule, K6 §4.8). Squad and bots share it.
// Spec: specs/K6-squad-leader.md §4.8
pub const LEASE_MARGIN: u64 = 10;
