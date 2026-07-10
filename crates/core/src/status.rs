use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum StatusKind {
    Strength,
    Speed,
    Regeneration,
    FireResistance,
    Slowness,
    Poison,
}

impl StatusKind {
    pub const fn label_zh(self) -> &'static str {
        match self {
            Self::Strength => "力量",
            Self::Speed => "迅捷",
            Self::Regeneration => "再生",
            Self::FireResistance => "火焰抗性",
            Self::Slowness => "迟缓",
            Self::Poison => "中毒",
        }
    }

    pub const fn is_beneficial(self) -> bool {
        matches!(
            self,
            Self::Strength | Self::Speed | Self::Regeneration | Self::FireResistance
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StatusEffect {
    pub kind: StatusKind,
    pub potency: u8,
    pub remaining_secs: f32,
}

impl StatusEffect {
    pub fn new(kind: StatusKind, potency: u8, remaining_secs: f32) -> Self {
        Self { kind, potency: potency.max(1), remaining_secs: remaining_secs.max(0.0) }
    }
}

#[derive(Component, Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StatusSet {
    effects: Vec<StatusEffect>,
}

impl StatusSet {
    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &StatusEffect> {
        self.effects.iter()
    }

    pub fn get(&self, kind: StatusKind) -> Option<&StatusEffect> {
        self.effects.iter().find(|effect| effect.kind == kind)
    }

    pub fn apply(&mut self, incoming: StatusEffect) {
        if incoming.remaining_secs <= 0.0 {
            return;
        }
        if let Some(existing) = self.effects.iter_mut().find(|effect| effect.kind == incoming.kind)
        {
            existing.potency = existing.potency.max(incoming.potency);
            existing.remaining_secs = existing.remaining_secs.max(incoming.remaining_secs);
            return;
        }
        self.effects.push(incoming);
        self.effects.sort_by_key(|effect| effect.kind);
    }

    pub fn copy_from(&mut self, source: &StatusSet) -> Vec<StatusKind> {
        let copied = source.effects.iter().map(|effect| effect.kind).collect::<Vec<_>>();
        for effect in &source.effects {
            self.apply(effect.clone());
        }
        copied
    }

    pub fn tick(&mut self, delta_secs: f32) {
        let delta_secs = delta_secs.max(0.0);
        for effect in &mut self.effects {
            effect.remaining_secs = (effect.remaining_secs - delta_secs).max(0.0);
        }
        self.effects.retain(|effect| effect.remaining_secs > 0.0);
    }

    pub fn summary_zh(&self) -> String {
        if self.effects.is_empty() {
            return "无".to_string();
        }
        self.effects
            .iter()
            .map(|effect| {
                format!(
                    "{}{} {:.0}s",
                    effect.kind.label_zh(),
                    effect.potency,
                    effect.remaining_secs
                )
            })
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

pub fn tick_status_sets(time: Res<Time>, mut statuses: Query<&mut StatusSet>) {
    for mut status_set in &mut statuses {
        status_set.tick(time.delta_secs());
    }
}

pub struct StatusPlugin;

impl Plugin for StatusPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<StatusKind>()
            .register_type::<StatusEffect>()
            .register_type::<StatusSet>()
            .add_systems(Update, tick_status_sets);
    }
}
