# 万国起源 · 服务器/客户端架构
## N 客户端 + 1 服务端 / 每维度 1 服务端

---

## 一、总体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      维度路由器 (Dimension Router)            │
│                    （负载均衡 + 玩家调度）                     │
└─────────────┬───────────────────────────────┬─────────────┘
              │                                 │
    ┌─────────▼─────────┐          ┌──────────▼──────────┐
    │   主世界服务端       │          │   以太界服务端       │
    │   (Overworld Server)│          │  (Aether Server)   │
    │                     │          │                     │
    │  ┌───────────────┐  │          │  ┌───────────────┐  │
    │  │  自运行世界    │  │          │  │  自运行世界    │  │
    │  │  (20 TPS)     │  │          │  │  (10 TPS)     │  │
    │  │               │  │          │  │               │  │
    │  │  生态演化      │  │          │  │  重力扭曲      │  │
    │  │  物理模拟      │  │          │  │  幽魂AI       │  │
    │  │  国家战争      │  │          │  │  虚空精华      │  │
    │  └───────┬───────┘  │          │  └───────┬───────┘  │
    │          │           │          │          │           │
    │  ┌───────▼───────┐  │          │  ┌───────▼───────┐  │
    │  │  网络层        │  │          │  │  网络层        │  │
    │  │  (lightyear)   │  │          │  │  (lightyear)   │  │
    │  └───────┬───────┘  │          │  └───────┬───────┘  │
    └──────────┼───────────┘          └──────────┼───────────┘
               │                                  │
    ┌──────────┼──────────┐           ┌──────────┼──────────┐
    │          │          │           │          │          │
┌───▼───┐ ┌───▼───┐ ┌───▼───┐   ┌───▼───┐ ┌───▼───┐ ┌───▼───┐
│玩家A  │ │玩家B  │ │玩家C  │   │玩家D  │ │玩家E  │ │玩家F  │
│客户端 │ │客户端 │ │客户端 │   │客户端 │ │客户端 │ │客户端 │
└───────┘ └───────┘ └───────┘   └───────┘ └───────┘ └───────┘
```

---

## 二、服务端架构

### 2.1 核心原则

1. **自运行**：服务端启动后，世界自行演化，无需客户端
2. **确定性**：给定相同输入序列，所有服务端产生相同输出
3. **可追溯**：任何时刻的状态都可以从初始种子 + 输入日志复原
4. **可分支**：任何时刻可以创建时间线分支，独立演化

### 2.2 服务端进程结构

```
wanguo-server (单进程，多线程)
│
├─ Main Thread (主线程)
│   ├─ Tick Scheduler (Tick 调度器)
│   ├─ State Manager (状态管理)
│   └─ Event Router (事件路由)
│
├─ Game Logic Threads (游戏逻辑线程池)
│   ├─ World Tick Thread (世界 Tick 线程)
│   │   └─ ECS System Execution (ECS 系统执行)
│   ├─ Physics Thread (物理线程)
│   │   └─ Collision Detection, Movement
│   ├─ AI Thread (AI 线程)
│   │   └─ Pathfinding, Behavior Trees
│   └─ Ecology Thread (生态线程)
│       └─ Plant Growth, Animal Migration
│
├─ Network Threads (网络线程池)
│   ├─ Connection Handler (连接处理)
│   ├─ Input Receiver (输入接收)
│   ├─ State Broadcaster (状态广播)
│   └─ Delta Compressor (差分压缩)
│
├─ IO Threads (IO 线程池)
│   ├─ Persistence Writer (持久化写入)
│   ├─ Snapshot Manager (快照管理)
│   └─ Audit Logger (审计日志)
│
└─ Background Threads (后台线程)
    ├─ World Generation (世界生成)
    ├─ Garbage Collection (ECS 实体清理)
    └─ Metrics Collector (指标收集)
```

### 2.3 Tick 系统

```rust
// 核心 Tick 循环
pub struct TickEngine {
    current_tick: u64,
    tick_rate: u32,        // TPS (Ticks Per Second)
    tick_duration: Duration, // 1000ms / TPS

    // 状态
    world_state: WorldState,
    input_buffer: BTreeMap<u64, Vec<PlayerInput>>, // Tick -> Inputs

    // 分支支持
    parent_timeline: Option<TimelineId>,
    branch_point: Option<u64>,
}

impl TickEngine {
    pub fn run(&mut self) {
        let mut last_tick = Instant::now();

        loop {
            let now = Instant::now();
            let elapsed = now - last_tick;

            if elapsed >= self.tick_duration {
                // 执行 Tick
                self.step();
                last_tick = now;
            } else {
                // 空闲时间：处理网络、IO
                self.process_network();
                self.process_io();

                // 精确睡眠
                sleep(self.tick_duration - elapsed);
            }
        }
    }

    fn step(&mut self) {
        let tick = self.current_tick;

        // 1. 收集输入（本 Tick 到达的所有输入）
        let inputs = self.input_buffer.remove(&tick).unwrap_or_default();

        // 2. 执行 ECS Systems（确定性顺序）
        self.world_state.ecs_world.run_systems(|systems| {
            systems.input_processing(inputs);
            systems.ecology_tick();
            systems.physics_tick();
            systems.ai_tick();
            systems.combat_tick();
            systems.economy_tick();
            systems.building_tick();
            systems.diplomacy_tick();
        });

        // 3. 生成状态快照（用于回滚/分支）
        if tick % SNAPSHOT_INTERVAL == 0 {
            self.save_snapshot(tick);
        }

        // 4. 广播状态更新
        self.broadcast_delta(tick);

        // 5. 记录审计日志
        self.audit_log.tick(tick);

        self.current_tick += 1;
    }
}
```

### 2.4 状态快照与回滚

```rust
pub struct SnapshotManager {
    // 快照存储（环形缓冲区）
    snapshots: RingBuffer<WorldSnapshot>,
    max_snapshots: usize,  // 保留最近 N 个快照

    // 增量日志（用于精确回滚到任意 Tick）
    delta_log: BTreeMap<u64, Vec<StateChange>>,
}

impl SnapshotManager {
    /// 创建快照（Copy-on-Write）
    pub fn create_snapshot(&mut self, tick: u64, world: &World) {
        let snapshot = WorldSnapshot {
            tick,
            // 只复制变化的部分
            changed_chunks: world.get_changed_chunks(),
            changed_entities: world.get_changed_entities(),
            resource_pool: world.resource_pool.clone(),
        };

        self.snapshots.push(snapshot);

        // 清理旧快照
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    /// 回滚到指定 Tick
    pub fn rollback_to(&self, target_tick: u64, world: &mut World) -> Result<(), RollbackError> {
        // 1. 找到最近的快照
        let snapshot = self.find_nearest_snapshot(target_tick)?;

        // 2. 恢复快照状态
        world.restore_from_snapshot(snapshot);

        // 3. 重放从快照 Tick 到目标 Tick 的所有输入
        for tick in snapshot.tick..target_tick {
            let inputs = self.get_inputs(tick);
            world.step_with_inputs(inputs);
        }

        Ok(())
    }

    /// 创建分支时间线
    pub fn create_branch(&self, branch_point: u64) -> TimelineId {
        let branch = Timeline {
            id: generate_uuid(),
            parent: Some(self.id),
            branch_point,
            base_snapshot: self.get_snapshot(branch_point).clone(),
            inputs: BTreeMap::new(),
        };

        TIMELINE_REGISTRY.insert(branch.id, branch);
        branch.id
    }
}
```

### 2.5 维度间通信

```rust
/// 维度间传送
pub struct DimensionPortal {
    from_dimension: DimensionId,
    to_dimension: DimensionId,
    position: Vec3,

    // 玩家转移协议
    transfer_protocol: TransferProtocol,
}

impl DimensionPortal {
    pub async fn transfer_player(
        &self,
        player: PlayerId,
        from_server: &mut DimensionServer,
        to_server: &mut DimensionServer,
    ) -> Result<(), TransferError> {
        // 1. 保存玩家状态（序列化）
        let player_state = from_server.save_player_state(player)?;

        // 2. 从源维度移除
        from_server.remove_player(player);

        // 3. 发送到目标维度（通过维度路由器）
        let transfer_packet = TransferPacket {
            player_id: player,
            state: player_state,
            arrival_position: self.position,
        };

        to_server.receive_player(transfer_packet).await?;

        // 4. 通知客户端切换连接
        Ok(())
    }
}
```

---

## 三、客户端架构

### 3.1 客户端类型

| 类型 | 用途 | 渲染 | 逻辑 |
|------|------|------|------|
| **游戏客户端** | 正常游玩 | 完整 3D | 预测 + 插值 |
| **观战客户端** | 观看比赛 | 完整 3D | 只接收，不发送 |
| **管理客户端** | GM 管理 | 2D 面板 | 管理指令 |
| **回放客户端** | 查看历史 | 完整 3D | 本地回放 |

### 3.2 客户端进程结构

```
wanguo-client
│
├─ Main Thread
│   ├─ Game Loop (游戏循环)
│   ├─ State Management (状态管理)
│   └─ UI System (UI 系统)
│
├─ Render Thread (渲染线程)
│   ├─ Scene Rendering (场景渲染)
│   ├─ UI Rendering (UI 渲染)
│   └─ Post Processing (后处理)
│
├─ Network Thread (网络线程)
│   ├─ Connection Manager (连接管理)
│   ├─ Input Sender (输入发送)
│   ├─ State Receiver (状态接收)
│   └─ Delta Decoder (差分解码)
│
├─ Prediction Thread (预测线程)
│   ├─ Local Simulation (本地模拟)
│   ├─ Snapshot Buffer (快照缓冲)
│   └─ Rollback Handler (回滚处理)
│
└─ Asset Thread (资源线程)
    ├─ Model Loading (模型加载)
    ├─ Texture Streaming (纹理流送)
    └─ Audio Loading (音频加载)
```

### 3.3 客户端预测与回滚

```rust
pub struct ClientPrediction {
    // 本地玩家状态
    local_player: Entity,

    // 输入历史（用于回滚后重放）
    input_history: VecDeque<(u64, PlayerInput)>,

    // 服务端快照缓冲
    server_snapshots: BTreeMap<u64, ServerSnapshot>,

    // 当前预测状态
    predicted_state: WorldState,

    // 显示状态（插值后的平滑状态）
    display_state: WorldState,
}

impl ClientPrediction {
    /// 发送输入并立即本地预测
    pub fn send_input(&mut self, input: PlayerInput) {
        let tick = self.get_prediction_tick();

        // 1. 保存输入
        self.input_history.push_back((tick, input.clone()));

        // 2. 发送到服务端
        self.network.send(PlayerInputPacket {
            tick,
            input,
        });

        // 3. 立即本地执行（预测）
        self.predicted_state.apply_input(input);
    }

    /// 收到服务端快照
    pub fn on_server_snapshot(&mut self, snapshot: ServerSnapshot) {
        let snapshot_tick = snapshot.tick;

        // 1. 保存快照
        self.server_snapshots.insert(snapshot_tick, snapshot);

        // 2. 检查预测是否正确
        if let Some(predicted) = self.get_predicted_state_at(snapshot_tick) {
            if !predicted.matches(&snapshot) {
                // 预测错误，需要回滚
                self.rollback_and_replay(snapshot_tick);
            }
        }
    }

    /// 回滚到服务端状态并重放
    fn rollback_and_replay(&mut self, snapshot_tick: u64) {
        // 1. 回滚到服务端快照
        self.predicted_state = self.server_snapshots[&snapshot_tick].clone();

        // 2. 重放所有后续输入
        for (tick, input) in &self.input_history {
            if *tick > snapshot_tick {
                self.predicted_state.apply_input(input.clone());
            }
        }
    }

    /// 每帧插值显示状态
    pub fn interpolate_display(&mut self, render_time: f32) {
        // 在最近的两个服务端快照之间插值
        let (prev, next) = self.get_surrounding_snapshots(render_time);

        let t = (render_time - prev.time) / (next.time - prev.time);
        self.display_state = prev.lerp(&next, t);
    }
}
```

### 3.4 时间倒带功能（客户端）

```rust
pub struct TimeRewind {
    // 本地回放缓冲区
    replay_buffer: RingBuffer<WorldFrame>,

    // 当前回放状态
    is_replaying: bool,
    replay_speed: f32,  // 1.0 = 正常, -1.0 = 倒放, 0.5 = 慢放

    // 回放目标
    target_tick: Option<u64>,
}

impl TimeRewind {
    /// 进入观察者倒带模式
    pub fn enter_rewind_mode(&mut self) {
        self.is_replaying = true;
        self.replay_speed = -2.0; // 2倍速倒放
    }

    /// 暂停在特定时刻
    pub fn pause_at(&mut self, tick: u64) {
        self.target_tick = Some(tick);
        self.replay_speed = 0.0;
    }

    /// 单步前进/后退
    pub fn step(&mut self, direction: i64) {
        if let Some(current) = self.target_tick {
            self.target_tick = Some((current as i64 + direction) as u64);
        }
    }

    /// 退出倒带，回到实时
    pub fn exit_rewind(&mut self) {
        self.is_replaying = false;
        self.replay_speed = 1.0;
        self.target_tick = None;
    }
}
```

---

## 四、同步协议

### 4.1 消息类型

```rust
/// 客户端 → 服务端
pub enum ClientMessage {
    // 玩家输入（不可靠，可丢包）
    Input {
        tick: u64,
        actions: Vec<PlayerAction>,
        sequence: u32,  // 序列号，用于丢包检测
    },

    // 请求（可靠）
    Request {
        request_id: u32,
        payload: RequestPayload,
    },
}

/// 服务端 → 客户端
pub enum ServerMessage {
    // 状态快照（不可靠，定期发送）
    Snapshot {
        tick: u64,
        delta: WorldDelta,  // 自上次快照的变化
        checksum: u64,     // 校验和
    },

    // 事件通知（可靠）
    Event {
        tick: u64,
        event: GameEvent,
    },

    // 确认（可靠）
    Ack {
        client_tick: u64,
        server_tick: u64,
    },

    // 强制校正（可靠，预测错误时）
    Correction {
        tick: u64,
        authoritative_state: EntityState,
    },
}
```

### 4.2 同步策略

| 数据类型 | 同步方式 | 频率 | 可靠性 |
|---------|---------|------|--------|
| 玩家输入 | 客户端发送 | 每 Tick | 不可靠 + 冗余 |
| 玩家状态 | 服务端广播 | 每 Tick | 不可靠 |
| 方块变化 | 服务端广播 | 即时 | 可靠 |
| 实体创建/销毁 | 服务端广播 | 即时 | 可靠 |
| 聊天消息 | 服务端广播 | 即时 | 可靠 |
| 全局事件 | 服务端广播 | 即时 | 可靠 |
| 区块数据 | 按需发送 | 视距变化 | 可靠 |

### 4.3 差分压缩

```rust
pub struct DeltaCompressor {
    last_sent_state: HashMap<EntityId, EntityState>,
}

impl DeltaCompressor {
    pub fn compress(&mut self, current: &WorldState) -> WorldDelta {
        let mut delta = WorldDelta::new();

        for (entity, state) in current.entities.iter() {
            match self.last_sent_state.get(entity) {
                None => {
                    // 新实体，发送完整状态
                    delta.additions.push(state.clone());
                }
                Some(last) if last != state => {
                    // 状态变化，发送差异
                    delta.updates.push(state.diff(last));
                }
                _ => {
                    // 无变化，不发送
                }
            }
        }

        // 检测删除
        for entity in self.last_sent_state.keys() {
            if !current.entities.contains_key(entity) {
                delta.deletions.push(*entity);
            }
        }

        // 更新基准
        self.last_sent_state = current.entities.clone();

        delta
    }
}
```

---

## 五、连接与断线重连

### 5.1 连接流程

```
客户端                          服务端
  │                              │
  │── Connect Request ──────────>│
  │                              │
  │<── World Seed + Snapshot ───│
  │                              │
  │── Ack + Client Capabilities ─>│
  │                              │
  │<── Chunk Data (视距内) ─────│
  │                              │
  │── Ready ───────────────────>│
  │                              │
  │<── Start Receiving Ticks ────│
  │                              │
```

### 5.2 断线重连

```rust
pub struct ReconnectionManager {
    // 断线玩家状态保留
    disconnected_players: HashMap<PlayerId, DisconnectedPlayer>,
    retention_duration: Duration, // 保留时间（如 5 分钟）
}

impl ReconnectionManager {
    pub fn on_disconnect(&mut self, player: PlayerId, state: PlayerState) {
        self.disconnected_players.insert(player, DisconnectedPlayer {
            state,
            disconnect_time: Instant::now(),
            // 转换为 AI 控制
            ai_controlled: true,
        });
    }

    pub fn on_reconnect(&mut self, player: PlayerId) -> Option<PlayerState> {
        if let Some(disconnected) = self.disconnected_players.remove(&player) {
            // 检查是否超时
            if disconnected.disconnect_time.elapsed() < self.retention_duration {
                // 移除 AI 控制，恢复玩家控制
                return Some(disconnected.state);
            }
        }
        None // 超时或不存在，需要重新创建
    }
}
```

---

## 六、性能优化

### 6.1 服务端优化

| 技术 | 用途 | 实现 |
|------|------|------|
| **Spatial Hashing** | 快速邻近查询 | 实体按空间分区存储 |
| **Chunk Sleeping** | 无玩家区域休眠 | 简化模拟，只保留关键状态 |
| **ECS Archetype** | 缓存友好 | 相同组件组合连续存储 |
| **SIMD** | 批量计算 | 物理、AI 向量化 |
| **Job System** | 并行执行 | Bevy 内置 |
| **Delta Compression** | 减少带宽 | 只发送变化 |
| **Interest Management** | 减少广播 | 只发送玩家关心的区域 |

### 6.2 客户端优化

| 技术 | 用途 | 实现 |
|------|------|------|
| **LOD Meshing** | 远距离简化 | Greedy Meshing + 合并 |
| **Occlusion Culling** | 剔除不可见 | GPU 查询 |
| **Texture Streaming** | 内存管理 | 按需加载 |
| **Prediction** | 降低延迟感 | 本地模拟 |
| **Interpolation** | 平滑显示 | 快照插值 |
| **Chunk Caching** | 减少加载 | LRU 缓存 |

---

## 七、部署架构

### 7.1 单机部署（开发/测试）

```
┌─────────────────────────────┐
│         本地机器              │
│  ┌─────────────────────┐   │
│  │  服务端 + 客户端      │   │
│  │  (同一进程或分离)     │   │
│  └─────────────────────┘   │
└─────────────────────────────┘
```

### 7.2 专用服务器部署

```
┌─────────────────────────────────────────────┐
│              云服务器集群                      │
│                                             │
│  ┌─────────────┐    ┌─────────────────────┐ │
│  │  网关服务器  │    │   游戏服务器集群      │ │
│  │  (Nginx/    │───>│  ┌─────┐ ┌─────┐   │ │
│  │   HAProxy)  │    │  │ S1  │ │ S2  │   │ │
│  └─────────────┘    │  │维度A│ │维度B│   │ │
│                     │  └─────┘ └─────┘   │ │
│  ┌─────────────┐    │  ┌─────┐ ┌─────┐   │ │
│  │  数据库      │<───│  │ S3  │ │ S4  │   │ │
│  │  (PostgreSQL)│    │  │维度C│ │维度D│   │ │
│  └─────────────┘    │  └─────┘ └─────┘   │ │
│                     └─────────────────────┘ │
│  ┌─────────────┐                            │
│  │  对象存储    │    ┌─────────────────────┐ │
│  │  (S3/MinIO) │<───│   监控与日志          │ │
│  └─────────────┘    │  (Prometheus/Grafana) │ │
│                     └─────────────────────┘ │
└─────────────────────────────────────────────┘
```

### 7.3 容器化部署

```yaml
# docker-compose.yml
version: '3.8'

services:
  dimension-router:
    image: wanguo/router:latest
    ports:
      - "7777:7777"
    environment:
      - ROUTER_CONFIG=/config/router.toml

  overworld-server:
    image: wanguo/server:latest
    environment:
      - DIMENSION=overworld
      - TICK_RATE=20
      - MAX_PLAYERS=100
    volumes:
      - overworld-data:/data
    depends_on:
      - postgres
      - redis

  aether-server:
    image: wanguo/server:latest
    environment:
      - DIMENSION=aether
      - TICK_RATE=10
      - MAX_PLAYERS=50
    volumes:
      - aether-data:/data

  postgres:
    image: postgres:15
    environment:
      - POSTGRES_DB=wanguo
      - POSTGRES_USER=wanguo
      - POSTGRES_PASSWORD=secret
    volumes:
      - postgres-data:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    volumes:
      - redis-data:/data

volumes:
  overworld-data:
  aether-data:
  postgres-data:
  redis-data:
```

---

## 八、监控与运维

### 8.1 关键指标

| 指标 | 类型 | 告警阈值 |
|------|------|---------|
| TPS (Tick Per Second) | Gauge | < 18 告警 |
| Tick Time (ms) | Histogram | > 60ms 告警 |
| Player Count | Gauge | - |
| Network Latency (ms) | Histogram | > 100ms 告警 |
| Packet Loss (%) | Gauge | > 1% 告警 |
| Memory Usage (MB) | Gauge | > 80% 告警 |
| ECS Entity Count | Gauge | > 100万 告警 |
| Snapshot Size (MB) | Histogram | > 10MB 告警 |

### 8.2 日志结构

```json
{
  "timestamp": "2026-06-06T17:01:00Z",
  "level": "INFO",
  "tick": 12345678,
  "dimension": "overworld",
  "event_type": "PlayerAction",
  "player_id": "uuid",
  "action": "PlaceBlock",
  "position": [100, 64, -200],
  "block_id": 42,
  "resource_delta": {
    "wood": -1
  }
}
```
