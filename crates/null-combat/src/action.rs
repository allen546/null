use null_bot_api::{BotCommands, EntityId, Hand};

/// Order of `DuelAction::to_vec()`. Booleans are 0/1; `select_slot` is -1 for none.
pub const ACTION_FIELDS: &[&str] = &[
    "forward", "strafe", "sprint", "jump", "sneak", "yaw", "pitch", "attack", "block", "select_slot",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DuelAction {
    pub forward: i8,
    pub strafe: i8,
    pub sprint: bool,
    pub jump: bool,
    pub sneak: bool,
    /// Absolute look the controller wants.
    pub yaw: f32,
    pub pitch: f32,
    pub attack: bool,
    pub block_with_shield: bool,
    pub select_slot: Option<u8>,
}

impl Default for DuelAction {
    fn default() -> Self {
        Self {
            forward: 0, strafe: 0, sprint: false, jump: false, sneak: false, yaw: 0.0,
            pitch: 0.0, attack: false, block_with_shield: false, select_slot: None,
        }
    }
}

impl DuelAction {
    pub fn to_vec(&self) -> Vec<f32> {
        let b = |x: bool| x as u8 as f32;
        vec![
            self.forward as f32, self.strafe as f32, b(self.sprint), b(self.jump), b(self.sneak),
            self.yaw, self.pitch, b(self.attack), b(self.block_with_shield),
            self.select_slot.map_or(-1.0, f32::from),
        ]
    }
}

/// Turns actions into commands. Walk, sprint, sneak and look are sent every tick: other systems
/// (the pathfinder stopping) can overwrite them in the game, so "already held" can be wrong.
#[derive(Default)]
pub struct ActionApplier {
    blocking: bool,
}

impl ActionApplier {
    /// Whether we believe our shield is up (our own state, see `sync_blocking`).
    pub fn blocking(&self) -> bool {
        self.blocking
    }

    /// Re-sync the shield flag when the game ended the use on its own. Not called each tick:
    /// the server's using-item flag lags our packets by a tick or two.
    pub fn sync_blocking(&mut self, using_shield: bool) {
        self.blocking = using_shield;
    }

    pub fn apply(&mut self, cmds: &mut dyn BotCommands, a: &DuelAction, target: EntityId) -> bool {
        cmds.walk(a.forward.clamp(-1, 1), a.strafe.clamp(-1, 1));
        cmds.sprint(a.sprint);
        cmds.sneak(a.sneak);
        cmds.look(a.yaw, a.pitch);
        if let Some(s) = a.select_slot {
            cmds.select_slot(s);
        }
        let was_blocking = self.blocking;
        if a.block_with_shield && !self.blocking {
            cmds.use_item(Hand::Off);
            self.blocking = true;
        } else if !a.block_with_shield && self.blocking {
            cmds.release_use();
            self.blocking = false;
        }
        if a.jump {
            cmds.jump();
        }
        // Vanilla drops attack clicks while an item is in use: lower the shield first,
        // swing on a later tick. Returns false when the attack was held back.
        if a.attack && was_blocking {
            if self.blocking {
                cmds.release_use();
                self.blocking = false;
            }
            return false;
        }
        if a.attack {
            cmds.attack(target);
        }
        true
    }

    /// Lets go of everything; used when a duel ends.
    pub fn release_all(&mut self, cmds: &mut dyn BotCommands) {
        if self.blocking {
            cmds.release_use();
        }
        cmds.stop();
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use null_bot_api::{Command, fake::FakeCommands};

    #[test]
    fn shield_comes_down_a_tick_before_the_swing() {
        let (mut ap, mut cmds) = (ActionApplier::default(), FakeCommands::default());
        let block = DuelAction { block_with_shield: true, ..DuelAction::default() };
        let swing = DuelAction { attack: true, ..DuelAction::default() };
        ap.apply(&mut cmds, &block, EntityId(7));
        assert!(!ap.apply(&mut cmds, &swing, EntityId(7)), "attack held while blocking");
        assert!(ap.apply(&mut cmds, &swing, EntityId(7)));
        let kinds: Vec<_> = cmds.log.iter().map(|(_, c)| c.clone()).filter(|c| {
            matches!(c, Command::UseItem(_) | Command::ReleaseUse | Command::Attack(_))
        }).collect();
        assert_eq!(kinds, vec![Command::UseItem(Hand::Off), Command::ReleaseUse, Command::Attack(EntityId(7))]);
    }

    #[test]
    fn walk_is_resent_every_tick() {
        // The pathfinder's stop can clear our walk after we set it; an unchanged action must still re-assert it.
        let (mut ap, mut cmds) = (ActionApplier::default(), FakeCommands::default());
        let fwd = DuelAction { forward: 1, ..DuelAction::default() };
        ap.apply(&mut cmds, &fwd, EntityId(7));
        ap.apply(&mut cmds, &fwd, EntityId(7));
        let walks = cmds.log.iter().filter(|(_, c)| matches!(c, Command::Walk { forward: 1, strafe: 0 })).count();
        assert_eq!(walks, 2);
    }
}
