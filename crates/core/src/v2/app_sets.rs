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
            $crate::v2::app_sets::SimSet::ReceiveNet,
            $crate::v2::app_sets::SimSet::ReadInput,
            $crate::v2::app_sets::SimSet::BuildIntent,
            $crate::v2::app_sets::SimSet::ValidateRules,
            $crate::v2::app_sets::SimSet::Movement,
            $crate::v2::app_sets::SimSet::Interaction,
            $crate::v2::app_sets::SimSet::Combat,
            $crate::v2::app_sets::SimSet::ResourceAndLoot,
            $crate::v2::app_sets::SimSet::EquipmentAndStats,
            $crate::v2::app_sets::SimSet::NationAndArtifact,
            $crate::v2::app_sets::SimSet::ScoreAndAudit,
            $crate::v2::app_sets::SimSet::Snapshot,
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
