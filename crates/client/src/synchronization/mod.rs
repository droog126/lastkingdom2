//! Authoritative natural-world input shared by online and offline clients.

use bevy::prelude::Resource;
use lk2_core::protocol::components::{
    EcoBerryNet, EcoCloudNet, EcoPlantNet, EcoRabbitNet, EcoSnapshot, EcoWildlifeNet,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotAcceptance {
    Accepted,
    DuplicateOrStale,
    RejectedNonFinite,
}

/// Monotonic cache at the transport-to-presentation boundary.
///
/// Online replication and offline authority both submit snapshots here. The presentation layer
/// therefore has no way to distinguish or accidentally fork the two simulation paths.
#[derive(Resource, Default, Debug)]
pub struct NatureSnapshotBuffer {
    latest: Option<EcoSnapshot>,
    accepted_count: u64,
    rejected_count: u64,
}

impl NatureSnapshotBuffer {
    pub fn push(&mut self, snapshot: EcoSnapshot) -> SnapshotAcceptance {
        if !snapshot_is_finite(&snapshot) {
            self.rejected_count = self.rejected_count.saturating_add(1);
            return SnapshotAcceptance::RejectedNonFinite;
        }
        if self.latest.as_ref().is_some_and(|latest| snapshot.tick <= latest.tick) {
            self.rejected_count = self.rejected_count.saturating_add(1);
            return SnapshotAcceptance::DuplicateOrStale;
        }
        self.latest = Some(snapshot);
        self.accepted_count = self.accepted_count.saturating_add(1);
        SnapshotAcceptance::Accepted
    }

    #[must_use]
    pub fn latest(&self) -> Option<&EcoSnapshot> {
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
}

#[derive(Clone, Copy, Debug)]
pub struct NaturePresentationInput<'a> {
    pub tick: u64,
    pub rain: f32,
    pub rainfall: f32,
    pub clouds: &'a [EcoCloudNet],
    pub rabbits: &'a [EcoRabbitNet],
    pub wildlife: &'a [EcoWildlifeNet],
    pub berries: &'a [EcoBerryNet],
    pub plants: &'a [EcoPlantNet],
}

impl<'a> From<&'a EcoSnapshot> for NaturePresentationInput<'a> {
    fn from(snapshot: &'a EcoSnapshot) -> Self {
        Self {
            tick: snapshot.tick,
            rain: snapshot.rain,
            rainfall: snapshot.rainfall,
            clouds: &snapshot.clouds,
            rabbits: &snapshot.rabbits,
            wildlife: &snapshot.wildlife,
            berries: &snapshot.berries,
            plants: &snapshot.plants,
        }
    }
}

fn snapshot_is_finite(snapshot: &EcoSnapshot) -> bool {
    snapshot.co2.is_finite()
        && snapshot.rain.is_finite()
        && snapshot.rainfall.is_finite()
        && snapshot.clouds.iter().all(|cloud| {
            finite_position(cloud.x, cloud.z) && cloud.rain.is_finite() && cloud.phase.is_finite()
        })
        && snapshot
            .rabbits
            .iter()
            .all(|rabbit| finite_position(rabbit.x, rabbit.z) && rabbit.energy.is_finite())
        && snapshot
            .wildlife
            .iter()
            .all(|animal| finite_position(animal.x, animal.z) && animal.energy.is_finite())
        && snapshot.berries.iter().all(|berry| finite_position(berry.x, berry.z))
        && snapshot.plants.iter().all(|plant| finite_position(plant.x, plant.z))
}

fn finite_position(x: f32, z: f32) -> bool {
    x.is_finite() && z.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_snapshot(tick: u64) -> EcoSnapshot {
        EcoSnapshot {
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
            buffer.push(EcoSnapshot { rain: f32::NAN, ..empty_snapshot(5) }),
            SnapshotAcceptance::RejectedNonFinite
        );
        assert_eq!(buffer.latest().map(|snapshot| snapshot.tick), Some(4));
        assert_eq!(buffer.accepted_count(), 1);
        assert_eq!(buffer.rejected_count(), 3);
    }
}
