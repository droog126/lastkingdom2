//! A Codex-driven online client whose decisions come from the local Codex CLI.
//!
//! Codex is intentionally kept outside the Bevy frame schedule. The rendered
//! online scene remains responsible for the window and player presentation;
//! the game sends a compact replicated observation to a worker thread, which asks
//! `codex exec` for one schema-constrained action, and the main thread sends
//! only the corresponding existing gameplay protocol command.  The server is
//! still the authority for every action.

use std::{
    collections::VecDeque,
    fs,
    net::SocketAddr,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

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
use serde::{Deserialize, Serialize};
use serde_json::json;

const CODEX_DECISION_INTERVAL_SECS: f32 = 3.0;
// The first local `codex exec` may warm plugins and model metadata before it
// produces a response. Keep this above that cold-start cost; the game never
// blocks its render thread because the request runs on the worker thread.
const CODEX_TIMEOUT_SECS: u64 = 90;
const CODEX_MAX_ERROR_CHARS: usize = 400;

#[derive(Resource, Clone)]
struct CodexClientConfig {
    server_addr: SocketAddr,
    client_id: u64,
    max_runtime: Option<Duration>,
    output_path: PathBuf,
    runtime: CodexRuntimeConfig,
}

#[derive(Clone)]
struct CodexRuntimeConfig {
    binary: String,
    work_dir: PathBuf,
    scratch_dir: PathBuf,
    timeout: Duration,
}

#[derive(Resource)]
pub(crate) struct CodexWorker {
    pub(crate) requests: mpsc::Sender<CodexRequest>,
    pub(crate) responses: Arc<Mutex<VecDeque<CodexResponse>>>,
}

#[derive(Clone, Debug)]
pub(crate) struct CodexRequest {
    pub(crate) request_id: u64,
    pub(crate) observation: CodexObservation,
}

#[derive(Debug)]
pub(crate) struct CodexResponse {
    pub(crate) request_id: u64,
    pub(crate) result: Result<CodexDecisionPayload, String>,
}

#[derive(Resource, Default)]
struct CodexBrain {
    elapsed: f32,
    last_observed_tick: u64,
    observed_ticks: u64,
    last_request_at: f32,
    request_sequence: u64,
    active_request: Option<u64>,
    requests: u64,
    successful_decisions: u64,
    failed_decisions: u64,
    fallback_actions: u64,
    move_commands: u64,
    action_commands: u64,
    feedback_ok: u64,
    feedback_failed: u64,
    feedback_seen: u64,
    first_position: Option<Vec3>,
    current_position: Option<Vec3>,
    last_intent: String,
    last_reason: String,
    last_feedback: String,
    last_error: String,
    announced: bool,
    state_written_at: f32,
    finished: bool,
}

#[derive(Component)]
pub struct CodexControlled;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct CodexObservation {
    pub(crate) tick: u64,
    pub(crate) position: [f32; 3],
    pub(crate) block_pos: [i32; 3],
    pub(crate) nation_id: Option<u32>,
    pub(crate) monsters_killed: u32,
    pub(crate) blocks_gathered: u32,
    pub(crate) nations_founded: u32,
    pub(crate) inventory_wood: i64,
    pub(crate) inventory_food: i64,
    pub(crate) inventory_apple: i64,
    pub(crate) inventory_soul: i64,
    pub(crate) pool_wood: i64,
    pub(crate) pool_food: i64,
    pub(crate) pool_apple: i64,
    pub(crate) pool_soul: i64,
    pub(crate) flag_count: u32,
    pub(crate) total_nations: u32,
    pub(crate) monster_count: u32,
    pub(crate) status_line: String,
    pub(crate) last_feedback: String,
    pub(crate) last_intent: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CodexDecisionPayload {
    pub(crate) intent: String,
    #[serde(default)]
    pub(crate) dx_milli: i16,
    #[serde(default)]
    pub(crate) dz_milli: i16,
    #[serde(default)]
    pub(crate) target: Option<[i32; 3]>,
    #[serde(default)]
    pub(crate) reason: Option<String>,
}

pub fn run_codex_client() {
    let args = std::env::args().collect::<Vec<_>>();
    let config = codex_config_from_args(&args);
    info!(
        "[codex] starting client id={} server={} binary={} max_runtime={:?}",
        config.client_id, config.server_addr, config.runtime.binary, config.max_runtime
    );

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .add_plugins(ClientPlugins::default())
        .add_plugins(ProtocolPlugin)
        .insert_resource(config)
        .init_resource::<CodexBrain>()
        .add_systems(Startup, (spawn_codex_client, start_codex_worker).chain())
        .add_systems(
            Update,
            (
                connect_codex_client,
                observe_codex_world,
                receive_codex_decisions,
                schedule_codex_decision,
                receive_codex_feedback,
                write_codex_state,
                stop_codex_client,
            )
                .chain(),
        )
        .run();
}

/// Install Codex decision systems into the fully rendered online scene.
///
/// The online scene owns the window, camera, replicated player visuals, and
/// Lightyear connection. This function only adds the observe-decide-act
/// driver and marks that connection as Codex-controlled; it does not spawn a
/// second headless client or a second authority.
pub fn install_codex_gameplay(app: &mut App, args: &[String]) {
    app.insert_resource(codex_config_from_args(args))
        .init_resource::<CodexBrain>()
        .add_systems(Startup, start_codex_worker)
        .add_systems(
            Update,
            (
                observe_codex_world,
                receive_codex_decisions,
                schedule_codex_decision,
                receive_codex_feedback,
                write_codex_state,
                stop_codex_client,
            )
                .chain(),
        );
}

/// Install only the asynchronous Codex worker for the fully local game scene.
/// The scene supplies its own observation and action projection.
pub(crate) fn install_codex_worker(app: &mut App, args: &[String]) {
    app.insert_resource(codex_config_from_args(args))
        .add_systems(Startup, start_codex_worker);
}

fn codex_config_from_args(args: &[String]) -> CodexClientConfig {
    let server_addr = parse_connect_arg(args).unwrap_or_else(|| {
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
        .find_map(|arg| arg.strip_prefix("--codex-output=").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("LK2_CODEX_OUTPUT")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("LK2_ITER_DIR")
                        .map(PathBuf::from)
                        .map(|dir| dir.join("codex_client.json"))
                })
        })
        .unwrap_or_else(|| PathBuf::from("screenshots/codex_client.json"));

    let root = std::env::temp_dir().join(format!("lastkingdom2-codex-{}", std::process::id()));
    let work_dir = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--codex-workdir=").map(PathBuf::from))
        .unwrap_or_else(|| root.join("work"));
    let scratch_dir = root.join("requests");
    let _ = fs::create_dir_all(&work_dir);
    let _ = fs::create_dir_all(&scratch_dir);
    let binary = args
        .iter()
        .find_map(|arg| arg.strip_prefix("--codex-bin=").map(ToOwned::to_owned))
        .or_else(|| std::env::var("LK2_CODEX_BIN").ok())
        .unwrap_or_else(default_codex_binary);

    CodexClientConfig {
        server_addr,
        client_id,
        max_runtime,
        output_path,
        runtime: CodexRuntimeConfig {
            binary,
            work_dir,
            scratch_dir,
            timeout: Duration::from_secs(CODEX_TIMEOUT_SECS),
        },
    }
}

fn default_codex_binary() -> String {
    if cfg!(windows) {
        "codex.cmd".to_string()
    } else {
        "codex".to_string()
    }
}

fn spawn_codex_client(mut commands: Commands, config: Res<CodexClientConfig>) {
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
        error!("[codex] failed to create NetcodeClient");
        return;
    };
    commands.spawn((
        Name::new("CodexClient"),
        CodexControlled,
        netcode_client,
        UdpIo::default(),
        LocalAddr(SocketAddr::from(([0, 0, 0, 0], 0))),
        PeerAddr(config.server_addr),
    ));
}

fn start_codex_worker(mut commands: Commands, config: Res<CodexClientConfig>) {
    let (request_tx, request_rx) = mpsc::channel::<CodexRequest>();
    let responses = Arc::new(Mutex::new(VecDeque::new()));
    let response_queue = Arc::clone(&responses);
    let runtime = config.runtime.clone();
    let spawn_result = thread::Builder::new()
        .name("lk2-codex-worker".to_string())
        .spawn(move || codex_worker_loop(request_rx, response_queue, runtime));
    if let Err(error) = spawn_result {
        error!("[codex] failed to start worker: {error}");
        return;
    }
    commands.insert_resource(CodexWorker {
        requests: request_tx,
        responses,
    });
}

fn codex_worker_loop(
    requests: mpsc::Receiver<CodexRequest>,
    responses: Arc<Mutex<VecDeque<CodexResponse>>>,
    runtime: CodexRuntimeConfig,
) {
    for request in requests {
        let result = run_codex_decision(&runtime, request.request_id, &request.observation);
        if let Ok(mut queue) = responses.lock() {
            queue.push_back(CodexResponse {
                request_id: request.request_id,
                result,
            });
        }
    }
}

fn connect_codex_client(
    mut commands: Commands,
    clients: Query<Entity, (With<CodexControlled>, Added<NetcodeClient>)>,
) {
    for entity in &clients {
        commands.trigger(Connect { entity });
        info!("[codex] connection requested");
    }
}

fn observe_codex_world(
    mut brain: ResMut<CodexBrain>,
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
        warn!("[codex] ignored non-finite replicated player position");
        return;
    }
    if brain.first_position.is_none() {
        brain.first_position = Some(position.0);
        info!("[codex] acquired controlled player at {:?}", position.0);
    }
    brain.current_position = Some(position.0);
    if let Some(hud) = hud {
        if hud.tick > brain.last_observed_tick {
            brain.last_observed_tick = hud.tick;
            brain.observed_ticks = brain.observed_ticks.saturating_add(1);
        }
    }
}

fn receive_codex_decisions(
    worker: Option<Res<CodexWorker>>,
    mut brain: ResMut<CodexBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    mut clients: Query<&mut MessageSender<GameplayCommand>, With<CodexControlled>>,
) {
    let Some(worker) = worker else {
        return;
    };
    let Ok(mut gameplay_sender) = clients.single_mut() else {
        return;
    };
    let Some((position, hud, _controlled)) = players.iter().next() else {
        return;
    };
    let tick = hud.map_or(brain.last_observed_tick, |value| value.tick);
    let block = hud.map_or_else(
        || scene_pos_to_block(position.0),
        |value| value.player_block_pos,
    );
    let responses = worker
        .responses
        .lock()
        .map(|mut queue| queue.drain(..).collect::<Vec<_>>())
        .unwrap_or_default();
    if responses.is_empty() {
        return;
    }

    for response in responses {
        if brain.active_request != Some(response.request_id) {
            continue;
        }
        brain.active_request = None;
        match response.result {
            Ok(decision) => {
                brain.successful_decisions = brain.successful_decisions.saturating_add(1);
                brain.last_intent = decision.intent.clone();
                brain.last_reason = decision.reason.clone().unwrap_or_default();
                if let Some(kind) = decision.to_gameplay_command(hud) {
                    send_command(&mut gameplay_sender, &mut brain, tick, block, kind);
                }
            }
            Err(error) => {
                brain.failed_decisions = brain.failed_decisions.saturating_add(1);
                brain.last_error = truncate(&error, CODEX_MAX_ERROR_CHARS);
                brain.last_intent = "fallback".to_string();
                if let Some(hud) = hud {
                    if let Some(kind) = fallback_action(hud, brain.elapsed) {
                        brain.fallback_actions = brain.fallback_actions.saturating_add(1);
                        send_command(&mut gameplay_sender, &mut brain, tick, block, kind);
                    }
                }
                warn!(
                    "[codex] decision failed; fallback action sent: {}",
                    brain.last_error
                );
            }
        }
    }
}

fn schedule_codex_decision(
    time: Res<Time>,
    worker: Option<Res<CodexWorker>>,
    mut brain: ResMut<CodexBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    mut clients: Query<
        (
            &mut MessageSender<ChatMessage>,
            Option<&lightyear::prelude::Connected>,
        ),
        With<CodexControlled>,
    >,
) {
    brain.elapsed += time.delta_secs();
    if brain.active_request.is_some()
        || brain.elapsed - brain.last_request_at < CODEX_DECISION_INTERVAL_SECS
    {
        return;
    }
    let Some(worker) = worker else {
        return;
    };
    let Ok((mut chat_sender, connection)) = clients.single_mut() else {
        return;
    };
    if connection.is_none() {
        return;
    }
    let Some((position, hud, _controlled)) = players.iter().next() else {
        return;
    };
    let tick = hud.map_or(brain.last_observed_tick, |value| value.tick);
    if !brain.announced {
        chat_sender.send::<ControlChannel>(ChatMessage {
            client_tick: tick,
            text: "Codex 客户端已进入世界，开始自主行动。".to_string(),
        });
        brain.announced = true;
    }
    let observation = CodexObservation::from_replicated(position, hud, &brain);
    brain.request_sequence = brain.request_sequence.saturating_add(1).max(1);
    let request_id = brain.request_sequence;
    if worker
        .requests
        .send(CodexRequest {
            request_id,
            observation,
        })
        .is_err()
    {
        brain.last_error = "Codex worker channel closed".to_string();
        return;
    }
    brain.last_request_at = brain.elapsed;
    brain.active_request = Some(request_id);
    brain.requests = brain.requests.saturating_add(1);
}

fn receive_codex_feedback(
    mut brain: ResMut<CodexBrain>,
    mut clients: Query<&mut MessageReceiver<GameplayFeedback>, With<CodexControlled>>,
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

fn write_codex_state(
    config: Res<CodexClientConfig>,
    time: Res<Time>,
    mut brain: ResMut<CodexBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    clients: Query<Option<&lightyear::prelude::Connected>, With<CodexControlled>>,
) {
    if brain.finished || time.elapsed_secs() - brain.state_written_at < 1.0 {
        return;
    }
    let Ok(connection) = clients.single() else {
        return;
    };
    write_codex_state_file(
        &config.output_path,
        &config.runtime,
        &brain,
        connection.is_some(),
        players.iter().next(),
    );
    brain.state_written_at = time.elapsed_secs();
}

fn write_codex_state_file(
    path: &PathBuf,
    runtime: &CodexRuntimeConfig,
    brain: &CodexBrain,
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
        "role": "codex_client",
        "provider": "codex_cli",
        "codex_binary": runtime.binary,
        "connected": connected,
        "elapsed_secs": brain.elapsed,
        "observed_ticks": brain.observed_ticks,
        "server_tick": hud.map_or(brain.last_observed_tick, |value| value.tick),
        "player": {
            "pos": position.map(|pos| [pos.x, pos.y, pos.z]),
            "block_pos": hud.map(|value| value.player_block_pos),
            "nation_id": hud.and_then(|value| value.nation_id),
            "inventory_wood": hud.map_or(0, |value| value.inventory_wood),
            "blocks_gathered": hud.map_or(0, |value| value.blocks_gathered),
            "monsters_killed": hud.map_or(0, |value| value.monsters_killed),
            "nations_founded": hud.map_or(0, |value| value.nations_founded)
        },
        "decision": {
            "requests": brain.requests,
            "successful_decisions": brain.successful_decisions,
            "failed_decisions": brain.failed_decisions,
            "fallback_actions": brain.fallback_actions,
            "move_commands": brain.move_commands,
            "action_commands": brain.action_commands,
            "feedback_seen": brain.feedback_seen,
            "feedback_ok": brain.feedback_ok,
            "feedback_failed": brain.feedback_failed,
            "last_intent": brain.last_intent,
            "last_reason": brain.last_reason,
            "last_error": brain.last_error,
            "last_feedback": brain.last_feedback
        },
        "movement_probe": {
            "first_pos": brain.first_position.map(|pos| [pos.x, pos.y, pos.z]),
            "current_pos": brain.current_position.map(|pos| [pos.x, pos.y, pos.z])
        }
    });
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(&payload) {
        let _ = fs::write(path, text);
    }
}

fn stop_codex_client(
    config: Res<CodexClientConfig>,
    time: Res<Time>,
    mut brain: ResMut<CodexBrain>,
    players: Query<(
        &PlayerPos,
        Option<&GameplayHudState>,
        &lightyear::prelude::Controlled,
    )>,
    clients: Query<Option<&lightyear::prelude::Connected>, With<CodexControlled>>,
) {
    let Some(max_runtime) = config.max_runtime else {
        return;
    };
    if brain.finished || time.elapsed() < max_runtime {
        return;
    }
    let connected = clients.single().is_ok_and(|value| value.is_some());
    write_codex_state_file(
        &config.output_path,
        &config.runtime,
        &brain,
        connected,
        players.iter().next(),
    );
    brain.finished = true;
    info!(
        "[codex] finished after {:.1}s: requests={}, successful={}, failed={}, fallbacks={}, feedback_ok={}",
        brain.elapsed,
        brain.requests,
        brain.successful_decisions,
        brain.failed_decisions,
        brain.fallback_actions,
        brain.feedback_ok
    );
    std::process::exit(0);
}

impl CodexObservation {
    fn from_replicated(
        position: &PlayerPos,
        hud: Option<&GameplayHudState>,
        brain: &CodexBrain,
    ) -> Self {
        let block_pos = hud
            .map(|value| value.player_block_pos)
            .unwrap_or_else(|| scene_pos_to_block(position.0));
        Self {
            tick: hud.map_or(brain.last_observed_tick, |value| value.tick),
            position: [position.0.x, position.0.y, position.0.z],
            block_pos,
            nation_id: hud.and_then(|value| value.nation_id),
            monsters_killed: hud.map_or(0, |value| value.monsters_killed),
            blocks_gathered: hud.map_or(0, |value| value.blocks_gathered),
            nations_founded: hud.map_or(0, |value| value.nations_founded),
            inventory_wood: hud.map_or(0, |value| value.inventory_wood),
            inventory_food: hud.map_or(0, |value| value.inventory_food),
            inventory_apple: hud.map_or(0, |value| value.inventory_apple),
            inventory_soul: hud.map_or(0, |value| value.inventory_soul),
            pool_wood: hud.map_or(0, |value| value.pool_wood),
            pool_food: hud.map_or(0, |value| value.pool_food),
            pool_apple: hud.map_or(0, |value| value.pool_apple),
            pool_soul: hud.map_or(0, |value| value.pool_soul),
            flag_count: hud.map_or(0, |value| value.flag_count),
            total_nations: hud.map_or(0, |value| value.total_nations),
            monster_count: hud.map_or(0, |value| value.monster_count),
            status_line: hud.map_or_else(String::new, |value| value.status_line.clone()),
            last_feedback: brain.last_feedback.clone(),
            last_intent: brain.last_intent.clone(),
        }
    }
}

impl CodexDecisionPayload {
    fn to_gameplay_command(&self, hud: Option<&GameplayHudState>) -> Option<GameplayCommandKind> {
        match self.intent.as_str() {
            "move" => Some(GameplayCommandKind::MoveWorld {
                dx_milli: self.dx_milli.clamp(-1000, 1000),
                dz_milli: self.dz_milli.clamp(-1000, 1000),
                dy_milli: 0,
            }),
            "jump" => Some(GameplayCommandKind::Jump),
            "mine" => Some(GameplayCommandKind::MineTarget {
                target: self.target.unwrap_or_else(|| {
                    hud.map_or([0, 0, 0], |value| {
                        [
                            value.player_block_pos[0],
                            value.player_block_pos[1] - 1,
                            value.player_block_pos[2],
                        ]
                    })
                }),
            }),
            "gather" => Some(GameplayCommandKind::GatherFootBlock),
            "place" => Some(GameplayCommandKind::PlaceWoodFootBlock),
            "craft_plank" => Some(GameplayCommandKind::Craft(BuildRecipe::PlankPack)),
            "craft_campfire" => Some(GameplayCommandKind::Craft(BuildRecipe::Campfire)),
            "found_nation" => Some(GameplayCommandKind::FoundNation),
            "kill" => Some(GameplayCommandKind::KillNearestCreature),
            "mount" => Some(GameplayCommandKind::MountCart),
            "idle" => None,
            _ => None,
        }
    }
}

fn build_codex_prompt(observation: &CodexObservation) -> String {
    // Keep the prompt on one command-line argument. On Windows the npm
    // launcher is a `.cmd` wrapper, and multiline stdin/arguments can make
    // `codex exec -` wait for another input frame.
    let state = serde_json::to_string(observation).unwrap_or_else(|_| "{}".to_string());
    format!(
        "You are the autonomous player of Last Kingdom 2. Choose exactly one next action from the allowed intents. You are not a coding assistant in this turn: do not edit files, do not run tools, and do not invent actions. Return only one JSON object with intent, optional dx_milli/dz_milli, optional target, and optional reason. The server validates the action. Movement values are signed thousandths and should normally be -1000, 0, or 1000. For mine, target a block adjacent to your current block position. Observed game state: {state}. Return the next action now.",
    )
}

fn run_codex_decision(
    runtime: &CodexRuntimeConfig,
    request_id: u64,
    observation: &CodexObservation,
) -> Result<CodexDecisionPayload, String> {
    let output_path = runtime
        .scratch_dir
        .join(format!("response-{request_id}.json"));
    let mut arguments = vec![
        "exec".to_string(),
        "--ephemeral".to_string(),
        "--sandbox".to_string(),
        "read-only".to_string(),
        "--skip-git-repo-check".to_string(),
        "--color".to_string(),
        "never".to_string(),
        "--output-last-message".to_string(),
        output_path.display().to_string(),
        "-C".to_string(),
        runtime.work_dir.display().to_string(),
    ];
    arguments.push(build_codex_prompt(observation));
    let mut command = if cfg!(windows)
        && (runtime.binary.ends_with(".cmd")
            || runtime.binary.ends_with(".bat")
            || runtime.binary.ends_with(".ps1"))
    {
        let mut command = if runtime.binary.ends_with(".ps1") {
            let mut command = Command::new("powershell.exe");
            command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
            command
        } else {
            let mut command = Command::new("cmd.exe");
            command.args(["/D", "/S", "/C"]);
            command
        };
        command.arg(&runtime.binary);
        command
    } else {
        Command::new(&runtime.binary)
    };
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to start {}: {error}", runtime.binary))?;

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll Codex: {error}"))?
        {
            break status;
        }
        if started.elapsed() >= runtime.timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "Codex timed out after {}s",
                runtime.timeout.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(50));
    };
    if !status.success() {
        let error = child
            .wait_with_output()
            .map(|output| String::from_utf8_lossy(&output.stderr).into_owned())
            .unwrap_or_else(|_| "no stderr".to_string());
        return Err(format!(
            "Codex exited with {status}: {}",
            truncate(error.trim(), CODEX_MAX_ERROR_CHARS)
        ));
    }
    let text = fs::read_to_string(&output_path)
        .map_err(|error| format!("Codex did not write a final JSON response: {error}"))?;
    let _ = fs::remove_file(&output_path);
    parse_codex_decision(&text)
}

fn parse_codex_decision(text: &str) -> Result<CodexDecisionPayload, String> {
    let trimmed = text.trim();
    let candidates = [trimmed, extract_json_object(trimmed).unwrap_or(trimmed)];
    for candidate in candidates {
        if let Ok(decision) = serde_json::from_str::<CodexDecisionPayload>(candidate) {
            if is_allowed_intent(&decision.intent) {
                return Ok(decision);
            }
            return Err(format!(
                "Codex returned unsupported intent {:?}",
                decision.intent
            ));
        }
    }
    Err(format!(
        "Codex response was not a valid action JSON: {}",
        truncate(trimmed, CODEX_MAX_ERROR_CHARS)
    ))
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (start < end).then_some(&text[start..=end])
}

fn is_allowed_intent(intent: &str) -> bool {
    matches!(
        intent,
        "move"
            | "jump"
            | "mine"
            | "gather"
            | "place"
            | "craft_plank"
            | "craft_campfire"
            | "found_nation"
            | "kill"
            | "mount"
            | "idle"
    )
}

fn fallback_action(hud: &GameplayHudState, elapsed: f32) -> Option<GameplayCommandKind> {
    if hud.nation_id.is_none() && hud.tick >= 3 {
        return Some(GameplayCommandKind::FoundNation);
    }
    if hud.inventory_wood >= 5 && (elapsed as u32).is_multiple_of(8) {
        return Some(GameplayCommandKind::Craft(BuildRecipe::PlankPack));
    }
    if hud.monster_count > 0 && (elapsed as u32).is_multiple_of(5) {
        return Some(GameplayCommandKind::KillNearestCreature);
    }
    Some(GameplayCommandKind::GatherFootBlock)
}

fn send_command(
    sender: &mut MessageSender<GameplayCommand>,
    brain: &mut CodexBrain,
    tick: u64,
    player_block: [i32; 3],
    kind: GameplayCommandKind,
) {
    brain.action_commands = brain.action_commands.saturating_add(1);
    if matches!(kind, GameplayCommandKind::MoveWorld { .. }) {
        brain.move_commands = brain.move_commands.saturating_add(1);
    }
    brain.request_sequence = brain.request_sequence.saturating_add(1).max(1);
    sender.send::<ControlChannel>(GameplayCommand {
        sequence: brain.request_sequence,
        tick,
        player_block,
        kind,
    });
}

fn scene_pos_to_block(position: Vec3) -> [i32; 3] {
    [
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    ]
}

fn truncate(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hud() -> GameplayHudState {
        GameplayHudState {
            tick: 12,
            player_block_pos: [4, 3, 5],
            player_pos: [4.5, 3.0, 5.5],
            nation_id: None,
            monsters_killed: 0,
            blocks_gathered: 0,
            nations_founded: 0,
            inventory_wood: 0,
            inventory_food: 0,
            inventory_apple: 0,
            inventory_soul: 0,
            pool_wood: 0,
            pool_food: 0,
            pool_apple: 0,
            pool_soul: 0,
            flag_count: 0,
            total_nations: 0,
            monster_count: 0,
            observer_anomalies: 0,
            observer_invariant_violations: 0,
            status_line: "ready".to_string(),
        }
    }

    #[test]
    fn structured_codex_json_maps_to_a_move_command() {
        let decision = parse_codex_decision(
            r#"{"intent":"move","dx_milli":1000,"dz_milli":-1000,"reason":"探路"}"#,
        )
        .expect("valid decision");
        assert_eq!(decision.intent, "move");
        assert!(matches!(
            decision.to_gameplay_command(Some(&hud())),
            Some(GameplayCommandKind::MoveWorld {
                dx_milli: 1000,
                dz_milli: -1000,
                dy_milli: 0,
            })
        ));
    }

    #[test]
    fn fenced_json_is_recovered_but_unsupported_intent_is_rejected() {
        let decision = parse_codex_decision("Here is the action: {\"intent\":\"gather\"}")
            .expect("embedded JSON should be recovered");
        assert_eq!(decision.intent, "gather");
        let error = parse_codex_decision(r#"{"intent":"delete_world"}"#)
            .expect_err("arbitrary actions must not reach the protocol");
        assert!(error.contains("unsupported intent"));
    }

    #[test]
    fn mine_without_target_uses_the_adjacent_player_block() {
        let decision = CodexDecisionPayload {
            intent: "mine".to_string(),
            dx_milli: 0,
            dz_milli: 0,
            target: None,
            reason: None,
        };
        assert_eq!(
            decision.to_gameplay_command(Some(&hud())),
            Some(GameplayCommandKind::MineTarget { target: [4, 2, 5] })
        );
    }

    #[test]
    fn prompt_contains_state_and_forbids_tool_actions() {
        let brain = CodexBrain::default();
        let observation = CodexObservation::from_replicated(
            &PlayerPos(Vec3::new(4.5, 3.0, 5.5)),
            Some(&hud()),
            &brain,
        );
        let prompt = build_codex_prompt(&observation);
        assert!(prompt.contains("\"tick\":12"));
        assert!(prompt.contains("do not edit files"));
    }
}
