use bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    ReceiveNet,

    ReadInput,

    BuildIntent,

    ValidateRules,

    Movement,

    Interaction,

    Combat,

    ResourceAndLoot,

    EquipmentAndStats,

    NationAndArtifact,

    ScoreAndAudit,

    Snapshot,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresentationSet {
    CameraFollow,

    Animation,

    Vfx,

    Ui,

    TransformSync,
}

#[macro_export]
macro_rules! sim_set_tuple {
    () => {
        (
            $crate::simulation::app_sets::SimSet::ReceiveNet,
            $crate::simulation::app_sets::SimSet::ReadInput,
            $crate::simulation::app_sets::SimSet::BuildIntent,
            $crate::simulation::app_sets::SimSet::ValidateRules,
            $crate::simulation::app_sets::SimSet::Movement,
            $crate::simulation::app_sets::SimSet::Interaction,
            $crate::simulation::app_sets::SimSet::Combat,
            $crate::simulation::app_sets::SimSet::ResourceAndLoot,
            $crate::simulation::app_sets::SimSet::EquipmentAndStats,
            $crate::simulation::app_sets::SimSet::NationAndArtifact,
            $crate::simulation::app_sets::SimSet::ScoreAndAudit,
            $crate::simulation::app_sets::SimSet::Snapshot,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simset_labels_unique() {
        use std::collections::HashSet;
        let labels: HashSet<&'static str> = [
            "ReceiveNet",
            "ReadInput",
            "BuildIntent",
            "ValidateRules",
            "Movement",
            "Interaction",
            "Combat",
            "ResourceAndLoot",
            "EquipmentAndStats",
            "NationAndArtifact",
            "ScoreAndAudit",
            "Snapshot",
        ]
        .iter()
        .copied()
        .collect();
        assert_eq!(labels.len(), 12);
    }
}
