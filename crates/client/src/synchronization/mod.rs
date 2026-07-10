//! Authoritative natural-world input shared by online and offline clients.

use bevy::prelude::Resource;
use lk2_core::protocol::components::{
    EcoBerryNet, EcoCloudNet, EcoPlantNet, EcoRabbitNet, EcoWildlifeNet,
};
use lk2_core::simulation::{NatureEvent, NatureSnapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotAcceptance {
    Accepted,
    DuplicateOrStale,
    RejectedNonFinite,
}

/// Monotonic cache at the transport-to-presentation boundary.
///
/// Online replication and offline authority both submit snapshots here. The presentation layer
/// therefore cannot accidentally fork the two simulation paths.
#[derive(Resource, Default, Debug)]
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
        if self.latest.as_ref().is_some_and(|latest| snapshot.tick <= latest.tick) {
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

#[derive(Clone, Copy, Debug)]
pub struct NaturePresentationInput<'a> {
    pub tick: u64,
    pub cloud_water: f32,
    pub available_water: f32,
    pub cumulative_rainfall: f32,
    pub clouds: &'a [EcoCloudNet],
    pub rabbits: &'a [EcoRabbitNet],
    pub wildlife: &'a [EcoWildlifeNet],
    pub berries: &'a [EcoBerryNet],
    pub plants: &'a [EcoPlantNet],
}

impl<'a> From<&'a NatureSnapshot> for NaturePresentationInput<'a> {
    fn from(snapshot: &'a NatureSnapshot) -> Self {
        let ecology = &snapshot.detailed_ecology;
        Self {
            tick: snapshot.tick,
            cloud_water: snapshot.atmosphere.cloud_water,
            available_water: snapshot.hydrology.available_water,
            cumulative_rainfall: snapshot.atmosphere.cumulative_rainfall,
            clouds: &ecology.clouds,
            rabbits: &ecology.rabbits,
            wildlife: &ecology.wildlife,
            berries: &ecology.berries,
            plants: &ecology.plants,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::atmosphere::AtmosphereSnapshot;
    use lk2_core::ecology::nature::NatureEcologySnapshot;
    use lk2_core::hydrology::HydrologySnapshot;
    use lk2_core::protocol::components::EcoSnapshot;

    fn empty_snapshot(tick: u64) -> NatureSnapshot {
        NatureSnapshot {
            tick,
            atmosphere: AtmosphereSnapshot {
                cloud_count: 0,
                cloud_water: 0.0,
                cumulative_rainfall: 0.0,
            },
            hydrology: HydrologySnapshot { available_water: 0.0 },
            ecology: NatureEcologySnapshot { plant_count: 0, plant_units: 0, animal_count: 0 },
            detailed_ecology: EcoSnapshot {
                tick,
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
            },
        }
    }

    #[test]
    fn rejects_duplicate_late_and_non_finite_snapshots() {
        let mut buffer = NatureSnapshotBuffer::default();
        assert_eq!(buffer.push(empty_snapshot(4)), SnapshotAcceptance::Accepted);
        assert_eq!(
            buffer.push(empty_snapshot(4)),
            SnapshotAcceptance::DuplicateOrStale
        );
        assert_eq!(
            buffer.push(empty_snapshot(3)),
            SnapshotAcceptance::DuplicateOrStale
        );
        assert_eq!(
            buffer.push(NatureSnapshot {
                hydrology: HydrologySnapshot { available_water: f32::NAN },
                ..empty_snapshot(5)
            }),
            SnapshotAcceptance::RejectedNonFinite
        );
        assert_eq!(buffer.latest().map(|snapshot| snapshot.tick), Some(4));
        assert_eq!(buffer.accepted_count(), 1);
        assert_eq!(buffer.rejected_count(), 3);
    }
}
