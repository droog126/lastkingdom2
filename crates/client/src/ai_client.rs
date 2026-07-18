use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use lightyear::prelude::{
    Connect, LocalAddr, MessageReceiver, MessageSender, PeerAddr, UdpIo, client::ClientPlugins,
};
use lightyear_netcode::prelude::Authentication;
use lightyear_netcode::prelude::client::{NetcodeClient, NetcodeConfig};
use lk2_core::protocol::components::{GameplayHudState, PlayerPos};
use lk2_core::protocol::messages::{
    BuildRecipe, ChatMessage, GameplayCommand, GameplayCommandKind, GameplayFeedback,
};
use lk2_core::protocol::{ControlChannel, ProtocolPlugin};
use lk2_core::transport::{
    DEFAULT_PORT, NETCODE_CLIENT_TIMEOUT_SECS, PRIVATE_KEY, PROTOCOL_ID, generate_client_id,
    parse_connect_arg,
};
use serde_json::json;

const AI_MOVE_INTERVAL_SECS: f32 = 0.4;
const AI_ACTION_INTERVAL_SECS: f32 = 1.0;

#[derive(Resource, Clone)]
struct AiClientConfig {
    server_addr: SocketAddr,
    client_id: u64,
    max_runtime: Option<Duration>,
    output_path: PathBuf,
}

#[derive(Resource, Default)]
struct AiBrain {
    elapsed: f32,
    last_observed_tick: u64,
    observed_ticks: u64,
    last_action_at: f32,
    last_move_at: f32,
    sequence: u64,
    move_commands: u64,
    action_commands: u64,
    feedback_ok: u64,
    feedback_failed: u64,
    feedback_seen: u64,
    first_position: Option<Vec3>,
    current_position: Option<Vec3>,
    last_decision: String,
    last_feedback: String,
    announced: bool,
    state_written_at: f32,
    finished: bool,
}

#[derive(Component)]
struct AiClientEntity;

/// Run a headless Lightyear client that plays through the same online protocol
/// as a human player. This is deliberately a client, not a second authority:
/// every observation comes from replicated state and every action is validated
/// by the server.
pub fn run_ai_client() {
    let args = std::env::args().collect::<Vec<_>>();
    let server_addr = parse_connect_arg(&args).unwrap_or_else(|| {
        format!("127.0.0.1:{DEFAULT_PORT}")
            .parse()
            .expect("default local server address must parse")
    });
    let client_id = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--client-id=")?.parse().ok())
        .filter(|client_id| *client_id != 0)
        .unwrap_or_else(generate_client_id);
    let max_runtime = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--seconds=")?.parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs);
    let output_path = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--ai-output=").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("LK2_AI_OUTPUT")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("LK2_ITER_DIR")
                        .map(PathBuf::from)
                        .map(|dir| dir.join("ai_client.json"))
                })
        })
        .unwrap_or_else(|| PathBuf::from("screenshots/ai_client.json"));

    info!(
        "[ai] starting autonomous client id={} server={} max_runtime={:?}",
        client_id, server_addr, max_runtime
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        // The Lightyear client group must be installed before the shared
        // protocol so its message and replication systems are available.
        .add_plugins(ClientPlugins::default())
        .add_plugins(ProtocolPlugin)
        .insert_resource(AiClientConfig {
            server_addr,
            client_id,
            max_runtime,
            output_path,
        })
        .init_resource::<AiBrain>()
        .add_systems(Startup, spawn_ai_client)
        .add_systems(
            Update,
            (
                connect_ai_client,
                observe_ai_world,
                ai_decide_and_act,
                receive_ai_feedback,
                write_ai_state,
                stop_ai_client,
            )
                .chain(),
        )
        .run();
}

fn spawn_ai_client(mut commands: Commands, config: Res<AiClientConfig>) {
    let auth = Authentication::Manual {
        server_addr: config.server_addr,
        client_id: config.client_id,
        private_key: PRIVATE_KEY,
        protocol_id: PROTOCOL_ID,
    };
    let netcode_config = NetcodeConfig {
        client_timeout_secs: NETCODE_CLIENT_TIMEOUT_SECS,
        token_expire_secs: -1,
        ..default()
    };
    let Ok(netcode_client) = NetcodeClient::new(auth, netcode_config) else {
        error!("[ai] failed to create NetcodeClient");
        return;
    };
    commands.spawn((
        Name::new("AiClient"),
        AiClientEntity,
        netcode_client,
        UdpIo::default(),
        LocalAddr(SocketAddr::from(([0, 0, 0, 0], 0))),
        PeerAddr(config.server_addr),
    ));
}

fn connect_ai_client(
    mut commands: Commands,
    clients: Query<Entity, (With<AiClientEntity>, Added<NetcodeClient>)>,
) {
    for entity in &clients {
        commands.trigger(Connect { entity });
        info!("[ai] connection requested");
    }
}

fn observe_ai_world(
    mut brain: ResMut<AiBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
) {
    let Some((position, hud, _controlled)) = players.iter().next() else {
        return;
    };
    if !position.0.is_finite() {
        warn!("[ai] ignored non-finite replicated player position");
        return;
    }
    if brain.first_position.is_none() {
        brain.first_position = Some(position.0);
        info!("[ai] acquired controlled player at {:?}", position.0);
    }
    brain.current_position = Some(position.0);
    if let Some(hud) = hud {
        if hud.tick > brain.last_observed_tick {
            brain.last_observed_tick = hud.tick;
            brain.observed_ticks = brain.observed_ticks.saturating_add(1);
        }
    }
}

fn ai_decide_and_act(
    time: Res<Time>,
    mut brain: ResMut<AiBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    mut clients: Query<
        (
            &mut MessageSender<GameplayCommand>,
            &mut MessageSender<ChatMessage>,
            Option<&lightyear::prelude::Connected>,
        ),
        With<AiClientEntity>,
    >,
) {
    brain.elapsed += time.delta_secs();
    let Ok((mut gameplay_sender, mut chat_sender, connected)) = clients.single_mut() else {
        return;
    };
    if connected.is_none() {
        return;
    }
    let Some((position, hud, _controlled)) = players.iter().next() else {
        return;
    };
    let block = hud.map_or_else(
        || scene_pos_to_block(position.0),
        |hud| hud.player_block_pos,
    );
    let tick = hud.map_or(0, |hud| hud.tick);

    if !brain.announced {
        chat_sender.send::<ControlChannel>(ChatMessage {
            client_tick: tick,
            text: "AI 客户端已进入世界，开始自主探索。".to_string(),
        });
        brain.announced = true;
        brain.last_decision = "announce_and_explore".to_string();
    }

    if brain.elapsed - brain.last_move_at >= AI_MOVE_INTERVAL_SECS {
        let (dx_milli, dz_milli) = exploration_direction(brain.elapsed);
        send_command(
            &mut gameplay_sender,
            &mut brain,
            tick,
            block,
            GameplayCommandKind::MoveWorld {
                dx_milli,
                dz_milli,
                dy_milli: 0,
            },
        );
        brain.last_move_at = brain.elapsed;
        brain.move_commands = brain.move_commands.saturating_add(1);
    }

    if brain.elapsed - brain.last_action_at < AI_ACTION_INTERVAL_SECS {
        return;
    }
    brain.last_action_at = brain.elapsed;

    let kind = choose_action(hud, brain.elapsed);
    brain.last_decision = format!("{kind:?}");
    send_command(&mut gameplay_sender, &mut brain, tick, block, kind);
    brain.action_commands = brain.action_commands.saturating_add(1);
}

fn choose_action(hud: Option<&GameplayHudState>, elapsed: f32) -> GameplayCommandKind {
    let Some(hud) = hud else {
        return GameplayCommandKind::GatherFootBlock;
    };
    // FoundNation is deliberately early enough to be observed in a short
    // loop run. The server still validates the resource cost and position;
    // this is a real gameplay action, not a client-side counter.
    if hud.nation_id.is_none() && hud.tick >= 3 {
        return GameplayCommandKind::FoundNation;
    }
    if hud.inventory_wood >= 5 && (elapsed as u32).is_multiple_of(8) {
        return GameplayCommandKind::Craft(BuildRecipe::PlankPack);
    }
    if hud.monster_count > 0 && (elapsed as u32).is_multiple_of(5) {
        return GameplayCommandKind::KillNearestCreature;
    }
    if (elapsed as u32).is_multiple_of(4) {
        return GameplayCommandKind::MineTarget {
            target: [
                hud.player_block_pos[0],
                hud.player_block_pos[1] - 1,
                hud.player_block_pos[2],
            ],
        };
    }
    GameplayCommandKind::GatherFootBlock
}

fn exploration_direction(elapsed: f32) -> (i16, i16) {
    match ((elapsed / 6.0).floor() as u32) % 4 {
        0 => (1000, 0),
        1 => (0, 1000),
        2 => (-1000, 0),
        _ => (0, -1000),
    }
}

fn send_command(
    sender: &mut MessageSender<GameplayCommand>,
    brain: &mut AiBrain,
    tick: u64,
    player_block: [i32; 3],
    kind: GameplayCommandKind,
) {
    brain.sequence = brain.sequence.wrapping_add(1).max(1);
    sender.send::<ControlChannel>(GameplayCommand {
        sequence: brain.sequence,
        tick,
        player_block,
        kind,
    });
}

fn receive_ai_feedback(
    mut brain: ResMut<AiBrain>,
    mut clients: Query<&mut MessageReceiver<GameplayFeedback>, With<AiClientEntity>>,
) {
    let Ok(mut receiver) = clients.single_mut() else {
        return;
    };
    for feedback in receiver.receive() {
        brain.feedback_seen = brain.feedback_seen.saturating_add(1);
        brain.last_feedback = feedback.summary.clone();
        if feedback.ok {
            brain.feedback_ok = brain.feedback_ok.saturating_add(1);
        } else {
            brain.feedback_failed = brain.feedback_failed.saturating_add(1);
        }
    }
}

fn write_ai_state(
    config: Res<AiClientConfig>,
    time: Res<Time>,
    mut brain: ResMut<AiBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    clients: Query<Option<&lightyear::prelude::Connected>, With<AiClientEntity>>,
) {
    if brain.finished || time.elapsed_secs() - brain.state_written_at < 1.0 {
        return;
    }
    let Ok(connection) = clients.single() else {
        return;
    };
    write_ai_state_file(
        &config.output_path,
        &brain,
        connection.is_some(),
        players.iter().next(),
    );
    brain.state_written_at = time.elapsed_secs();
}

fn write_ai_state_file(
    path: &PathBuf,
    brain: &AiBrain,
    connected: bool,
    player: Option<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
) {
    let (position, hud) = player
        .map(|(position, hud, _)| (Some(position.0), hud))
        .unwrap_or((None, None));
    let payload = json!({
        "schema": 1,
        "role": "client_ai",
        "connected": connected,
        "elapsed_secs": brain.elapsed,
        "observed_ticks": brain.observed_ticks,
        "server_tick": hud.map_or(brain.last_observed_tick, |hud| hud.tick),
        "player": {
            "pos": position.map(|pos| [pos.x, pos.y, pos.z]),
            "block_pos": hud.map(|hud| hud.player_block_pos),
            "nation_id": hud.and_then(|hud| hud.nation_id),
            "inventory_wood": hud.map_or(0, |hud| hud.inventory_wood),
            "blocks_gathered": hud.map_or(0, |hud| hud.blocks_gathered),
            "monsters_killed": hud.map_or(0, |hud| hud.monsters_killed),
            "nations_founded": hud.map_or(0, |hud| hud.nations_founded)
        },
        "decision": {
            "strategy": "observe_replicated_state_then_explore_and_interact",
            "last": brain.last_decision,
            "last_feedback": brain.last_feedback,
            "move_commands": brain.move_commands,
            "action_commands": brain.action_commands,
            "feedback_seen": brain.feedback_seen,
            "feedback_ok": brain.feedback_ok,
            "feedback_failed": brain.feedback_failed
        },
        "movement_probe": {
            "first_pos": brain.first_position.map(|pos| [pos.x, pos.y, pos.z]),
            "current_pos": brain.current_position.map(|pos| [pos.x, pos.y, pos.z])
        }
    });
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = std::fs::write(path, text);
    }
}

fn stop_ai_client(
    config: Res<AiClientConfig>,
    time: Res<Time>,
    mut brain: ResMut<AiBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    clients: Query<Option<&lightyear::prelude::Connected>, With<AiClientEntity>>,
) {
    let Some(max_runtime) = config.max_runtime else {
        return;
    };
    if brain.finished || time.elapsed() < max_runtime {
        return;
    }
    let connected = clients.single().is_ok_and(|value| value.is_some());
    write_ai_state_file(
        &config.output_path,
        &brain,
        connected,
        players.iter().next(),
    );
    brain.finished = true;
    info!(
        "[ai] finished after {:.1}s: moves={}, actions={}, feedback_ok={}, feedback_failed={}",
        brain.elapsed,
        brain.move_commands,
        brain.action_commands,
        brain.feedback_ok,
        brain.feedback_failed
    );
    std::process::exit(0);
}

fn scene_pos_to_block(position: Vec3) -> [i32; 3] {
    [
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ]
}
