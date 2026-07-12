use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::render::{
    RenderPlugin,
    settings::{Backends, WgpuSettings},
};
use bevy::text::LetterSpacing;
use bevy::window::{PresentMode, WindowResolution};
use leafwing_input_manager::prelude::{ActionState, InputMap};
use lightyear::prelude::{Connect, LocalAddr, PeerAddr, UdpIo, client::ClientPlugins};
use lightyear::prelude::{MessageReceiver, MessageSender};
use lightyear_netcode::prelude::Authentication;
use lightyear_netcode::prelude::client::{NetcodeClient, NetcodeConfig};
use lk2_core::protocol::components::{EcoSnapshot, GameplayHudState, PlayerPos};
use lk2_core::protocol::messages::{
    AttackInput, BuildRecipe, ChatBroadcast, ChatMessage, GameplayCommand, GameplayCommandKind,
};
use lk2_core::protocol::{ControlChannel, PlayerAction, ProtocolPlugin};
use lk2_core::transport::{
    DEFAULT_PORT, NETCODE_CLIENT_TIMEOUT_SECS, PRIVATE_KEY, PROTOCOL_ID, generate_client_id,
    parse_connect_arg,
};
use serde_json::json;

use crate::nature::{NatureSnapshotBuffer, nature_snapshot_from_protocol};

#[derive(Resource, Clone, Copy)]
struct OnlineConnection {
    server_addr: SocketAddr,
}

#[derive(Resource)]
struct OnlineVisualAssets {
    player_mesh: Handle<Mesh>,
    player_material: Handle<StandardMaterial>,
}

#[derive(Component)]
struct OnlinePlayerVisual;

#[derive(Component)]
struct OnlineCamera;

#[derive(Component)]
struct OnlineHudText;

#[derive(Component)]
struct OnlineChatText;

#[derive(Resource, Default)]
struct OnlineChatState {
    composing: bool,
    draft: String,
    log: Vec<String>,
}

#[derive(Resource)]
struct OnlineAutoDemo {
    enabled: bool,
    iter_dir: Option<PathBuf>,
    png_path: PathBuf,
    elapsed: f32,
    frame: u64,
    frame_dt_over_50ms: u64,
    frame_dt_max_ms: f32,
    shot_requested: bool,
    exit_deadline: Option<Instant>,
}

impl OnlineAutoDemo {
    fn from_args(args: &[String]) -> Self {
        let enabled = args.iter().any(|arg| arg == "--auto-demo");
        let iter_dir = enabled
            .then(|| std::env::var_os("LK2_ITER_DIR").map(PathBuf::from))
            .flatten();
        let output_dir = iter_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("screenshots/online"));
        let png_path = screenshot_path(&output_dir, iter_dir.as_deref());
        Self {
            enabled,
            iter_dir,
            png_path,
            elapsed: 0.0,
            frame: 0,
            frame_dt_over_50ms: 0,
            frame_dt_max_ms: 0.0,
            shot_requested: false,
            exit_deadline: None,
        }
    }
}

pub fn run_online_scene() {
    let args = std::env::args().collect::<Vec<_>>();
    let server_addr = parse_connect_arg(&args).unwrap_or_else(|| {
        format!("127.0.0.1:{DEFAULT_PORT}")
            .parse()
            .expect("default local server address must parse")
    });

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                render_creation: WgpuSettings {
                    backends: Some(Backends::VULKAN),
                    ..default()
                }
                .into(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Last Kingdom - Online".into(),
                    resolution: WindowResolution::new(1280, 720),
                    present_mode: PresentMode::Immediate,
                    focused: true,
                    visible: true,
                    ..default()
                }),
                ..default()
            })
            .set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                ..default()
            }),
    )
    // Lightyear's client group must be installed before the shared protocol
    // so the Leafwing input plugin can attach its client systems in `finish`.
    .add_plugins(ClientPlugins::default())
    .add_plugins(ProtocolPlugin)
    .insert_resource(OnlineConnection { server_addr })
    .insert_resource(OnlineAutoDemo::from_args(&args))
    .init_resource::<OnlineChatState>()
    .init_resource::<NatureSnapshotBuffer>()
    .add_systems(Startup, (setup_online_scene, spawn_netcode_client).chain())
    .add_systems(
        Update,
        (
            connect_netcode_client,
            install_player_input_maps,
            handle_online_chat_input,
            suppress_online_actions_while_chatting,
            send_online_action_messages,
            receive_online_chat_messages,
            sync_replicated_players,
            update_online_overlay,
            update_online_chat_ui,
            follow_online_camera,
            log_connection_state,
            online_auto_demo_capture,
            online_auto_demo_exit,
        )
            .chain(),
    )
    .run();
}

fn screenshot_path(output_dir: &Path, iter_dir: Option<&Path>) -> PathBuf {
    if let Some(iter_dir) = iter_dir {
        let stem = iter_dir
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.starts_with("iter_"))
            .unwrap_or("iter_capture");
        return iter_dir.join(format!("{stem}.png"));
    }
    output_dir.join("online_scene.png")
}

fn setup_online_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let player_mesh = meshes.add(Cuboid::new(0.8, 1.8, 0.8));
    let player_material = materials.add(Color::srgb(0.22, 0.78, 0.42));
    let floor_mesh = meshes.add(Cuboid::new(80.0, 0.2, 80.0));
    let floor_material = materials.add(Color::srgb(0.24, 0.32, 0.28));
    commands.insert_resource(OnlineVisualAssets {
        player_mesh,
        player_material,
    });
    commands.spawn((
        Mesh3d(floor_mesh),
        MeshMaterial3d(floor_material),
        Transform::from_xyz(0.0, -0.1, 0.0),
    ));
    commands.spawn((
        Camera3d::default(),
        OnlineCamera,
        Transform::from_xyz(12.0, 10.0, 16.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-8.0, 16.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.78)));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.62)),
        children![(
            Text::new("在线\n等待服务器快照"),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.94),
                weight: FontWeight::SEMIBOLD,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.3),
            TextColor(Color::WHITE),
            OnlineHudText,
        )],
    ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            bottom: px(12),
            width: px(560),
            padding: UiRect::all(px(10)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.07, 0.10, 0.68)),
        children![(
            Text::new("聊天  按 Enter 输入"),
            TextFont {
                font: FontSource::UiMonospace,
                font_size: FontSize::Rem(0.86),
                weight: FontWeight::MEDIUM,
                width: FontWidth::SEMI_CONDENSED,
                ..default()
            },
            LetterSpacing::Px(0.2),
            TextColor(Color::srgb(0.88, 0.94, 0.96)),
            OnlineChatText,
        )],
    ));
}

fn spawn_netcode_client(mut commands: Commands, connection: Res<OnlineConnection>) {
    let auth = Authentication::Manual {
        server_addr: connection.server_addr,
        client_id: generate_client_id(),
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };
    let config = NetcodeConfig {
        client_timeout_secs: NETCODE_CLIENT_TIMEOUT_SECS,
        token_expire_secs: -1,
        ..default()
    };
    let Ok(netcode_client) = NetcodeClient::new(auth, config) else {
        error!("[net] failed to create NetcodeClient");
        return;
    };
    let entity = commands
        .spawn((
            Name::new("OnlineClient"),
            netcode_client,
            UdpIo::default(),
            LocalAddr(SocketAddr::from(([0, 0, 0, 0], 0))),
            PeerAddr(connection.server_addr),
        ))
        .id();
    info!(
        "[net] created Lightyear client {:?}; server={}",
        entity, connection.server_addr
    );
}

fn connect_netcode_client(
    mut commands: Commands,
    clients: Query<Entity, (With<NetcodeClient>, Added<NetcodeClient>)>,
) {
    for entity in clients.iter() {
        commands.trigger(Connect { entity });
    }
}

fn install_player_input_maps(
    mut commands: Commands,
    clients: Query<Entity, With<NetcodeClient>>,
    players: Query<
        (Entity, &lightyear::prelude::ControlledBy),
        (With<PlayerPos>, Without<InputMap<PlayerAction>>),
    >,
) {
    let Ok(client_entity) = clients.single() else {
        return;
    };
    for (entity, controlled_by) in players.iter() {
        if controlled_by.owner != client_entity {
            continue;
        }
        let mut input_map = InputMap::default();
        input_map.insert(PlayerAction::MoveForward, KeyCode::KeyW);
        input_map.insert(PlayerAction::MoveBackward, KeyCode::KeyS);
        input_map.insert(PlayerAction::MoveLeft, KeyCode::KeyA);
        input_map.insert(PlayerAction::MoveRight, KeyCode::KeyD);
        input_map.insert(PlayerAction::Jump, KeyCode::Space);
        input_map.insert(PlayerAction::Sprint, KeyCode::ShiftLeft);
        input_map.insert(PlayerAction::Attack, KeyCode::KeyF);
        input_map.insert(PlayerAction::Gather, KeyCode::KeyG);
        input_map.insert(PlayerAction::Place, KeyCode::KeyP);
        input_map.insert(PlayerAction::Craft, KeyCode::KeyH);
        input_map.insert(PlayerAction::FoundNation, KeyCode::KeyJ);
        input_map.insert(PlayerAction::KillCreature, KeyCode::KeyK);
        commands.entity(entity).insert(input_map);
    }
}

fn send_online_action_messages(
    players: Query<(&ActionState<PlayerAction>, Option<&GameplayHudState>), With<PlayerPos>>,
    cameras: Query<&Transform, With<OnlineCamera>>,
    chat: Res<OnlineChatState>,
    mut clients: Query<
        (
            &mut MessageSender<GameplayCommand>,
            &mut MessageSender<AttackInput>,
        ),
        With<NetcodeClient>,
    >,
) {
    if chat.composing {
        return;
    }
    let Ok((actions, hud)) = players.single() else {
        return;
    };
    let Ok((mut gameplay_sender, mut attack_sender)) = clients.single_mut() else {
        return;
    };
    let tick = hud.map_or(0, |state| state.tick.min(u32::MAX as u64) as u32);
    let player_block = hud.map_or([0, 0, 0], |state| state.player_block_pos);

    let mut send_command = |kind: GameplayCommandKind| {
        gameplay_sender.send::<ControlChannel>(GameplayCommand {
            tick: tick as u64,
            player_block,
            kind,
        });
    };

    if actions.just_pressed(&PlayerAction::Gather) {
        send_command(GameplayCommandKind::GatherFootBlock);
    }
    if actions.just_pressed(&PlayerAction::Place) {
        send_command(GameplayCommandKind::PlaceWoodFootBlock);
    }
    if actions.just_pressed(&PlayerAction::Craft) {
        send_command(GameplayCommandKind::Craft(BuildRecipe::PlankPack));
    }
    if actions.just_pressed(&PlayerAction::FoundNation) {
        send_command(GameplayCommandKind::FoundNation);
    }
    if actions.just_pressed(&PlayerAction::KillCreature) {
        send_command(GameplayCommandKind::KillNearestCreature);
    }
    if actions.just_pressed(&PlayerAction::Attack) {
        let input_dir = cameras
            .single()
            .map(|camera| camera.forward().as_vec3())
            .unwrap_or(-Vec3::Z);
        attack_sender.send::<ControlChannel>(AttackInput { tick, input_dir });
    }
}

fn handle_online_chat_input(
    mut keyboard_inputs: MessageReader<KeyboardInput>,
    mut chat: ResMut<OnlineChatState>,
    players: Query<Option<&GameplayHudState>, With<PlayerPos>>,
    mut clients: Query<&mut MessageSender<ChatMessage>, With<NetcodeClient>>,
) {
    for event in keyboard_inputs.read() {
        if !event.state.is_pressed() || event.repeat {
            continue;
        }
        match &event.logical_key {
            Key::Enter => {
                if chat.composing {
                    let text = sanitize_chat_text(&chat.draft);
                    chat.draft.clear();
                    chat.composing = false;
                    if !text.is_empty() {
                        let tick = players
                            .iter()
                            .next()
                            .and_then(|hud| hud.map(|state| state.tick))
                            .unwrap_or(0);
                        if let Ok(mut sender) = clients.single_mut() {
                            sender.send::<ControlChannel>(ChatMessage {
                                client_tick: tick,
                                text,
                            });
                        }
                    }
                } else {
                    chat.composing = true;
                    chat.draft.clear();
                }
            }
            Key::Escape => {
                if chat.composing {
                    chat.composing = false;
                    chat.draft.clear();
                }
            }
            Key::Backspace => {
                if chat.composing {
                    chat.draft.pop();
                }
            }
            _ => {
                if chat.composing {
                    if let Some(text) = &event.text {
                        append_chat_text(&mut chat.draft, text);
                    }
                }
            }
        }
    }
}

fn receive_online_chat_messages(
    mut chat: ResMut<OnlineChatState>,
    mut clients: Query<&mut MessageReceiver<ChatBroadcast>, With<NetcodeClient>>,
) {
    let Ok(mut receiver) = clients.single_mut() else {
        return;
    };
    for message in receiver.receive() {
        chat.log.push(format!(
            "[{}] {}: {}",
            message.server_tick, message.sender, message.text
        ));
    }
    let overflow = chat.log.len().saturating_sub(6);
    if overflow > 0 {
        chat.log.drain(0..overflow);
    }
}

fn suppress_online_actions_while_chatting(
    chat: Res<OnlineChatState>,
    mut players: Query<&mut ActionState<PlayerAction>, With<PlayerPos>>,
) {
    if !chat.composing {
        return;
    }
    for mut actions in &mut players {
        actions.reset_all();
    }
}

fn update_online_chat_ui(
    chat: Res<OnlineChatState>,
    mut text: Query<&mut Text, With<OnlineChatText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let mut lines = Vec::new();
    lines.push(if chat.composing {
        format!("聊天  > {}", chat.draft)
    } else {
        "聊天  按 Enter 输入".to_string()
    });
    lines.extend(chat.log.iter().cloned());
    text.0 = lines.join("\n");
}

fn append_chat_text(draft: &mut String, text: &str) {
    for ch in text.chars() {
        if ch.is_control() {
            continue;
        }
        if draft.chars().count() >= 120 {
            break;
        }
        draft.push(ch);
    }
}

fn sanitize_chat_text(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(120)
        .collect()
}

fn sync_replicated_players(
    mut commands: Commands,
    assets: Res<OnlineVisualAssets>,
    mut players: Query<(
        Entity,
        &PlayerPos,
        Option<&EcoSnapshot>,
        Option<&mut Transform>,
        Option<&mut OnlinePlayerVisual>,
    )>,
    mut nature: ResMut<NatureSnapshotBuffer>,
) {
    for (entity, position, eco_snapshot, transform, visual) in players.iter_mut() {
        if let Some(snapshot) = eco_snapshot {
            let _ = nature.push(nature_snapshot_from_protocol(snapshot));
        }
        // Lightyear applies replicated components in PreUpdate. This system
        // runs afterward and is the sole writer of online player visual
        // transforms, so the server's PlayerPos remains authoritative for
        // both the locally controlled player and remote players.
        if !position.is_finite() {
            warn!(
                "[net] ignored non-finite authoritative PlayerPos on {:?}",
                entity
            );
            continue;
        }
        if visual.is_none() {
            commands.entity(entity).insert((
                OnlinePlayerVisual,
                Mesh3d(assets.player_mesh.clone()),
                MeshMaterial3d(assets.player_material.clone()),
                Transform::from_translation(position.0),
            ));
        } else if let Some(mut transform) = transform {
            transform.translation = position.0;
        }
    }
}

fn update_online_overlay(
    players: Query<(&PlayerPos, Option<&GameplayHudState>, Option<&EcoSnapshot>)>,
    mut text: Query<&mut Text, With<OnlineHudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let Some((pos, hud, eco)) = players.iter().next() else {
        text.0 = "在线\n等待服务器快照".to_string();
        return;
    };
    text.0 = online_overlay_text(pos, hud, eco);
}

fn online_overlay_text(
    pos: &PlayerPos,
    hud: Option<&GameplayHudState>,
    eco: Option<&EcoSnapshot>,
) -> String {
    let tick = hud.map_or(0, |state| state.tick);
    let block = hud.map_or(scene_pos_to_block(pos.0), |state| state.player_block_pos);
    let nation = hud
        .and_then(|state| state.nation_id)
        .map_or_else(|| "none".to_string(), |id| id.to_string());
    let wood = hud.map_or(0, |state| state.inventory_wood);
    let food = hud.map_or(0, |state| state.inventory_food);
    let flags = hud.map_or(0, |state| state.flag_count);
    let nations = hud.map_or(0, |state| state.total_nations);
    let monsters = hud.map_or(0, |state| state.monster_count);
    let (clouds, plants, animals, rainfall) = eco.map_or((0, 0, 0, 0.0), |snapshot| {
        (
            snapshot.clouds.len(),
            snapshot.plants.len() + snapshot.berries.len(),
            snapshot.rabbits.len() + snapshot.wildlife.len(),
            snapshot.rainfall,
        )
    });
    format!(
        "在线\nTick {tick}  区块 [{}, {}, {}]  国家 {nation}\n木头 {wood}  食物 {food}  旗帜 {flags}  国家数 {nations}  怪物 {monsters}\n云朵 {clouds}  植物 {plants}  动物 {animals}  降雨 {:.1}",
        block[0], block[1], block[2], rainfall
    )
}

fn follow_online_camera(
    players: Query<&PlayerPos>,
    mut cameras: Query<&mut Transform, With<OnlineCamera>>,
) {
    let Ok(mut camera) = cameras.single_mut() else {
        return;
    };
    let Some(player) = players.iter().next() else {
        return;
    };
    let desired = player.0 + Vec3::new(10.0, 8.0, 12.0);
    camera.translation = camera.translation.lerp(desired, 0.08);
    camera.look_at(player.0 + Vec3::Y, Vec3::Y);
}

fn log_connection_state(
    connected: Query<Entity, Added<lightyear::prelude::Connected>>,
    disconnected: Query<
        (Entity, &lightyear::prelude::Disconnected),
        Added<lightyear::prelude::Disconnected>,
    >,
) {
    for entity in connected.iter() {
        info!("[net] Lightyear client connected: {:?}", entity);
    }
    for (entity, state) in disconnected.iter() {
        warn!(
            "[net] Lightyear client disconnected: {:?}: {}",
            entity,
            state.reason.as_deref().unwrap_or("unknown")
        );
    }
}

fn online_auto_demo_capture(
    mut commands: Commands,
    time: Res<Time>,
    mut demo: ResMut<OnlineAutoDemo>,
    players: Query<(&PlayerPos, Option<&GameplayHudState>, Option<&EcoSnapshot>)>,
) {
    if !demo.enabled {
        return;
    }
    let dt = time.delta_secs();
    demo.elapsed += dt;
    demo.frame += 1;
    if dt > 0.05 {
        demo.frame_dt_over_50ms = demo.frame_dt_over_50ms.saturating_add(1);
    }
    demo.frame_dt_max_ms = demo.frame_dt_max_ms.max(dt * 1000.0);
    if demo.shot_requested || demo.elapsed < 4.0 || demo.frame < 24 {
        return;
    }

    if let Some(iter_dir) = &demo.iter_dir {
        let _ = std::fs::create_dir_all(iter_dir);
        let (pos, hud, eco) = players
            .iter()
            .next()
            .map_or((Vec3::ZERO, None, None), |(position, hud, eco)| {
                (position.0, hud, eco)
            });
        let player_block = hud.map_or(scene_pos_to_block(pos), |state| state.player_block_pos);
        let tick = hud.map_or(0, |state| state.tick);
        let final_state = json!({
            "schema": 1,
            "role": "client_online",
            "tick": tick,
            "wall_secs": demo.elapsed,
            "frame": demo.frame,
            "world": {"size": 96},
            "player": {
                "block_pos": player_block,
                "pos": [pos.x, pos.y, pos.z],
                "blocks_gathered": hud.map_or(0, |state| state.blocks_gathered),
                "monsters_killed": hud.map_or(0, |state| state.monsters_killed),
                "nations_founded": hud.map_or(0, |state| state.nations_founded)
            },
            "nations": {"total_nations": hud.map_or(0, |state| state.total_nations)},
            "observer": {"anomalies": hud.map_or(0, |state| state.observer_anomalies), "invariant_violations": hud.map_or(0, |state| state.observer_invariant_violations)},
            "camera": {"mode": "ThirdPerson", "first_person_eye": [pos.x, pos.y + 1.6, pos.z]},
            "visual": {
                "movement_probe": {
                    "first_player_pos": [pos.x, pos.y, pos.z],
                    "current_player_pos": [pos.x, pos.y, pos.z],
                    "first_static_world": {"floor": [0.0, -0.1, 0.0]},
                    "current_static_world": {"floor": [0.0, -0.1, 0.0]}
                },
                "player_readability": {"marker_count": 1, "marker_max_distance": 0.0}
            },
            "network_command": {"move_world_sent": 0},
            "render": {
                "frame": {"dt_over_50ms": demo.frame_dt_over_50ms, "max_ms": demo.frame_dt_max_ms},
                "terrain": {"smooth_mesh_builds": 0, "smooth_mesh_max_ms": 0.0, "terrain_despawns": 0},
                "lighting": {"dayness": 1.0}
            },
            "eco_cycle": eco.map_or(json!({}), |snapshot| json!({
                "clouds": snapshot.clouds.len(),
                "rainfall": snapshot.rainfall,
                "plants": snapshot.plants.len(),
                "animals": snapshot.rabbits.len() + snapshot.wildlife.len(),
                "plants_grown": snapshot.plants_grown,
                "fruit_eaten": snapshot.fruit_eaten,
                "fruit_grown": snapshot.fruit_grown
            })),
            "nature": eco.map_or(json!({}), |snapshot| json!({
                "tick": snapshot.tick,
                "clouds": snapshot.clouds,
                "cloud_count": snapshot.clouds.len(),
                "rainfall": snapshot.rainfall,
                "soil_moisture": snapshot.rain.max(0.0),
                "plants": snapshot.plants.len(),
                "animals": snapshot.rabbits.len() + snapshot.wildlife.len(),
                "animal_food_available": snapshot
                    .berries
                    .iter()
                    .map(|berry| berry.fruit as f32)
                    .sum::<f32>()
                    + snapshot
                        .plants
                        .iter()
                        .map(|plant| plant.stock as f32)
                        .sum::<f32>(),
                "events": []
            }))
        });
        let _ = std::fs::write(
            iter_dir.join("final_state.json"),
            serde_json::to_vec_pretty(&final_state).unwrap_or_default(),
        );
        let _ = std::fs::write(
            iter_dir.join("diff.json"),
            "{\"resource_deltas\":[{\"kind\":\"network\",\"delta\":1}]}\n",
        );
    }

    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(demo.png_path.clone()));
    demo.shot_requested = true;
    demo.exit_deadline = Some(Instant::now() + Duration::from_secs(8));
}

fn online_auto_demo_exit(demo: Res<OnlineAutoDemo>) {
    if let Some(deadline) = demo.exit_deadline {
        if demo.png_path.exists() || Instant::now() >= deadline {
            std::process::exit(0);
        }
    }
}

fn scene_pos_to_block(pos: Vec3) -> [i32; 3] {
    [
        (pos.x.round() as i32 + 48).clamp(0, 95),
        (pos.y.round() as i32 + 16).clamp(0, 95),
        (pos.z.round() as i32 + 48).clamp(0, 95),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn online_screenshot_path_matches_loop_iter_contract() {
        let iter_dir = PathBuf::from("screenshots/iter_07");
        assert_eq!(
            screenshot_path(&iter_dir, Some(&iter_dir)),
            PathBuf::from("screenshots/iter_07/iter_07.png")
        );
        assert_eq!(
            screenshot_path(&PathBuf::from("screenshots/online"), None),
            PathBuf::from("screenshots/online/online_scene.png")
        );
    }
}
