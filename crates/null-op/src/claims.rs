//! Claims registry: every claim has an owner, a priority and an expiry (K5 §4.5, CONVENTIONS §3.3).
//! The registry is pure: callers pass the game tick. Each change is also recorded as a [`ClaimEvent`] for the log.
use null_proto::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Spec: specs/K5-operation-service.md §4.5
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: ClaimKind,
    pub owner: ClaimOwner,
    pub priority: Priority,
    pub token: u64,
    pub expires_at: Gt,
}

/// What happened to a claim. The log stores these; [`ClaimRegistry::apply`] replays them (K5 §4.9).
// Spec: specs/K5-operation-service.md §4.9
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimEvent {
    Granted {
        id: ClaimId,
        kind: ClaimKind,
        owner: ClaimOwner,
        priority: Priority,
        token: u64,
        expires_at: Gt,
    },
    Renewed {
        id: ClaimId,
        expires_at: Gt,
    },
    Released {
        id: ClaimId,
    },
    Expired {
        id: ClaimId,
        owner: ClaimOwner,
    },
    /// Taken away by a higher priority. `owner` is the old owner.
    Preempted {
        id: ClaimId,
        owner: ClaimOwner,
        by: ClaimOwner,
    },
}

// Spec: specs/K5-operation-service.md §4.5
#[derive(Default)]
pub struct ClaimRegistry {
    entries: BTreeMap<ClaimId, Entry>,
    /// Preempted ids, kept until the old owner asks again, so its renew gets `Preempt` and not `NotHeld`.
    preempted: BTreeMap<ClaimId, ClaimOwner>,
    next_token: u64,
    pending: Vec<ClaimEvent>,
}

/// True when two claims cannot both hold.
// Spec: specs/K5-operation-service.md §4.5
fn overlaps(a: &ClaimKind, b: &ClaimKind) -> bool {
    use ClaimKind::*;
    match (a, b) {
        (BlockSet(x), BlockSet(y)) => x.iter().any(|l| y.contains(l)),
        (Region(x), Region(y)) => x.overlaps(y),
        (Region(r), BlockSet(s)) | (BlockSet(s), Region(r)) => s.iter().any(|l| r.contains(l)),
        (Container(x), Container(y))
        | (Container(x), Items { container: y, .. })
        | (Items { container: x, .. }, Container(y)) => x == y,
        (
            Items {
                container: x,
                item: i,
                ..
            },
            Items {
                container: y,
                item: j,
                ..
            },
        ) => x == y && i == j,
        (Entity(x), Entity(y)) => x == y,
        (Bot(x), Bot(y)) => x == y,
        _ => false,
    }
}

impl ClaimRegistry {
    // Spec: specs/K5-operation-service.md §4.5
    pub fn handle(&mut self, now: Gt, req: &ClaimRequest) -> ClaimReply {
        self.expire(now);
        match req {
            ClaimRequest::Take {
                id,
                kind,
                owner,
                priority,
                ttl,
            } => {
                // A live id taken again by its own owner is a resend, not a new grant: it renews (same token, so the
                // holder's fence stays valid) when the block set is the same, and is denied when it differs. A silent
                // replace could change a held claim's kind or token without the holder knowing.
                if let Some(e) = self.entries.get_mut(id).filter(|e| &e.owner == owner) {
                    if &e.kind != kind {
                        return ClaimReply::Denied {
                            id: id.clone(),
                            reason: DenyReason::Held { by: owner.clone() },
                        };
                    }
                    e.expires_at = now + *ttl;
                    let (token, expires_at) = (e.token, e.expires_at);
                    self.pending.push(ClaimEvent::Renewed {
                        id: id.clone(),
                        expires_at,
                    });
                    return ClaimReply::Granted {
                        id: id.clone(),
                        token,
                        expires_at,
                    };
                }
                // Same id counts as a conflict, so a re-take under another owner cannot slip past. The same owner
                // conflicts too: an old attempt's take under a new id must not hold a tree twice (S3a check P1).
                let conflicts: Vec<ClaimId> = self
                    .entries
                    .iter()
                    .filter(|(i, e)| *i == id || overlaps(&e.kind, kind))
                    .map(|(i, _)| i.clone())
                    .collect();
                if let Some(blocker) = conflicts
                    .iter()
                    .map(|i| &self.entries[i])
                    .find(|e| e.priority >= *priority)
                {
                    return ClaimReply::Denied {
                        id: id.clone(),
                        reason: DenyReason::Held {
                            by: blocker.owner.clone(),
                        },
                    };
                }
                for c in conflicts {
                    let old = self.entries.remove(&c).expect("listed above");
                    self.preempted.insert(c.clone(), owner.clone());
                    self.pending.push(ClaimEvent::Preempted {
                        id: c,
                        owner: old.owner,
                        by: owner.clone(),
                    });
                }
                self.preempted.remove(id);
                self.next_token += 1;
                let (token, expires_at) = (self.next_token, now + *ttl);
                let ev = ClaimEvent::Granted {
                    id: id.clone(),
                    kind: kind.clone(),
                    owner: owner.clone(),
                    priority: *priority,
                    token,
                    expires_at,
                };
                self.apply(&ev);
                self.pending.push(ev);
                ClaimReply::Granted {
                    id: id.clone(),
                    token,
                    expires_at,
                }
            }
            ClaimRequest::Renew { id, owner, ttl } => match self.check(id, owner) {
                Err(r) => r,
                Ok(e) => {
                    e.expires_at = now + *ttl;
                    let (token, expires_at) = (e.token, e.expires_at);
                    self.pending.push(ClaimEvent::Renewed {
                        id: id.clone(),
                        expires_at,
                    });
                    ClaimReply::Granted {
                        id: id.clone(),
                        token,
                        expires_at,
                    }
                }
            },
            ClaimRequest::Release { id, owner } => match self.check(id, owner) {
                Err(r) => r,
                Ok(_) => {
                    self.entries.remove(id);
                    self.pending.push(ClaimEvent::Released { id: id.clone() });
                    ClaimReply::Released { id: id.clone() }
                }
            },
        }
    }

    /// The entry if `owner` holds it, else the reply to send.
    // Spec: specs/K5-operation-service.md §4.5
    fn check(&mut self, id: &ClaimId, owner: &ClaimOwner) -> Result<&mut Entry, ClaimReply> {
        if let Some(by) = self.preempted.get(id) {
            if !self.entries.contains_key(id) {
                return Err(ClaimReply::Preempt {
                    id: id.clone(),
                    by: by.clone(),
                });
            }
        }
        match self.entries.get_mut(id) {
            None => Err(ClaimReply::Denied {
                id: id.clone(),
                reason: DenyReason::NotHeld,
            }),
            Some(e) if &e.owner != owner => Err(ClaimReply::Denied {
                id: id.clone(),
                reason: DenyReason::NotOwner,
            }),
            Some(e) => Ok(e),
        }
    }

    /// Frees every claim whose time is up. A claim frees itself: nobody has to release it.
    // Spec: specs/K5-operation-service.md §4.5
    pub fn expire(&mut self, now: Gt) {
        let dead: Vec<ClaimId> = self
            .entries
            .iter()
            .filter(|(_, e)| e.expires_at <= now)
            .map(|(i, _)| i.clone())
            .collect();
        for id in dead {
            let e = self.entries.remove(&id).expect("listed above");
            self.pending
                .push(ClaimEvent::Expired { id, owner: e.owner });
        }
    }

    /// Replay one logged change (restart, K5 §4.9). Does not record a new event.
    // Spec: specs/K5-operation-service.md §4.9
    pub fn apply(&mut self, ev: &ClaimEvent) {
        match ev {
            ClaimEvent::Granted {
                id,
                kind,
                owner,
                priority,
                token,
                expires_at,
            } => {
                self.next_token = self.next_token.max(*token);
                self.preempted.remove(id);
                self.entries.insert(
                    id.clone(),
                    Entry {
                        kind: kind.clone(),
                        owner: owner.clone(),
                        priority: *priority,
                        token: *token,
                        expires_at: *expires_at,
                    },
                );
            }
            ClaimEvent::Renewed { id, expires_at } => {
                if let Some(e) = self.entries.get_mut(id) {
                    e.expires_at = *expires_at;
                }
            }
            ClaimEvent::Released { id } | ClaimEvent::Expired { id, .. } => {
                self.entries.remove(id);
            }
            ClaimEvent::Preempted { id, by, .. } => {
                self.entries.remove(id);
                self.preempted.insert(id.clone(), by.clone());
            }
        }
    }

    /// Changes since the last call, in order.
    pub fn drain_events(&mut self) -> Vec<ClaimEvent> {
        std::mem::take(&mut self.pending)
    }
    pub fn get(&self, id: &ClaimId) -> Option<&Entry> {
        self.entries.get(id)
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    /// Held claims, for comparing state after a restart.
    pub fn snapshot(&self) -> Vec<(ClaimId, Entry)> {
        self.entries
            .iter()
            .map(|(i, e)| (i.clone(), e.clone()))
            .collect()
    }
}
