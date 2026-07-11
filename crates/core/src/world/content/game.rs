use super::{ContentCatalog, ContentGenerationError, ContentId, ContentVolume, Direction3};
use serde::{Deserialize, Serialize};

pub const GAME_CONTENT_EMPTY: ContentId = ContentId(0);
pub const GAME_CONTENT_WILDERNESS: ContentId = ContentId(1);
pub const GAME_CONTENT_SETTLEMENT: ContentId = ContentId(2);
pub const GAME_CONTENT_MONSTER_TERRITORY: ContentId = ContentId(3);
pub const GAME_CONTENT_CAVERN: ContentId = ContentId(4);
pub const GAME_CONTENT_DUNGEON: ContentId = ContentId(5);
pub const GAME_CONTENT_TREASURE_VAULT: ContentId = ContentId(6);
pub const GAME_CONTENT_VERTICAL_PASSAGE: ContentId = ContentId(7);

const SURFACE_CONTENT: [ContentId; 4] = [
    GAME_CONTENT_WILDERNESS,
    GAME_CONTENT_SETTLEMENT,
    GAME_CONTENT_MONSTER_TERRITORY,
    GAME_CONTENT_VERTICAL_PASSAGE,
];
const UNDERGROUND_CONTENT: [ContentId; 4] = [
    GAME_CONTENT_CAVERN,
    GAME_CONTENT_DUNGEON,
    GAME_CONTENT_TREASURE_VAULT,
    GAME_CONTENT_VERTICAL_PASSAGE,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameContentTheme {
    Frontier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentSpiceProfile {
    HomesteadWilds,
    MonsterMarch,
    CrystalDescent,
    CavernGarden,
}

impl ContentSpiceProfile {
    pub const ALL: [Self; 4] = [
        Self::HomesteadWilds,
        Self::MonsterMarch,
        Self::CrystalDescent,
        Self::CavernGarden,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameContentVolumeConfig {
    pub dimensions: [usize; 3],
    pub seed: u64,
    pub profile: Option<ContentSpiceProfile>,
}

impl GameContentVolumeConfig {
    pub const fn new(dimensions: [usize; 3], seed: u64) -> Self {
        Self {
            dimensions,
            seed,
            profile: None,
        }
    }

    #[must_use]
    pub const fn with_profile(mut self, profile: ContentSpiceProfile) -> Self {
        self.profile = Some(profile);
        self
    }
}

pub fn game_content_catalog(theme: GameContentTheme) -> ContentCatalog {
    match theme {
        GameContentTheme::Frontier => frontier_catalog(),
    }
}

pub fn generate_game_content_volume(
    catalog: &ContentCatalog,
    config: &GameContentVolumeConfig,
) -> Result<ContentVolume, ContentGenerationError> {
    if config.dimensions[0] < 5 || config.dimensions[1] < 3 || config.dimensions[2] < 5 {
        return Err(ContentGenerationError::EmptyDimensions);
    }

    let profile = config
        .profile
        .unwrap_or_else(|| choose_spice_profile(config.seed));
    let [width, height, depth] = config.dimensions;
    let mut volume = ContentVolume::new(
        config.dimensions,
        config.seed,
        vec![GAME_CONTENT_EMPTY; width * height * depth],
    );

    for y in 0..config.dimensions[1] {
        let layer_content = match y {
            0 => underground_background(profile),
            1 => surface_background(profile),
            _ => GAME_CONTENT_EMPTY,
        };
        for z in 0..config.dimensions[2] {
            for x in 0..config.dimensions[0] {
                volume.set([x, y, z], layer_content);
            }
        }
    }

    let center = [config.dimensions[0] / 2, 1, config.dimensions[2] / 2];
    let underground_center = [config.dimensions[0] / 2, 0, config.dimensions[2] / 2];
    let monster = [1, 1, 1];
    let treasure = [config.dimensions[0] - 2, 0, config.dimensions[2] - 2];

    apply_surface_spice(&mut volume, profile, config.seed);
    apply_underground_spice(&mut volume, profile, underground_center, treasure);
    protect_treasure_neighbors(&mut volume, treasure);

    volume.set(center, GAME_CONTENT_SETTLEMENT);
    volume.set(underground_center, GAME_CONTENT_VERTICAL_PASSAGE);
    volume.set(monster, GAME_CONTENT_MONSTER_TERRITORY);
    volume.set(treasure, GAME_CONTENT_TREASURE_VAULT);

    volume.validate(catalog)?;
    Ok(volume)
}

fn frontier_catalog() -> ContentCatalog {
    let mut builder = ContentCatalog::builder()
        .archetype(GAME_CONTENT_EMPTY, "empty", 1)
        .archetype(GAME_CONTENT_WILDERNESS, "wilderness", 12)
        .archetype(GAME_CONTENT_SETTLEMENT, "settlement", 2)
        .archetype(GAME_CONTENT_MONSTER_TERRITORY, "monster_territory", 3)
        .archetype(GAME_CONTENT_CAVERN, "cavern", 10)
        .archetype(GAME_CONTENT_DUNGEON, "dungeon", 3)
        .archetype(GAME_CONTENT_TREASURE_VAULT, "treasure_vault", 1)
        .archetype(GAME_CONTENT_VERTICAL_PASSAGE, "vertical_passage", 2)
        .allow_same_position_neighbors(GAME_CONTENT_EMPTY);

    for &first in &SURFACE_CONTENT {
        for &second in &SURFACE_CONTENT {
            builder = allow_horizontal(builder, first, second);
        }
        builder = builder.allow_bidirectional(first, Direction3::PosY, GAME_CONTENT_EMPTY);
    }

    for &first in &UNDERGROUND_CONTENT {
        for &second in &UNDERGROUND_CONTENT {
            let treasure_pair =
                first == GAME_CONTENT_TREASURE_VAULT || second == GAME_CONTENT_TREASURE_VAULT;
            let treasure_supported = matches!(
                (first, second),
                (
                    GAME_CONTENT_TREASURE_VAULT,
                    GAME_CONTENT_DUNGEON | GAME_CONTENT_TREASURE_VAULT
                ) | (GAME_CONTENT_DUNGEON, GAME_CONTENT_TREASURE_VAULT)
            );
            if !treasure_pair || treasure_supported {
                builder = allow_horizontal(builder, first, second);
            }
        }
        for &surface in &SURFACE_CONTENT {
            builder = builder.allow_bidirectional(first, Direction3::PosY, surface);
        }
    }

    builder
        .build()
        .expect("frontier content catalog is internally valid")
}

fn allow_horizontal(
    mut builder: super::ContentCatalogBuilder,
    first: ContentId,
    second: ContentId,
) -> super::ContentCatalogBuilder {
    builder = builder.allow_bidirectional(first, Direction3::PosX, second);
    builder = builder.allow_bidirectional(second, Direction3::PosX, first);
    builder = builder.allow_bidirectional(first, Direction3::PosZ, second);
    builder.allow_bidirectional(second, Direction3::PosZ, first)
}

fn choose_spice_profile(seed: u64) -> ContentSpiceProfile {
    let hash = mix64(seed ^ 0x51A1_CE5E_ED5);
    let rare_roll = (hash >> 56) as u8;
    if rare_roll < 24 {
        return ContentSpiceProfile::CrystalDescent;
    }
    match (hash % 3) as usize {
        0 => ContentSpiceProfile::HomesteadWilds,
        1 => ContentSpiceProfile::MonsterMarch,
        _ => ContentSpiceProfile::CavernGarden,
    }
}

fn surface_background(profile: ContentSpiceProfile) -> ContentId {
    match profile {
        ContentSpiceProfile::MonsterMarch => GAME_CONTENT_MONSTER_TERRITORY,
        _ => GAME_CONTENT_WILDERNESS,
    }
}

fn underground_background(profile: ContentSpiceProfile) -> ContentId {
    match profile {
        ContentSpiceProfile::CrystalDescent => GAME_CONTENT_DUNGEON,
        _ => GAME_CONTENT_CAVERN,
    }
}

fn apply_surface_spice(volume: &mut ContentVolume, profile: ContentSpiceProfile, seed: u64) {
    let [width, _, depth] = volume.dimensions;
    match profile {
        ContentSpiceProfile::HomesteadWilds => {
            for z in 0..depth {
                for x in 0..width {
                    if (x + z) % 4 == 0 {
                        volume.set([x, 1, z], GAME_CONTENT_WILDERNESS);
                    }
                }
            }
        }
        ContentSpiceProfile::MonsterMarch => {
            for z in 0..depth {
                for x in 0..width {
                    if x > width / 2 && z > depth / 2 {
                        volume.set([x, 1, z], GAME_CONTENT_WILDERNESS);
                    }
                }
            }
        }
        ContentSpiceProfile::CrystalDescent => {
            let mid_x = width / 2;
            let mid_z = depth / 2;
            for z in 0..depth {
                volume.set([mid_x, 1, z], GAME_CONTENT_VERTICAL_PASSAGE);
            }
            for x in 0..width {
                volume.set([x, 1, mid_z], GAME_CONTENT_VERTICAL_PASSAGE);
            }
        }
        ContentSpiceProfile::CavernGarden => {
            for z in 0..depth {
                for x in 0..width {
                    if spice_hash(seed, x, z) % 5 == 0 {
                        volume.set([x, 1, z], GAME_CONTENT_VERTICAL_PASSAGE);
                    }
                }
            }
        }
    }
}

fn apply_underground_spice(
    volume: &mut ContentVolume,
    profile: ContentSpiceProfile,
    start: [usize; 3],
    treasure: [usize; 3],
) {
    let [width, _, depth] = volume.dimensions;
    match profile {
        ContentSpiceProfile::HomesteadWilds => {
            carve_dungeon_path(
                volume,
                [start[0], start[2]],
                [treasure[0], treasure[2]],
                true,
            );
        }
        ContentSpiceProfile::MonsterMarch => {
            for x in 0..width {
                volume.set([x, 0, 1], GAME_CONTENT_DUNGEON);
            }
            carve_dungeon_path(
                volume,
                [start[0], start[2]],
                [treasure[0], treasure[2]],
                false,
            );
        }
        ContentSpiceProfile::CrystalDescent => {
            for z in 0..depth {
                for x in 0..width {
                    if x == z || x + z == width.saturating_sub(1) {
                        volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
                    }
                }
            }
            carve_dungeon_path(
                volume,
                [start[0], start[2]],
                [treasure[0], treasure[2]],
                true,
            );
        }
        ContentSpiceProfile::CavernGarden => {
            let ring_min_x = 1;
            let ring_max_x = width.saturating_sub(2);
            let ring_min_z = 1;
            let ring_max_z = depth.saturating_sub(2);
            for x in ring_min_x..=ring_max_x {
                volume.set([x, 0, ring_min_z], GAME_CONTENT_DUNGEON);
                volume.set([x, 0, ring_max_z], GAME_CONTENT_DUNGEON);
            }
            for z in ring_min_z..=ring_max_z {
                volume.set([ring_min_x, 0, z], GAME_CONTENT_DUNGEON);
                volume.set([ring_max_x, 0, z], GAME_CONTENT_DUNGEON);
            }
            carve_dungeon_path(
                volume,
                [start[0], start[2]],
                [treasure[0], treasure[2]],
                false,
            );
        }
    }
}

fn carve_dungeon_path(
    volume: &mut ContentVolume,
    start: [usize; 2],
    end: [usize; 2],
    x_first: bool,
) {
    let [mut x, mut z] = start;
    volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
    if x_first {
        while x != end[0] {
            x = step_toward(x, end[0]);
            volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
        }
        while z != end[1] {
            z = step_toward(z, end[1]);
            volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
        }
    } else {
        while z != end[1] {
            z = step_toward(z, end[1]);
            volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
        }
        while x != end[0] {
            x = step_toward(x, end[0]);
            volume.set([x, 0, z], GAME_CONTENT_DUNGEON);
        }
    }
}

fn protect_treasure_neighbors(volume: &mut ContentVolume, treasure: [usize; 3]) {
    let [width, _, depth] = volume.dimensions;
    let [x, _, z] = treasure;
    for [nx, nz] in [
        [x.saturating_sub(1), z],
        [x + 1, z],
        [x, z.saturating_sub(1)],
        [x, z + 1],
    ] {
        if nx < width && nz < depth && [nx, nz] != [x, z] {
            volume.set([nx, 0, nz], GAME_CONTENT_DUNGEON);
        }
    }
}

fn step_toward(current: usize, target: usize) -> usize {
    if current < target {
        current + 1
    } else {
        current - 1
    }
}

fn spice_hash(seed: u64, x: usize, z: usize) -> u64 {
    mix64(seed ^ (x as u64).wrapping_mul(0x9E37_79B9) ^ (z as u64).wrapping_mul(0xC2B2_AE3D))
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
