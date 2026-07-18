//! Client-side boundary shared by offline authority and online replication.

use bevy::prelude::Resource;
use lk2_core::atmosphere::AtmosphereSnapshot;
use lk2_core::ecology::nature::NatureEcologySnapshot;
use lk2_core::hydrology::HydrologySnapshot;
use lk2_core::protocol::components::EcoSnapshot;
use lk2_core::simulation::{NatureEvent, NatureSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotAcceptance {
    Accepted,
    DuplicateOrStale,
    RejectedNonFinite,
}

#[derive(Resource, Debug, Default)]
pub struct NatureSnapshotBuffer {
    latest: Option<NatureSnapshot>,
    pending_events: Vec<NatureEvent>,
    accepted_count: u64,
    rejected_count: u64,
}

impl NatureSnapshotBuffer {
    pub fn push(&mut self, snapshot: NatureSnapshot) -> SnapshotAcceptance {
        self.push_with_events(snapshot, std::iter::empty())
    }

    pub fn push_with_events(
        &mut self,
        snapshot: NatureSnapshot,
        events: impl IntoIterator<Item = NatureEvent>,
    ) -> SnapshotAcceptance {
        if !snapshot.is_finite() {
            self.rejected_count = self.rejected_count.saturating_add(1);
            return SnapshotAcceptance::RejectedNonFinite;
        }
        if self
            .latest
            .as_ref()
            .is_some_and(|latest| snapshot.tick <= latest.tick)
        {
            self.rejected_count = self.rejected_count.saturating_add(1);
            return SnapshotAcceptance::DuplicateOrStale;
        }
        self.pending_events.extend(events);
        self.latest = Some(snapshot);
        self.accepted_count = self.accepted_count.saturating_add(1);
        SnapshotAcceptance::Accepted
    }

    #[must_use]
    pub fn latest(&self) -> Option<&NatureSnapshot> {
        self.latest.as_ref()
    }

    #[must_use]
    pub fn accepted_count(&self) -> u64 {
        self.accepted_count
    }

    #[must_use]
    pub fn rejected_count(&self) -> u64 {
        self.rejected_count
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = NatureEvent> + '_ {
        self.pending_events.drain(..)
    }
}

#[must_use]
pub fn nature_snapshot_from_protocol(snapshot: &EcoSnapshot) -> NatureSnapshot {
    let atmosphere = AtmosphereSnapshot {
        cloud_count: snapshot.clouds.len() as u32,
        cloud_water: snapshot
            .clouds
            .iter()
            .map(|cloud| cloud.rain.max(0.0))
            .sum(),
        cumulative_rainfall: snapshot.rainfall.max(0.0),
    };
    let hydrology = HydrologySnapshot {
        available_water: snapshot.rain.max(0.0),
    };
    let ecology = NatureEcologySnapshot {
        plant_count: (snapshot.plants.len() + snapshot.berries.len()) as u32,
        plant_units: snapshot
            .plants
            .iter()
            .map(|plant| u64::from(plant.stock))
            .chain(snapshot.berries.iter().map(|berry| u64::from(berry.fruit)))
            .sum(),
        animal_count: (snapshot.rabbits.len() + snapshot.wildlife.len()) as u32,
    };
    NatureSnapshot {
        tick: snapshot.tick,
        atmosphere,
        hydrology,
        ecology,
        detailed_ecology: snapshot.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_rejects_duplicate_snapshots() {
        let mut buffer = NatureSnapshotBuffer::default();
        let detail = EcoSnapshot {
            tick: 1,
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: vec![],
            rabbits: vec![],
            wildlife: vec![],
            berries: vec![],
            plants: vec![],
        };
        let snapshot = nature_snapshot_from_protocol(&detail);
        assert_eq!(buffer.push(snapshot.clone()), SnapshotAcceptance::Accepted);
        assert_eq!(buffer.push(snapshot), SnapshotAcceptance::DuplicateOrStale);
        assert_eq!(buffer.accepted_count(), 1);
        assert_eq!(buffer.rejected_count(), 1);
    }

    #[test]
    fn buffer_drains_pending_events_once() {
        let mut buffer = NatureSnapshotBuffer::default();
        let detail = EcoSnapshot {
            tick: 1,
            co2: 0.0,
            rain: 0.0,
            rainfall: 1.25,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 2,
            rabbits_born: 0,
            wildlife_born: 0,
            clouds: vec![],
            rabbits: vec![],
            wildlife: vec![],
            berries: vec![],
            plants: vec![],
        };
        let snapshot = nature_snapshot_from_protocol(&detail);

        assert_eq!(
            buffer.push_with_events(
                snapshot,
                [
                    NatureEvent::RainFell {
                        tick: 1,
                        amount: 1.25,
                    },
                    NatureEvent::PlantsGrown { tick: 1, count: 2 },
                ],
            ),
            SnapshotAcceptance::Accepted
        );

        let drained = buffer.drain_events().collect::<Vec<_>>();
        assert_eq!(
            drained,
            vec![
                NatureEvent::RainFell {
                    tick: 1,
                    amount: 1.25,
                },
                NatureEvent::PlantsGrown { tick: 1, count: 2 },
            ]
        );
        assert!(buffer.drain_events().next().is_none());
    }
}
