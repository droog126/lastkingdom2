//! Authoritative natural-world input for the client presentation layer.

use lk2_core::protocol::components::{EcoSnapshot, EcoCloudNet, EcoPlantNet, EcoWildlifeNet};

#[derive(Clone, Debug, PartialEq)]
pub enum NatureEvent {
    Snapshot(EcoSnapshot),
}

#[derive(Default, Debug)]
pub struct NatureSnapshotBuffer {
    latest: Option<EcoSnapshot>,
    last_tick: Option<u64>,
}

impl NatureSnapshotBuffer {
    /// Accept snapshots monotonically; duplicates and late packets are ignored.
    pub fn push(&mut self, snapshot: EcoSnapshot) -> bool {
        if self.last_tick.is_some_and(|tick| snapshot.tick <= tick) {
            return false;
        }
        self.last_tick = Some(snapshot.tick);
        self.latest = Some(snapshot);
        true
    }

    #[must_use]
    pub fn latest(&self) -> Option<&EcoSnapshot> { self.latest.as_ref() }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NaturePresentationInput<'a> {
    pub tick: u64,
    pub clouds: &'a [EcoCloudNet],
    pub plants: &'a [EcoPlantNet],
    pub animals: &'a [EcoWildlifeNet],
}

impl<'a> From<&'a EcoSnapshot> for NaturePresentationInput<'a> {
    fn from(snapshot: &'a EcoSnapshot) -> Self {
        Self { tick: snapshot.tick, clouds: &snapshot.clouds, plants: &snapshot.plants, animals: &snapshot.wildlife }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_rejects_duplicate_and_late_snapshots() {
        let snapshot = EcoSnapshot { tick: 4, co2: 0.0, rain: 0.0, rainfall: 0.0, fruit_eaten: 0, fruit_grown: 0, plants_grown: 0, rabbits_born: 0, wildlife_born: 0, clouds: vec![], rabbits: vec![], wildlife: vec![], berries: vec![], plants: vec![] };
        let mut buffer = NatureSnapshotBuffer::default();
        assert!(buffer.push(snapshot.clone()));
        assert!(!buffer.push(snapshot.clone()));
        assert!(!buffer.push(EcoSnapshot { tick: 3, ..snapshot }));
    }
}
