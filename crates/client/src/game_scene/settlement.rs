//! Offline settlement loop: turn gathered materials and farmed food into a
//! persistent camp that can either become a settlement or fail from hunger.

use bevy::prelude::*;
use lk2_core::resource::ResourceKind;
use lk2_core::settlement::{
    CAMP_STONE_COST, CAMP_WOOD_COST, SettlementEvent, SettlementPhase, SettlementState,
};

use super::keybindings::{GameAction, KeyBindings};
use super::offline::OfflineNature;
use super::state::{PlayerActor, ProceduralTerrainSurface, SettlementCampVisual};
use super::util::{CAMPFIRE_PATH, spawn_asset};

pub fn build_camp_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut nature: ResMut<OfflineNature>,
    terrain: Res<ProceduralTerrainSurface>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    players: Query<&Transform, With<PlayerActor>>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::BuildCamp, &keys, &mouse) {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };

    let result = {
        let OfflineNature {
            settlement,
            resources,
            ..
        } = &mut *nature;
        settlement.build_camp(resources)
    };
    match result {
        Ok(SettlementEvent::CampBuilt) => {
            let position = Vec3::new(
                player.translation.x,
                terrain.ground_height(player.translation) + 0.02,
                player.translation.z,
            );
            spawn_asset(
                &mut commands,
                &asset_server,
                CAMPFIRE_PATH,
                position,
                0.92,
                0.0,
                "settlement_camp",
            )
            .insert(SettlementCampVisual);
            info!("[settlement] camp built at {:?}", position);
        }
        Err(error) => info!("[settlement] camp build rejected: {error:?}"),
        Ok(event) => warn!("[settlement] unexpected build event: {event:?}"),
    }
}

pub fn restart_settlement_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut nature: ResMut<OfflineNature>,
    mut commands: Commands,
    camps: Query<Entity, With<SettlementCampVisual>>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::RestartSettlement, &keys, &mouse) {
        return;
    }
    if nature.settlement.phase == SettlementPhase::Wilderness {
        return;
    }
    nature.settlement = SettlementState::default();
    for camp in &camps {
        commands
            .entity(camp)
            .despawn_related::<Children>()
            .despawn();
    }
    info!("[settlement] settlement attempt restarted");
}

pub(crate) fn settlement_hud_text(nature: &OfflineNature, bindings: &KeyBindings) -> String {
    let wood = nature.resources.get(ResourceKind::Wood);
    let stone = nature.resources.get(ResourceKind::Stone);
    let food = nature.resources.get(ResourceKind::Food);
    let build = bindings.binding(GameAction::BuildCamp).label();
    let restart = bindings.binding(GameAction::RestartSettlement).label();
    match nature.settlement.phase {
        SettlementPhase::Wilderness => format!(
            "目标：建造营地  {build}\n木材 {wood}/{CAMP_WOOD_COST}  石头 {stone}/{CAMP_STONE_COST}\n建成后每 5 tick 消耗 1 食物",
        ),
        SettlementPhase::Camp => format!(
            "营地维持中  {}/{} tick\n人口 {}  食物 {}\n每 5 tick 消耗 1 食物",
            nature.settlement.tick,
            lk2_core::settlement::SETTLEMENT_GOAL_TICKS,
            nature.settlement.population,
            food,
        ),
        SettlementPhase::Established => format!(
            "定居成功！人口 {}\n已消耗食物 {}\n按 {restart} 可重新挑战",
            nature.settlement.population, nature.settlement.food_spent,
        ),
        SettlementPhase::Failed => format!(
            "定居失败：食物耗尽\n营地坚持了 {} tick\n按 {restart} 重新挑战",
            nature.settlement.tick,
        ),
    }
}
