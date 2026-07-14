use bevy::prelude::*;
use std::collections::HashMap;

use crate::nation::NationId;
use crate::resource::ResourceKind;

/// Maximum number of characters allowed in a player display name across the wire.
pub const PLAYER_NAME_MAX_CHARS: usize = 20;

/// Strip control characters and non-ASCII bytes from a player-supplied name,
/// trim the result, and clamp it to [`PLAYER_NAME_MAX_CHARS`] characters.
///
/// This is the same defensive chain golab's `shared::sanitize_player_name`
/// uses; here it lives on `lk2-core` so both client and server agree on the
/// canonical form before names ever touch the network.
#[must_use]
pub fn sanitize_player_name(input: &str) -> String {
    input
        .chars()
        .filter(|character| character.is_ascii() && !character.is_ascii_control())
        .take(PLAYER_NAME_MAX_CHARS)
        .collect::<String>()
        .trim()
        .to_string()
}

#[derive(Debug, Clone, Resource, Default)]
pub struct PlayerState {
    pub pos: Vec3,
    pub block_pos: [i32; 3],
    pub inventory: HashMap<ResourceKind, i64>,
    pub nation_id: Option<NationId>,
    pub monsters_killed: u32,
    pub blocks_gathered: u32,
    pub nations_founded: u32,
}

#[derive(Component, Debug, Default)]
pub struct PlayerStateComponent(pub PlayerState);

#[derive(Component, Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PlayerTag(pub u32);

impl PlayerTag {
    pub fn id(&self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_state_default_is_empty() {
        let p = PlayerState::default();
        assert_eq!(p.pos, Vec3::ZERO);
        assert_eq!(p.block_pos, [0, 0, 0]);
        assert!(p.inventory.is_empty());
        assert_eq!(p.nation_id, None);
        assert_eq!(p.monsters_killed, 0);
        assert_eq!(p.blocks_gathered, 0);
        assert_eq!(p.nations_founded, 0);
    }

    #[test]
    fn player_state_update_fields() {
        let mut p = PlayerState::default();
        p.pos = Vec3::new(10.0, 5.0, 20.0);
        p.block_pos = [10, 5, 20];
        p.nation_id = Some(NationId(42));
        p.monsters_killed = 10;
        p.blocks_gathered = 100;
        p.nations_founded = 3;

        assert_eq!(p.pos, Vec3::new(10.0, 5.0, 20.0));
        assert_eq!(p.block_pos, [10, 5, 20]);
        assert_eq!(p.nation_id, Some(NationId(42)));
        assert_eq!(p.monsters_killed, 10);
        assert_eq!(p.blocks_gathered, 100);
        assert_eq!(p.nations_founded, 3);
    }

    #[test]
    fn player_state_inventory_ops() {
        let mut p = PlayerState::default();
        p.inventory.insert(ResourceKind::Wood, 100);
        p.inventory.insert(ResourceKind::Food, 50);

        assert_eq!(p.inventory.get(&ResourceKind::Wood), Some(&100));
        assert_eq!(p.inventory.get(&ResourceKind::Food), Some(&50));
        assert_eq!(p.inventory.get(&ResourceKind::Apple), None);

        *p.inventory.get_mut(&ResourceKind::Wood).unwrap() += 20;
        assert_eq!(p.inventory.get(&ResourceKind::Wood), Some(&120));
    }

    #[test]
    fn player_tag_id() {
        let tag = PlayerTag(123);
        assert_eq!(tag.id(), 123);
        assert_eq!(tag.0, 123);
    }

    #[test]
    fn player_tag_eq_hash() {
        let tag1 = PlayerTag(42);
        let tag2 = PlayerTag(42);
        let tag3 = PlayerTag(99);

        assert_eq!(tag1, tag2);
        assert_ne!(tag1, tag3);

        let mut map = std::collections::HashMap::new();
        map.insert(tag1, "value");
        assert_eq!(map.get(&tag2), Some(&"value"));
    }

    #[test]
    fn sanitize_player_name_strips_controls_and_clamps() {
        assert_eq!(sanitize_player_name("Alice"), "Alice");
        assert_eq!(sanitize_player_name("  Bob\n"), "Bob");
        assert_eq!(sanitize_player_name("Eve\u{0000}\u{0007}"), "Eve");
        assert_eq!(sanitize_player_name("Caesar"), "Caesar");
        // ASCII clamp at PLAYER_NAME_MAX_CHARS.
        let long = "a".repeat(PLAYER_NAME_MAX_CHARS + 50);
        let cleaned = sanitize_player_name(&long);
        assert_eq!(cleaned.len(), PLAYER_NAME_MAX_CHARS);
        // Non-ASCII characters are filtered out (must_use ASCII only).
        assert_eq!(sanitize_player_name("中文名"), "");
        assert_eq!(sanitize_player_name(""), "");
        // Trim still runs after take() so leading whitespace goes away.
        assert_eq!(sanitize_player_name("   trailing   "), "trailing");
    }
}
