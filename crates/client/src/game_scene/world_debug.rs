//! F2 content and runtime debug panel for the offline playable scene.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lk2_core::resource::{GlobalResourcePool, ResourceKind};
use lk2_core::world::content::{
    ContentId, GAME_CONTENT_CAVERN, GAME_CONTENT_DUNGEON, GAME_CONTENT_EMPTY,
    GAME_CONTENT_MONSTER_TERRITORY, GAME_CONTENT_SETTLEMENT, GAME_CONTENT_TREASURE_VAULT,
    GAME_CONTENT_VERTICAL_PASSAGE, GAME_CONTENT_WILDERNESS,
};

use super::content_visuals::{
    content_profile_label, is_monster_anchor, LivingContentLayout, MONSTER_ANCHOR_PILLAR_SCALE,
    MONSTER_DECORATIVE_MARKER_SCALE,
};
use super::keybindings::{GameAction, KeyBindings, UiSettings};
use super::offline::OfflineNature;
use super::state::{
    BerryBush, BossActor, Cloud, GrassTuft, LivingSceneState, PlayerActor, Rabbit, RainDrop, Wolf,
};

#[derive(Resource, Default)]
pub struct WorldDebugUiState {
    pub open: bool,
}

#[derive(Component)]
pub struct WorldDebugUiRoot;

#[derive(Component)]
pub struct WorldDebugBodyText;

#[derive(SystemParam)]
pub struct WorldDebugCounts<'w, 's> {
    players: Query<'w, 's, (), With<PlayerActor>>,
    bosses: Query<'w, 's, (), With<BossActor>>,
    clouds: Query<'w, 's, (), With<Cloud>>,
    rain: Query<'w, 's, (), With<RainDrop>>,
    rabbits: Query<'w, 's, (), With<Rabbit>>,
    wolves: Query<'w, 's, (), With<Wolf>>,
    grass: Query<'w, 's, (), With<GrassTuft>>,
    berries: Query<'w, 's, (), With<BerryBush>>,
}

impl WorldDebugCounts<'_, '_> {
    fn presentation_line(&self) -> String {
        format!(
            "表现层        玩家 {}  Boss {}  云朵 {}  雨滴 {}  兔子 {}  狼 {}  草 {}  浆果模型 {}",
            self.players.iter().count(),
            self.bosses.iter().count(),
            self.clouds.iter().count(),
            self.rain.iter().count(),
            self.rabbits.iter().count(),
            self.wolves.iter().count(),
            self.grass.iter().count(),
            self.berries.iter().count(),
        )
    }
}

pub fn setup_world_debug_ui(mut commands: Commands) {
    let root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(24),
                right: px(24),
                bottom: px(24),
                max_height: percent(58),
                padding: UiRect::all(px(16)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.05, 0.075, 0.92)),
            BorderColor::all(Color::srgba(0.72, 0.64, 0.28, 0.85)),
            Visibility::Hidden,
            WorldDebugUiRoot,
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("内容调试  F2"),
            TextFont {
                font: FontSource::UiSansSerif,
                font_size: FontSize::Px(20.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.93, 0.72)),
        ));
        parent
            .spawn((Node {
                max_height: px(430),
                overflow: Overflow::scroll_y(),
                ..default()
            },))
            .with_children(|body| {
                body.spawn((
                    Text::new(""),
                    TextFont {
                        font: FontSource::UiMonospace,
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.98, 0.94)),
                    WorldDebugBodyText,
                ));
            });
    });
}

pub fn toggle_world_debug_ui(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut state: ResMut<WorldDebugUiState>,
) {
    if bindings.menu_open {
        return;
    }
    if bindings.just_pressed(GameAction::OpenWorldStatus, &keys, &mouse) {
        state.open = !state.open;
    }
}

pub fn update_world_debug_ui(
    nature: Res<OfflineNature>,
    scene: Res<LivingSceneState>,
    ui_state: Res<WorldDebugUiState>,
    ui_settings: Res<UiSettings>,
    layout: Option<Res<LivingContentLayout>>,
    counts: WorldDebugCounts,
    mut root: Query<(&mut Visibility, &mut BackgroundColor), With<WorldDebugUiRoot>>,
    mut body: Query<&mut Text, With<WorldDebugBodyText>>,
) {
    if let Ok((mut visibility, mut background)) = root.single_mut() {
        *visibility = if ui_state.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        background.0 = Color::srgba(0.035, 0.05, 0.075, ui_settings.panel_opacity);
    }
    if let Ok(mut text) = body.single_mut() {
        text.0 = world_debug_text(&nature, &scene, layout.as_deref(), &counts);
    }
}

fn world_debug_text(
    nature: &OfflineNature,
    scene: &LivingSceneState,
    layout: Option<&LivingContentLayout>,
    counts: &WorldDebugCounts,
) -> String {
    let detail = &nature.snapshot.detailed_ecology;
    let resources = &nature.resources;
    let total_current = resource_sum(resources, |pool, kind| pool.get(kind));
    let total_added = resource_sum(resources, |pool, kind| {
        pool.audit_added.get(&kind).copied().unwrap_or(0)
    });
    let total_spent = resource_sum(resources, |pool, kind| {
        pool.audit_subtracted.get(&kind).copied().unwrap_or(0)
    });
    let conservation = resources
        .verify_conservation()
        .map_or("守恒异常", |_| "正常");

    let mut text = format!(
        "时钟          tick {}  待处理 {:.2}s  帧 {}  已运行 {:.1}s\n\
         生态          云朵 {}  降雨强度 {:.2}  累计降雨 {:.2}  二氧化碳 {:.2}\n\
         生命          植物 {}  存量 {}  浆果 {}  水果 {}  兔子 {}  野生动物 {}\n\
         周期统计      植物生长 {}  水果生成 {}  水果消耗 {}  兔子出生 {}  野生动物出生 {}\n\
         资源          当前 {}  增加 {}  消耗 {}  非零 {}  资源守恒 {}\n\
         关键资源      木头 {}  食物 {}  苹果 {}  灵魂 {}  日石 {}  冰霜核心 {}  生命根 {}\n\
         流转审计      食物 +{}  苹果 -{}  木头 +{}  生命根 +{}\n\
         {}\n",
        nature.tick,
        nature.accumulated_secs,
        scene.frame,
        scene.elapsed,
        detail.clouds.len(),
        detail.rain,
        detail.rainfall,
        detail.co2,
        detail.plants.len(),
        nature.snapshot.ecology.plant_units,
        detail.berries.len(),
        detail.berries.iter().map(|berry| berry.fruit).sum::<u32>(),
        detail.rabbits.len(),
        detail.wildlife.len(),
        detail.plants_grown,
        detail.fruit_grown,
        detail.fruit_eaten,
        detail.rabbits_born,
        detail.wildlife_born,
        total_current,
        total_added,
        total_spent,
        resources.non_zero_count(),
        conservation,
        resources.get(ResourceKind::Wood),
        resources.get(ResourceKind::Food),
        resources.get(ResourceKind::Apple),
        resources.get(ResourceKind::Soul),
        resources.get(ResourceKind::Sunstone),
        resources.get(ResourceKind::Frostcore),
        resources.get(ResourceKind::LivingRoot),
        resources
            .audit_added
            .get(&ResourceKind::Food)
            .copied()
            .unwrap_or(0),
        resources
            .audit_subtracted
            .get(&ResourceKind::Apple)
            .copied()
            .unwrap_or(0),
        resources
            .audit_added
            .get(&ResourceKind::Wood)
            .copied()
            .unwrap_or(0),
        resources
            .audit_added
            .get(&ResourceKind::LivingRoot)
            .copied()
            .unwrap_or(0),
        counts.presentation_line(),
    );

    if let Some(layout) = layout {
        text.push_str(&content_debug_text(layout));
    }
    text.push('\n');
    text.push_str("资源        当前 / 增加 / 消耗\n");
    text.push_str("--------------------------------\n");
    for kind in ResourceKind::ALL {
        let current = resources.get(*kind);
        let added = resources.audit_added.get(kind).copied().unwrap_or(0);
        let spent = resources.audit_subtracted.get(kind).copied().unwrap_or(0);
        if current > 0 || added > 0 || spent > 0 {
            text.push_str(&format!(
                "{:<18} {:>6} / {:>6} / {:>6}\n",
                kind.label_zh(),
                current,
                added,
                spent,
            ));
        }
    }
    if resources.non_zero_count() == 0 && total_added == 0 && total_spent == 0 {
        text.push_str("暂时没有资源流动。\n");
    }
    text
}

pub(crate) fn content_debug_text(layout: &LivingContentLayout) -> String {
    let volume = &layout.volume;
    let mut text = format!(
        "内容          种子 {}  配置 {}  尺寸 {:?}  单元格 {}\n\
         计数          空地 {}  野外 {}  聚落 {}  怪物 {}  洞穴 {}  地牢 {}  宝藏 {}  通道 {}\n\
         立柱          大型锚点 [1,1,1] 缩放 {:.2}  装饰标记缩放 {:.2}\n",
        volume.seed,
        content_profile_label(layout.profile),
        volume.dimensions,
        volume.cells.len(),
        volume.count(GAME_CONTENT_EMPTY),
        volume.count(GAME_CONTENT_WILDERNESS),
        volume.count(GAME_CONTENT_SETTLEMENT),
        volume.count(GAME_CONTENT_MONSTER_TERRITORY),
        volume.count(GAME_CONTENT_CAVERN),
        volume.count(GAME_CONTENT_DUNGEON),
        volume.count(GAME_CONTENT_TREASURE_VAULT),
        volume.count(GAME_CONTENT_VERTICAL_PASSAGE),
        MONSTER_ANCHOR_PILLAR_SCALE,
        MONSTER_DECORATIVE_MARKER_SCALE,
    );
    text.push_str("地表 y=1     A 锚点  M 怪物  S 聚落  W 野外  V 通道  . 空地\n");
    text.push_str(&content_layer_map(layout, 1));
    text.push_str("地下 y=0     C 洞穴  D 地牢  T 宝藏  V 通道  . 空地\n");
    text.push_str(&content_layer_map(layout, 0));
    text
}

fn content_layer_map(layout: &LivingContentLayout, y: usize) -> String {
    let [width, _, depth] = layout.volume.dimensions;
    let mut text = String::new();
    for z in 0..depth {
        text.push_str(&format!("z{z:<2} "));
        for x in 0..width {
            let cell = [x, y, z];
            let symbol = layout
                .volume
                .get(cell)
                .map_or('.', |content| content_symbol(content, cell));
            text.push(symbol);
            text.push(' ');
        }
        text.push('\n');
    }
    text
}

fn content_symbol(content: ContentId, cell: [usize; 3]) -> char {
    if is_monster_anchor(cell) {
        return 'A';
    }
    match content {
        GAME_CONTENT_EMPTY => '.',
        GAME_CONTENT_WILDERNESS => 'W',
        GAME_CONTENT_SETTLEMENT => 'S',
        GAME_CONTENT_MONSTER_TERRITORY => 'M',
        GAME_CONTENT_CAVERN => 'C',
        GAME_CONTENT_DUNGEON => 'D',
        GAME_CONTENT_TREASURE_VAULT => 'T',
        GAME_CONTENT_VERTICAL_PASSAGE => 'V',
        _ => '?',
    }
}

fn resource_sum(
    resources: &GlobalResourcePool,
    value: impl Fn(&GlobalResourcePool, ResourceKind) -> i64,
) -> i64 {
    ResourceKind::ALL
        .iter()
        .map(|kind| value(resources, *kind))
        .sum()
}
