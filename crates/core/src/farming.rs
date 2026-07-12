//! Deterministic crop rules shared by offline and online authority.

use crate::resource::{GlobalResourcePool, PoolError, ResourceKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CropKind {
    Wheat,
    Carrot,
    Potato,
}

impl CropKind {
    pub const ALL: [Self; 3] = [Self::Wheat, Self::Carrot, Self::Potato];

    #[must_use]
    pub const fn seed(self) -> ResourceKind {
        match self {
            Self::Wheat => ResourceKind::WheatSeeds,
            Self::Carrot => ResourceKind::Carrot,
            Self::Potato => ResourceKind::Potato,
        }
    }

    #[must_use]
    pub const fn harvest(self) -> ResourceKind {
        match self {
            Self::Wheat => ResourceKind::Food,
            Self::Carrot => ResourceKind::Carrot,
            Self::Potato => ResourceKind::Potato,
        }
    }

    #[must_use]
    pub const fn growth_ticks(self) -> u32 {
        match self {
            Self::Wheat => 4,
            Self::Carrot => 5,
            Self::Potato => 6,
        }
    }

    #[must_use]
    pub const fn harvest_amount(self) -> i64 {
        match self {
            Self::Wheat => 2,
            Self::Carrot | Self::Potato => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrowingCrop {
    pub kind: CropKind,
    pub age_ticks: u32,
}

impl GrowingCrop {
    #[must_use]
    pub const fn is_mature(self) -> bool {
        self.age_ticks >= self.kind.growth_ticks()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FarmPlot {
    pub id: u32,
    pub crop: Option<GrowingCrop>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FarmingState {
    pub plots: Vec<FarmPlot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FarmingError {
    UnknownPlot(u32),
    PlotOccupied(u32),
    MissingSeed(ResourceKind),
    NotMature(u32),
    HarvestTransfer(PoolError),
}

impl FarmingState {
    #[must_use]
    pub fn new(plot_count: u32) -> Self {
        Self {
            plots: (0..plot_count)
                .map(|id| FarmPlot { id, crop: None })
                .collect(),
        }
    }

    pub fn plant(
        &mut self,
        plot_id: u32,
        kind: CropKind,
        resources: &mut GlobalResourcePool,
    ) -> Result<(), FarmingError> {
        let plot = self
            .plots
            .iter_mut()
            .find(|plot| plot.id == plot_id)
            .ok_or(FarmingError::UnknownPlot(plot_id))?;
        if plot.crop.is_some() {
            return Err(FarmingError::PlotOccupied(plot_id));
        }
        resources
            .try_sub(kind.seed(), 1)
            .map_err(|error| match error {
                PoolError::Insufficient { .. } => FarmingError::MissingSeed(kind.seed()),
                other => FarmingError::HarvestTransfer(other),
            })?;
        plot.crop = Some(GrowingCrop { kind, age_ticks: 0 });
        Ok(())
    }

    pub fn advance_tick(&mut self) {
        for plot in &mut self.plots {
            if let Some(crop) = &mut plot.crop {
                crop.age_ticks = crop.age_ticks.saturating_add(1);
            }
        }
    }

    pub fn harvest(
        &mut self,
        plot_id: u32,
        resources: &mut GlobalResourcePool,
    ) -> Result<(ResourceKind, i64), FarmingError> {
        let plot = self
            .plots
            .iter_mut()
            .find(|plot| plot.id == plot_id)
            .ok_or(FarmingError::UnknownPlot(plot_id))?;
        let crop = plot.crop.ok_or(FarmingError::NotMature(plot_id))?;
        if !crop.is_mature() {
            return Err(FarmingError::NotMature(plot_id));
        }
        let resource = crop.kind.harvest();
        let amount = crop.kind.harvest_amount();
        resources
            .try_add(resource, amount)
            .map_err(FarmingError::HarvestTransfer)?;
        plot.crop = None;
        Ok((resource, amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planting_consumes_seed_and_growth_is_deterministic() {
        let mut state = FarmingState::new(1);
        let mut resources = GlobalResourcePool::new();
        resources.force_add(ResourceKind::WheatSeeds, 1);

        state.plant(0, CropKind::Wheat, &mut resources).unwrap();
        assert_eq!(resources.get(ResourceKind::WheatSeeds), 0);
        assert!(!state.plots[0].crop.unwrap().is_mature());

        for _ in 0..CropKind::Wheat.growth_ticks() {
            state.advance_tick();
        }
        assert!(state.plots[0].crop.unwrap().is_mature());
    }

    #[test]
    fn harvest_is_atomic_when_output_capacity_is_full() {
        let mut state = FarmingState::new(1);
        let mut resources = GlobalResourcePool::new();
        resources.force_add(ResourceKind::WheatSeeds, 1);
        resources.force_add(ResourceKind::Food, ResourceKind::Food.max());
        state.plant(0, CropKind::Wheat, &mut resources).unwrap();
        for _ in 0..CropKind::Wheat.growth_ticks() {
            state.advance_tick();
        }

        assert!(matches!(
            state.harvest(0, &mut resources),
            Err(FarmingError::HarvestTransfer(
                PoolError::WouldExceedMax { .. }
            ))
        ));
        assert!(state.plots[0].crop.is_some());
    }

    #[test]
    fn failed_plant_does_not_occupy_plot() {
        let mut state = FarmingState::new(1);
        let mut resources = GlobalResourcePool::new();

        assert_eq!(
            state.plant(0, CropKind::Carrot, &mut resources),
            Err(FarmingError::MissingSeed(ResourceKind::Carrot))
        );
        assert!(state.plots[0].crop.is_none());
    }
}
