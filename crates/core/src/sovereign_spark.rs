use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::nation::{NationId, NationRegistry};
use crate::player::PlayerTag;
use crate::resource::{GlobalResourcePool, ResourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparkStatus {
    Dropped,

    Carried,

    Consumed,

    Sealed,
}

impl SparkStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dropped => "dropped",
            Self::Carried => "carried",
            Self::Consumed => "consumed",
            Self::Sealed => "sealed",
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Spark {
    pub id: u32,
    pub status: SparkStatus,

    pub holder: Option<u32>,

    pub world_pos: [f32; 3],

    pub revealed_at_secs: f32,

    pub broadcast: bool,

    pub recirculated: bool,
}

impl Spark {
    pub fn new(id: u32, world_pos: [f32; 3], revealed_at_secs: f32) -> Self {
        Self {
            id,
            status: SparkStatus::Dropped,
            holder: None,
            world_pos,
            revealed_at_secs,
            broadcast: true,
            recirculated: false,
        }
    }

    pub fn is_pickupable(&self) -> bool {
        matches!(self.status, SparkStatus::Dropped)
    }
}

#[derive(Resource, Debug)]
pub struct SparkRegistry {
    next_id: u32,

    pub max_sparks: u32,

    pub active: Vec<u32>,
}

impl Default for SparkRegistry {
    fn default() -> Self {
        Self {
            next_id: 1,
            max_sparks: 3,
            active: Vec::new(),
        }
    }
}

impl SparkRegistry {
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SparkDropped {
    pub spark_id: u32,
    pub world_pos: [f32; 3],
    pub prev_holder: Option<u32>,
    pub at_wall_secs: f32,
}

#[derive(Message, Debug, Clone, Copy)]
pub struct SparkPickedUp {
    pub spark_id: u32,
    pub holder: u32,
    pub at_wall_secs: f32,
}

#[derive(Message, Debug, Clone)]
pub struct SparkConsumedForFounding {
    pub spark_id: u32,
    pub founder: u32,
    pub nation_id: Option<NationId>,
    pub at_wall_secs: f32,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FounderIntent;

pub fn drop_spark_on_player_death(
    mut sparks: Query<(Entity, &mut Spark)>,
    dead_players: Query<&PlayerTag, With<DeadMarker>>,
    mut events: MessageWriter<SparkDropped>,
    match_clock: Res<crate::match_state::MatchClock>,
) {
    for (_entity, mut spark) in sparks.iter_mut() {
        if spark.status != SparkStatus::Carried {
            continue;
        }
        let Some(holder) = spark.holder else { continue };
        let is_dead = dead_players.iter().any(|p| p.id() == holder);
        if !is_dead {
            continue;
        }

        spark.status = SparkStatus::Dropped;
        spark.holder = None;
        events.write(SparkDropped {
            spark_id: spark.id,
            world_pos: spark.world_pos,
            prev_holder: Some(holder),
            at_wall_secs: match_clock.wall_secs,
        });
        info!(
            "[spark] id={} dropped from dead player {} at {:?}",
            spark.id, holder, spark.world_pos
        );
    }
}

pub fn consume_spark_for_founding(
    mut sparks: Query<(Entity, &mut Spark)>,
    players: Query<&PlayerTag, With<FounderIntent>>,
    mut registry: ResMut<NationRegistry>,
    mut pool: ResMut<GlobalResourcePool>,
    mut spark_registry: ResMut<SparkRegistry>,
    mut events: MessageWriter<SparkConsumedForFounding>,
    match_clock: Res<crate::match_state::MatchClock>,
) {
    for (_entity, mut spark) in sparks.iter_mut() {
        if spark.status != SparkStatus::Carried {
            continue;
        }
        let Some(holder) = spark.holder else { continue };
        let is_founding = players.iter().any(|p| p.id() == holder);
        if !is_founding {
            continue;
        }

        spark.status = SparkStatus::Consumed;
        spark.holder = None;

        let nation_name = format!("Nation-{}", holder);
        let tick = match_clock.wall_secs as u64;
        let result = registry.found(&mut pool, holder, nation_name.clone(), [0, 0, 0], tick);
        let nation_id = match result {
            Ok(id) => Some(id),
            Err(e) => {
                warn!(
                    "[spark] founding failed for player {}: {:?}; spark remains on ground",
                    holder, e
                );

                spark.status = SparkStatus::Carried;
                spark.holder = Some(holder);
                continue;
            }
        };
        spark_registry.active.retain(|id| *id != spark.id);
        events.write(SparkConsumedForFounding {
            spark_id: spark.id,
            founder: holder,
            nation_id,
            at_wall_secs: match_clock.wall_secs,
        });
        info!(
            "[spark] id={} consumed → nation={:?} ({}) by player {}",
            spark.id, nation_id, nation_name, holder
        );
    }
}

pub fn recirculate_spark_on_nation_end(
    mut sparks: Query<&mut Spark>,
    mut pool: ResMut<GlobalResourcePool>,
) {
    for mut spark in sparks.iter_mut() {
        if spark.status == SparkStatus::Consumed && !spark.recirculated {
            let _ = pool.try_add(ResourceKind::Soul, 50);
            spark.recirculated = true;
            info!("[spark] id={} recirculated to +50 Soul (V2 §5.4)", spark.id);
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DeadMarker;

pub struct SovereignSparkPlugin;

impl Plugin for SovereignSparkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SparkRegistry>()
            .add_message::<SparkDropped>()
            .add_message::<SparkPickedUp>()
            .add_message::<SparkConsumedForFounding>()
            .add_systems(
                FixedUpdate,
                (
                    drop_spark_on_player_death,
                    consume_spark_for_founding,
                    recirculate_spark_on_nation_end,
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_state::MatchClock;
    use crate::nation::NationRegistry;

    #[test]
    fn spark_creation_and_status() {
        let s = Spark::new(1, [10.0, 0.0, 10.0], 0.0);
        assert!(s.is_pickupable());
        assert_eq!(s.status, SparkStatus::Dropped);
    }

    #[test]
    fn spark_pickupable_to_carried() {
        let mut s = Spark::new(1, [10.0, 0.0, 10.0], 0.0);
        s.holder = Some(42);
        s.status = SparkStatus::Carried;
        assert!(!s.is_pickupable());
    }

    #[test]
    fn registry_allocates_unique_ids() {
        let mut r = SparkRegistry::default();
        let a = r.alloc_id();
        let b = r.alloc_id();
        assert_ne!(a, b);
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[test]
    fn consumed_spark_recirculates_to_pool() {
        let _s = Spark {
            status: SparkStatus::Consumed,
            ..Spark::new(1, [0.0; 3], 0.0)
        };

        let mut pool = GlobalResourcePool::default();
        let _ = pool.try_add(ResourceKind::Soul, 50);
        assert_eq!(pool.get(ResourceKind::Soul), 50);
    }

    #[test]
    fn carried_spark_without_founder_intent_is_not_consumed() {
        let mut app = App::new();
        app.insert_resource(NationRegistry::default())
            .insert_resource(GlobalResourcePool::default())
            .insert_resource(SparkRegistry::default())
            .insert_resource(MatchClock::default())
            .add_message::<SparkConsumedForFounding>()
            .add_systems(Update, consume_spark_for_founding);

        app.world_mut().spawn(PlayerTag(7));
        let spark_entity = app
            .world_mut()
            .spawn(Spark {
                id: 1,
                status: SparkStatus::Carried,
                holder: Some(7),
                world_pos: [0.0; 3],
                revealed_at_secs: 0.0,
                broadcast: false,
                recirculated: false,
            })
            .id();

        app.update();

        let spark = app.world().get::<Spark>(spark_entity).unwrap();
        assert_eq!(spark.status, SparkStatus::Carried);
        assert_eq!(spark.holder, Some(7));
        assert_eq!(app.world().resource::<NationRegistry>().flag_count, 0);
    }
}
