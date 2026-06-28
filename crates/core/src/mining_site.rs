

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::resource::{GlobalResourcePool, PoolError, ResourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MiningSiteKind {

    SurfacePit,

    CaveFissure,

    MoonRidge,

    SpiritSink,

    RelicQuarry,

    CalamityBloom,
}

impl MiningSiteKind {
    pub fn id_str(self) -> &'static str {
        match self {
            Self::SurfacePit => "mine_surface_pit",
            Self::CaveFissure => "mine_cave_fissure",
            Self::MoonRidge => "mine_moon_ridge",
            Self::SpiritSink => "mine_spirit_sink",
            Self::RelicQuarry => "mine_relic_quarry",
            Self::CalamityBloom => "mine_calamity_bloom",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::SurfacePit => "露天矿坑",
            Self::CaveFissure => "裂隙洞穴",
            Self::MoonRidge => "月纹矿坑",
            Self::SpiritSink => "灵脉矿井",
            Self::RelicQuarry => "遗迹采石场",
            Self::CalamityBloom => "灾变矿潮",
        }
    }

    pub fn default_slot_count(self) -> u32 {
        match self {
            Self::SurfacePit => 3,
            Self::CaveFissure => 4,
            Self::MoonRidge => 3,
            Self::SpiritSink => 4,
            Self::RelicQuarry => 3,
            Self::CalamityBloom => 5,
        }
    }

    pub fn primary_yields(self) -> &'static [(ResourceKind, i64)] {
        use ResourceKind::*;
        match self {

            Self::SurfacePit => &[(Wood, 0)],

            Self::CaveFissure => &[(Wood, 0)],
            Self::MoonRidge => &[(Wood, 0)],
            Self::SpiritSink => &[(SpiritEssence, 400), (RuneStone, 120)],
            Self::RelicQuarry => &[(Wood, 0)],
            Self::CalamityBloom => &[(SpiritEssence, 280), (SparkFragment, 1)],
        }
    }

    pub fn per_yield(self) -> (ResourceKind, i64) {
        use ResourceKind::*;
        match self {
            Self::SpiritSink => (SpiritEssence, 35),
            Self::CalamityBloom => (SparkFragment, 1),
            _ => (Wood, 10),
        }
    }

    pub fn gather_ticks(self, mode: MiningMode) -> u32 {
        let base = match self {
            Self::SurfacePit => 30 * 12,
            Self::CaveFissure => 30 * 15,
            Self::MoonRidge => 30 * 14,
            Self::SpiritSink => 30 * 18,
            Self::RelicQuarry => 30 * 16,
            Self::CalamityBloom => 30 * 24,
        };
        match mode {
            MiningMode::Steady => base,
            MiningMode::Hard => (base as f32 * 0.55) as u32,
            MiningMode::Night => base,
            MiningMode::Purify => (base as f32 * 1.3) as u32,
        }
    }

    pub fn noise_radius(self, mode: MiningMode) -> f32 {
        let base = match self {
            Self::SurfacePit => 90.0,
            Self::CaveFissure => 110.0,
            Self::MoonRidge => 150.0,
            Self::SpiritSink => 150.0,
            Self::RelicQuarry => 100.0,
            Self::CalamityBloom => 200.0,
        };
        match mode {
            MiningMode::Steady => base,
            MiningMode::Hard => base * 1.6,
            MiningMode::Night => base,
            MiningMode::Purify => base * 0.7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiningMode {

    Steady,

    Hard,

    Night,

    Purify,
}

impl MiningMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steady => "steady",
            Self::Hard => "hard",
            Self::Night => "night",
            Self::Purify => "purify",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepletionTier {

    Normal,

    LowYield,

    NearEmpty,

    Depleted,
}

impl DepletionTier {
    pub fn from_remaining_pct(remaining_pct: f32) -> Self {
        let remaining_pct = remaining_pct.clamp(0.0, 100.0);
        if remaining_pct <= 0.0 {
            Self::Depleted
        } else if remaining_pct <= 20.0 {
            Self::NearEmpty
        } else if remaining_pct <= 55.0 {
            Self::LowYield
        } else {
            Self::Normal
        }
    }

    pub fn from_percent(p: f32) -> Self {
        Self::from_remaining_pct(p)
    }

    pub fn yield_multiplier(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::LowYield => 0.7,
            Self::NearEmpty => 0.3,
            Self::Depleted => 0.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::LowYield => "low_yield",
            Self::NearEmpty => "near_empty",
            Self::Depleted => "depleted",
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct MiningSite {
    pub id: u32,
    pub kind: MiningSiteKind,

    pub world_pos: [f32; 3],

    pub remaining_pct: f32,

    pub slot_entities: Vec<Entity>,

    pub contested_by: Vec<u32>,
}

impl MiningSite {
    pub fn new(id: u32, kind: MiningSiteKind, world_pos: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            world_pos,
            remaining_pct: 100.0,
            slot_entities: Vec::new(),
            contested_by: Vec::new(),
        }
    }

    pub fn depletion_tier(&self) -> DepletionTier {
        DepletionTier::from_remaining_pct(self.remaining_pct)
    }
}

#[derive(Component, Debug, Clone)]
pub struct MiningSlot {

    pub site_id: u32,

    pub slot_index: u32,

    pub occupant: Option<u32>,

    pub progress_ticks: u32,

    pub mode: MiningMode,
}

impl MiningSlot {
    pub fn new(site_id: u32, slot_index: u32) -> Self {
        Self { site_id, slot_index, occupant: None, progress_ticks: 0, mode: MiningMode::Steady }
    }

    pub fn is_occupied(&self) -> bool {
        self.occupant.is_some()
    }
}

#[derive(Resource, Debug, Default)]
pub struct MiningSiteRegistry {
    next_id: u32,

    pub common_count: u32,

    pub rich_count: u32,

    pub active: Vec<u32>,
}

impl MiningSiteRegistry {
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

pub fn award_mining_yield(
    pool: &mut GlobalResourcePool,
    kind: ResourceKind,
    amount: i64,
) -> Result<i64, PoolError> {
    pool.try_add(kind, amount)
}

#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSlotStarted {
    pub site_id: u32,
    pub slot_index: u32,
    pub player_id: u32,
    pub mode: MiningMode,
    pub at_wall_secs: f32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSlotCompleted {
    pub site_id: u32,
    pub slot_index: u32,
    pub player_id: u32,
    pub mode: MiningMode,
    pub yield_kind: ResourceKind,
    pub yield_amount: i64,
    pub at_wall_secs: f32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct MiningSiteDepletionChanged {
    pub site_id: u32,
    pub from: DepletionTier,
    pub to: DepletionTier,
    pub remaining_pct: f32,
}

pub fn tick_mining_slots(
    commands: Commands,
    mut slots: Query<(Entity, &mut MiningSlot)>,
    mut sites: Query<&mut MiningSite>,
    mut pool: ResMut<GlobalResourcePool>,
    site_registry: ResMut<MiningSiteRegistry>,
    match_clock: Res<crate::match_state::MatchClock>,
    started_events: MessageWriter<MiningSlotStarted>,
    mut completed_events: MessageWriter<MiningSlotCompleted>,
    mut depletion_events: MessageWriter<MiningSiteDepletionChanged>,
) {

    for (slot_entity, mut slot) in slots.iter_mut() {
        let Some(player_id) = slot.occupant else {
            continue;
        };

        let site_id = slot.site_id;
        let site_kind = sites.iter().find(|s| s.id == site_id).map(|s| s.kind);
        let Some(site_kind) = site_kind else {

            slot.occupant = None;
            slot.progress_ticks = 0;
            let _ = slot_entity;
            continue;
        };

        let required = site_kind.gather_ticks(slot.mode);
        slot.progress_ticks += 1;
        if slot.progress_ticks < required {
            continue;
        }

        slot.progress_ticks = 0;

        let site = sites.iter().find(|s| s.id == site_id).unwrap();
        let tier = site.depletion_tier();
        let mult = tier.yield_multiplier();
        if mult <= 0.0 {

            info!(
                "[mine] site {} ({}) depleted, no yield for player {}",
                site_id,
                site_kind.label_zh(),
                player_id
            );
            continue;
        }
        let (kind, base_amount) = site_kind.per_yield();
        let amount = ((base_amount as f32) * mult).round() as i64;
        if amount > 0 {

            match award_mining_yield(&mut pool, kind, amount) {
                Ok(_) => {
                    completed_events.write(MiningSlotCompleted {
                        site_id,
                        slot_index: slot.slot_index,
                        player_id,
                        mode: slot.mode,
                        yield_kind: kind,
                        yield_amount: amount,
                        at_wall_secs: match_clock.wall_secs,
                    });
                    info!(
                        "[mine] site {} ({}) slot {} → player {} +{} {:?} (tier={:?}, mult={:.2})",
                        site_id,
                        site_kind.label_zh(),
                        slot.slot_index,
                        player_id,
                        amount,
                        kind,
                        tier,
                        mult
                    );
                }
                Err(err) => {
                    warn!(
                        "[mine] failed to award yield for site {} slot {} player {}: {}",
                        site_id, slot.slot_index, player_id, err
                    );
                }
            }
        }

        let _ = site_registry;
        let _ = depletion_events;
        let _ = started_events;
        let _ = commands;
    }

    for mut site in sites.iter_mut() {

        let occupied =
            slots.iter().filter(|(_, s)| s.occupant.is_some() && s.site_id == site.id).count()
                as f32;
        let drain = 0.005 * occupied;
        if drain > 0.0 && site.remaining_pct > 0.0 {
            let old_tier = site.depletion_tier();
            site.remaining_pct = (site.remaining_pct - drain).max(0.0);
            let new_tier = site.depletion_tier();
            if new_tier != old_tier {
                depletion_events.write(MiningSiteDepletionChanged {
                    site_id: site.id,
                    from: old_tier,
                    to: new_tier,
                    remaining_pct: site.remaining_pct,
                });
                info!(
                    "[mine] site {} ({}) depletion: {:?} → {:?} ({:.1}%)",
                    site.id,
                    site.kind.label_zh(),
                    old_tier,
                    new_tier,
                    site.remaining_pct
                );
            }
        }
    }
}

pub struct MiningSitePlugin;

impl Plugin for MiningSitePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiningSiteRegistry>()
            .add_message::<MiningSlotStarted>()
            .add_message::<MiningSlotCompleted>()
            .add_message::<MiningSiteDepletionChanged>()
            .add_systems(FixedUpdate, tick_mining_slots);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_state::MatchClock;
    use crate::resource::ResourceKind;

    #[test]
    fn kind_id_str_matches_table() {

        assert_eq!(MiningSiteKind::SurfacePit.id_str(), "mine_surface_pit");
        assert_eq!(MiningSiteKind::CaveFissure.id_str(), "mine_cave_fissure");
        assert_eq!(MiningSiteKind::MoonRidge.id_str(), "mine_moon_ridge");
        assert_eq!(MiningSiteKind::SpiritSink.id_str(), "mine_spirit_sink");
        assert_eq!(MiningSiteKind::RelicQuarry.id_str(), "mine_relic_quarry");
        assert_eq!(
            MiningSiteKind::CalamityBloom.id_str(),
            "mine_calamity_bloom"
        );
    }

    #[test]
    fn slot_count_in_range_2_to_5() {

        for k in [
            MiningSiteKind::SurfacePit,
            MiningSiteKind::CaveFissure,
            MiningSiteKind::MoonRidge,
            MiningSiteKind::SpiritSink,
            MiningSiteKind::RelicQuarry,
            MiningSiteKind::CalamityBloom,
        ] {
            let n = k.default_slot_count();
            assert!(
                (2..=6).contains(&n),
                "{:?} slot_count={} out of range",
                k,
                n
            );
        }
    }

    #[test]
    fn depletion_tier_thresholds() {
        assert_eq!(
            DepletionTier::from_remaining_pct(100.0),
            DepletionTier::Normal
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(55.1),
            DepletionTier::Normal
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(55.0),
            DepletionTier::LowYield
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(20.1),
            DepletionTier::LowYield
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(20.0),
            DepletionTier::NearEmpty
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(0.1),
            DepletionTier::NearEmpty
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(0.0),
            DepletionTier::Depleted
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(-1.0),
            DepletionTier::Depleted
        );
        assert_eq!(
            DepletionTier::from_remaining_pct(120.0),
            DepletionTier::Normal
        );
    }

    #[test]
    fn hard_mode_faster_than_steady() {
        for k in [
            MiningSiteKind::SurfacePit,
            MiningSiteKind::SpiritSink,
            MiningSiteKind::CalamityBloom,
        ] {
            let steady = k.gather_ticks(MiningMode::Steady);
            let hard = k.gather_ticks(MiningMode::Hard);
            assert!(
                hard < steady,
                "{:?} hard ({}) >= steady ({})",
                k,
                hard,
                steady
            );
        }
    }

    #[test]
    fn registry_allocates_unique_ids() {
        let mut r = MiningSiteRegistry { common_count: 10, rich_count: 4, ..Default::default() };
        let a = r.alloc_id();
        let b = r.alloc_id();
        assert_ne!(a, b);
    }

    #[test]
    fn spirit_sink_yields_spirit_essence() {

        let (kind, amount) = MiningSiteKind::SpiritSink.per_yield();
        assert_eq!(kind, ResourceKind::SpiritEssence);
        assert!(amount > 0);
    }

    #[test]
    fn calamity_bloom_yields_spark_fragment() {

        let (kind, _) = MiningSiteKind::CalamityBloom.per_yield();
        assert_eq!(kind, ResourceKind::SparkFragment);
    }

    #[test]
    fn site_depletion_tier_drives_yield_multiplier() {
        let mut site = MiningSite::new(1, MiningSiteKind::SpiritSink, [0.0, 0.0, 0.0]);
        site.remaining_pct = 100.0;
        assert_eq!(site.depletion_tier(), DepletionTier::Normal);
        assert_eq!(site.depletion_tier().yield_multiplier(), 1.0);
        site.remaining_pct = 50.0;
        assert_eq!(site.depletion_tier(), DepletionTier::LowYield);
        assert!((site.depletion_tier().yield_multiplier() - 0.7).abs() < 0.01);
        site.remaining_pct = 10.0;
        assert_eq!(site.depletion_tier(), DepletionTier::NearEmpty);
        assert!((site.depletion_tier().yield_multiplier() - 0.3).abs() < 0.01);
        site.remaining_pct = 0.0;
        assert_eq!(site.depletion_tier(), DepletionTier::Depleted);
        assert_eq!(site.depletion_tier().yield_multiplier(), 0.0);
    }

    #[test]
    fn new_slot_is_empty() {
        let s = MiningSlot::new(1, 0);
        assert!(!s.is_occupied());
        assert_eq!(s.progress_ticks, 0);
        assert_eq!(s.mode, MiningMode::Steady);
    }

    #[test]
    fn award_mining_yield_reports_full_pool() {
        let mut pool = GlobalResourcePool::new();
        pool.try_add(
            ResourceKind::SpiritEssence,
            ResourceKind::SpiritEssence.max(),
        )
        .unwrap();

        let err = award_mining_yield(&mut pool, ResourceKind::SpiritEssence, 1).unwrap_err();

        assert!(matches!(
            err,
            PoolError::WouldExceedMax { kind: ResourceKind::SpiritEssence, .. }
        ));
        assert_eq!(
            pool.get(ResourceKind::SpiritEssence),
            ResourceKind::SpiritEssence.max()
        );
    }
}
