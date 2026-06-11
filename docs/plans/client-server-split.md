# 客户端 / 服务端 拆分 (MinimalPlugins vs DefaultPlugins) 计划

> 日期: 2026-06-07
> 状态: 草稿，等用户拍板后进 `.harness/reins/developer` 执行
> 范围: 仓库从「单 binary 单 crate」拆成「workspace 3 crate」 — `core` (sim 逻辑) / `server` (headless MinimalPlugins) / `client` (DefaultPlugins)
> 这次拆分**完全替代**前几轮讨论的 fixed-tick + 插值方案 — 拆完之后那些问题自动有解

---

## 1. 背景 & 用户诉求

用户原话: "我要有两个东西，一个是 MinimalPlugins 搞的无头服务器，一个是 DefaultPlugins 的客服端。搞定他。这两个要在一个仓库里，但是分两套构建。"

**关键发现**（盘点现状后）:
- `Cargo.toml` 已经是 bevy 0.18.1 + **lightyear 0.26** + leafwing-input-manager 0.20 + avian3d 0.6.1
- `pvp::systems_server.rs` 和 `pvp::systems_client.rs` **已经按 client/server 拆过**（雏形）
- `network::protocol.rs` 已经写了 message / component 定义 — 但**没有真的接 lightyear 的 protocol! 宏**（注释里说 "lightyear 0.22 移除了 X，定义本地 newtype 即可" — 是占位）
- `controller/components.rs` 注释: "所有状态都是 ECS 组件，天然支持 lightyear 网络同步"
- **整个项目是单 binary 单 crate** — 还没真的拆构建
- 也就是说: **意图早就有了, 工具链没跟上**。这次就是收尾。

**前几轮问题的解法**:
| 之前的痛点 | 拆完之后的解 |
|---|---|
| tick 跑在 `Update` + wall-time 检查 | 服务端跑 `MinimalPlugins`, lightyear 自带 `Time<Fixed>` |
| TPS 写死 1.0 | server runtime 改 `Time::<Fixed>::set_ticks_per_second()` |
| 渲染 vsync 锁 60 | client 跑 DefaultPlugins + `PresentMode::AutoNoVsync` |
| 缺插值系统 | client 收到 lightyear 复制的 `Position` 时, 用 `Time<Fixed>::alpha()` 自动插值 |
| 渲染和逻辑耦合 | server 不带渲染, 物理隔离 |

---

## 2. 推荐方案: Cargo Workspace (3 crate)

```
F:\rustProject\lastkingdom2\
├── Cargo.toml                  ← 改成 workspace 根
├── crates/
│   ├── core/                   ← lk2-core   (lib)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs          ← re-export: world/nation/monster/ai/resource/
│   │       │                          constant/scenario/creature/pvp/components
│   │       └── protocol.rs     ← lightyear protocol! 宏 (从 src/network/protocol.rs 迁)
│   ├── server/                 ← lk2-server (bin)
│   │   ├── Cargo.toml          ← MinimalPlugins + lightyear ServerPlugins + lk2-core
│   │   └── src/main.rs
│   └── client/                 ← lk2-client (bin)
│       ├── Cargo.toml          ← DefaultPlugins + lightyear ClientPlugins + lk2-core
│       └── src/main.rs
└── src/                        ← 删掉, 全部迁到 crates/
```

### 2.1 为什么是 3 crate, 不是 2

| 方案 | 评估 |
|---|---|
| **2 crate**: `core+client` 合一个, `server` 一个 | ❌ server 依赖 client 才拿得到 sim, 依赖反了 |
| **2 crate**: `core+server` 合一个, `client` 一个 | ❌ client 依赖 server 才拿得到 sim, 依赖反了 |
| **3 crate**: `core` / `server` / `client` | ✅ 依赖方向: server → core, client → core, 谁也不依赖谁 |

`core` 必须**独立**, 原因:
- 以后写"纯 sim 单元测试"不用拉 bevy 渲染 (虽然会拉 bevy_ecs, 至少不拉 winit/wgpu)
- 协议 (protocol.rs) 必须 share, 必须住在 core
- 两个 binary 都不能依赖对方, 所以必须有一个共同祖先 = core

### 2.2 为什么不用单 crate + features

- **DefaultPlugins 和 MinimalPlugins 不能共存于同一个 `bevy = "0.18"` 实例** — MinimalPlugins 必须在 `default-features = false` 才有, 但这样 DefaultPlugins 也被关了
- 即使走 `#[cfg(feature = "client")]`, bvy 自己 feature 的开关很琐碎, cfg 容易漏
- 单 crate 编译时间比 workspace 慢 (workspace 是 3 个 crate 并行编译)
- 用户的描述 ("**分两套构建**") 本身就是 workspace 语义

---

## 3. 模块去向表 (每个文件去哪)

| 现有文件 | 去向 | 理由 |
|---|---|---|
| `src/main.rs` | **拆**: client 入口走 `crates/client/src/main.rs`, server 入口走 `crates/server/src/main.rs` | 两个 binary |
| `src/constant/mod.rs` | `crates/core/src/constant/` | 纯数据, 全共享 |
| `src/world/mod.rs` | `crates/core/src/world/` | sim 逻辑 |
| `src/world/terrain/*` | `crates/core/src/world/terrain/` | sim 逻辑 |
| `src/resource/mod.rs` | `crates/core/src/resource/` | sim 逻辑 |
| `src/nation/mod.rs` | `crates/core/src/nation/` | sim 逻辑 |
| `src/monster/mod.rs` | `crates/core/src/monster/` | sim 逻辑 |
| `src/ai/mod.rs` | `crates/core/src/ai/` | sim 逻辑 (TickObserver) |
| `src/scenario/mod.rs` | `crates/core/src/scenario/` | sim 逻辑 |
| `src/creature/mod.rs` | `crates/core/src/creature/` | sim 逻辑 |
| `src/pvp/components.rs` | `crates/core/src/pvp/` | 数据结构共享 |
| `src/pvp/systems_server.rs` | `crates/server/src/pvp_systems.rs` | 只 server 跑 |
| `src/pvp/systems_client.rs` | `crates/client/src/pvp_systems.rs` | 只 client 跑 |
| `src/pvp/los.rs` | `crates/core/src/pvp/los.rs` (or client only, 看实现) | LOS 通常 client 跑, 暂放 core 共享 |
| `src/pvp/mod.rs` | 拆, 见上 | |
| `src/controller/components.rs` | `crates/core/src/controller/` (component) + `crates/client/src/controller/` (输入采集) | 输入只 client 有, 但 component 必须 share |
| `src/controller/systems.rs` | `crates/core/src/controller/systems.rs` | 移动逻辑 sim 侧 |
| `src/network/protocol.rs` | `crates/core/src/protocol.rs` (用 lightyear macro 重写) | 必须 share |
| `src/render/mod.rs` | `crates/client/src/render/` | **只 client** |
| `src/pretty/mod.rs` | `crates/client/src/pretty/` | **只 client** |
| `src/utils/*` | 看具体内容, 默认 core | 看一眼再定 |
| `assets/` | 保留在仓库根, `client/Cargo.toml` 里 asset 处理 | server 不用 assets |
| `scenarios/` | core (server 加载) | scenario 状态机在 server |
| `screenshots/`, `loop.ps1` | 保留, 改 `loop.ps1` 默认跑 `client` (单机会话) | 闭环脚本 |

---

## 4. server (MinimalPlugins) 长这样

```rust
// crates/server/src/main.rs
use bevy::MinimalPlugins;
use bevy::prelude::*;
use lightyear::prelude::*;
use lk2_core::protocol::MyProtocol;

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ServerPlugins {
        tick_duration: Duration::from_secs_f64(1.0 / 20.0),  // 20 TPS
    });
    app.add_plugins(MyProtocol);

    // sim 系统 — 全部跑在 FixedUpdate
    app.add_systems(FixedUpdate, (
        lk2_core::simulation_tick,
        lk2_core::monster_ai,
        lk2_core::resource_regen,
        lk2_core::scenario_step,
        // ... server-authoritative 的所有逻辑
    ));

    // 启动 UDP / WebTransport transport, listen :5000
    app.add_plugins(TransportPlugin::new(/*...*/));

    app.run();
}
```

**server 完全没有**:
- Window / Winit / WindowPlugin
- RenderPlugin / WgpuPlugin
- AssetPlugin (除非用 bevy_asset 处理 scenario.json, 暂用 std::fs)
- InputPlugin (键盘鼠标)
- AudioPlugin
- UiPlugin
- 任何 `src/render` / `src/pretty` 的代码

**server 自检** (从 `main.rs::self_check` 迁过来): 启动后跑 100 headless tick, invariants 全过, 退出码 0/1。

---

## 5. client (DefaultPlugins) 长这样

```rust
// crates/client/src/main.rs
use bevy::prelude::*;
use bevy::window::PresentMode;
use lightyear::prelude::*;
use lk2_core::protocol::MyProtocol;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "万国起源 — Client".into(),
            present_mode: PresentMode::AutoNoVsync,   // ← 渲染"无上限"
            ..default()
        }),
        ..default()
    }));
    app.add_plugins(ClientPlugins { /* ... */ });
    app.add_plugins(MyProtocol);

    // 渲染 + 输入 + UI + 插值
    app.add_systems(Startup, (client_setup_camera, client_setup_hud, client_connect_server));
    app.add_systems(Update, (
        client_collect_input,           // 写 PlayerAction 到本地 ActionState
        client_send_input_to_server,    // leafwing ActionState → lightyear 消息
        client_apply_predicted_position,
        client_interpolate_remote_actors, // ← 用 Time<Fixed>::alpha() 插值
        client_render_terrain,
        client_hud_update,
    ));
    // 物理 (avian3d) — 只 client 跑可视化插值, server 是确定性 step
    app.add_plugins(PhysicsPlugins::default());

    app.run();
}
```

**client 不跑 sim 逻辑**: 玩家的 predicted position 是从 input 算的, 其他玩家 / 怪物是 lightyear 复制过来后插值。`simulation_tick` 是在 server 跑的。

**特殊情况 — 单机模式**: 保留 `cargo run -p client -- --offline` 选项, 此时 client 启一个**进程内**的 server (用 `MinimalPlugins` 加在同一个 App, 但不打开窗口 transport)。这样 demo / 截图 / 自动化测试不用开网络。这是 `loop.ps1` 默认走的路径。

---

## 6. protocol.rs 重写 (用 lightyear 真宏)

现在的 `protocol.rs` 是手写 `derive(Message)` / `derive(Component)`, 这是占位。要改成 lightyear 的 protocol! 宏:

```rust
// crates/core/src/protocol.rs
use lightyear::prelude::*;

protocol! {
    // 输入
    #[input]
    PlayerActions(actions: PlayerAction),

    // 消息
    AttackInput { tick: u32, dir: Vec3, combo: u8 } => Reliable,
    HitConfirm { victim: PeerId, dmg: f32, hit_pos: Vec3 } => Unreliable,
    DamageResult { victim: PeerId, new_hp: f32 } => Reliable,
    KnockbackEvent { victim: PeerId, vel: Vec3 } => Unreliable,

    // 复制组件 (server → client)
    PlayerPos(pub Vec3),
    PlayerHealth(pub f32),
    MonsterKind(pub u8),
    // ... 等
}

pub struct MyProtocol;
impl Plugin for MyProtocol {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
    }
}
```

---

## 7. 关键决策点 (等你拍板)

> 这 5 个问题定下来我才好动手。每个我都标了**推荐**, 拍板后我直接开干。

### Q1. workspace 3 crate ✅
- **推荐**: 3 crate (core / server / client)
- 备选: 2 crate (core+client / server) — 不推荐, 依赖会反

### Q2. lightyear 集成深度
- **推荐**: **只复制状态 + 传输输入** (起步)
  - server 权威位置 / 血量, 复制到 client
  - client 收集输入, 发送到 server
  - client 拿到的远端实体用 `Interpolated` 自动插值
- 备选: **全套 prediction/rollback**（完整动作预测与回滚）— 工作量 3x, 收益对 demo 阶段不大

### Q3. transport 选型
- **推荐**: **UDP + lightyear 默认的 Netcode** (简单, 跨平台, 适合 demo)
- 备选: WebTransport (浏览器跑) / Steam Sockets (联机大厅) — 暂不需要

### Q4. 资源权威性
- **推荐**: **server 全权威** (位置 / 血量 / 资源 / 怪物状态 / scenario 进度)
  - client 几乎所有东西都从 server 复制
  - client 只预测自己玩家的输入响应 (走 lightyear Predicted)
- 备选: 部分 client 权威 (自己挖方块先 client 跑, server 校验) — 体素编辑响应更快, 但工作量 2x

### Q5. 单机模式
- **推荐**: **保留** (`--offline` flag, client 进程内嵌一个 MinimalPlugins server)
  - `loop.ps1` 默认 `--offline` 跑
  - 这样截图 / 自动化测试不用真的开网络
- 备选: 拆完必须开两个进程 — 不利于 demo

---

## 8. 验证 / 验收

| 验收项 | 怎么测 | 预期 |
|---|---|---|
| **构建** | `cargo build --workspace` | ✅ 通过, 编译时间 < 5 分钟增量 |
| **server 跑通** | `cargo run -p server` | listen `:5000`, 打印 "Server ready", CPU 占用 < 2% |
| **client 跑通** | `cargo run -p client -- --offline` | 弹出窗口, 看到玩家 + 地形 (单机会话) |
| **client 连 server** | 开 server, 再开 `cargo run -p client -- --connect=127.0.0.1:5000` | 看到自己的玩家在远端世界, 移动有插值 |
| **unit test** | `cargo test -p core` | 50+ 单元测试全过 (和现在一样) |
| **server self-check** | server 启动时跑 100 headless tick, invariants 全过 | 退出码 0, 日志 "✅ 100 tick 全部通过" |
| **截图** | `loop.ps1` (改后) 跑完 | screenshots/iter_NN.png 出来, 跟现在一样 |

---

## 9. 实施步骤 (拍板后给 developer 跑)

1. **建 workspace 骨架** (developer)
   - `Cargo.toml` 改成 `[workspace] members = ["crates/*"]`
   - 建 `crates/core/`, `crates/server/`, `crates/client/` 三个空 crate
   - 确认 `cargo build --workspace` 三个空 crate 都能编
2. **迁 core** (developer, 一个模块一个模块)
   - 先迁 `constant/` `resource/` `world/` (无依赖, 单纯 mv)
   - 再迁 `nation/` `monster/` `ai/` `scenario/` `creature/`
   - 再迁 `pvp/components.rs` + `controller/components.rs`
   - 每个迁完跑 `cargo test -p core` 不掉测试
3. **重写 protocol.rs** (developer)
   - 用 lightyear `protocol!` 宏替换手写 derive
   - 跑通 build, 暂时不接
4. **建 server** (developer)
   - 写 `crates/server/src/main.rs` 骨架 (MinimalPlugins + lightyear ServerPlugins)
   - 把 `pvp::systems_server` 迁过去 + 接到 FixedUpdate
   - 把 `simulation_tick` 迁过去 + 接 lightyear 的 `Time<Fixed>`
   - 启动 self_check, 跑 100 tick
5. **建 client** (developer)
   - 写 `crates/client/src/main.rs` 骨架 (DefaultPlugins + lightyear ClientPlugins)
   - 迁 `src/render/*` `src/pretty/*` `src/creature/spawn`
   - 迁 `pvp::systems_client` (插值用 `Time<Fixed>::alpha()`)
   - 迁 `controller/systems.rs` (本地预测)
6. **接 lightyear 网络** (developer)
   - server 启动 UDP transport, listen 5000
   - client 启动后连 server, 走 `--connect=` 参数
   - 测攻击 / 移动 / 怪物同步
7. **改 loop.ps1** (iter-tester)
   - 默认跑 `cargo run -p client -- --offline` (单机会话, 不动网络)
   - 截图 / state.json 跟现在一样
8. **回归** (code-reviewer)
   - PR review, 跑 `cargo build --workspace` + `cargo test --workspace` + `loop.ps1` 三件套

**预估工时**: 1.5 - 2.5 天 (developer 主体, 1 天; 测试 + 调通 1 天; review + 文档 0.5 天)

---

## 10. 风险 & 缓解

| 风险 | 缓解 |
|---|---|
| lightyear 0.26 的 macro API 跟手写 derive 行为不一致 | 先做**第 3 步**单跑 protocol 编译, 别等接完才发现 |
| 现在的 `simulation_tick` 用 wall-time 算, 迁到 FixedUpdate 节奏对不上 | server 启动 100 tick 自检, 跟原来的 100 tick 自检对比输出 |
| `crates/server` 编译时间会不会比单 crate 慢 | 不会, workspace 是并行编译, 实际更快 |
| 拆分后 `loop.ps1` 路径全变 | 第 7 步独立处理, 截图/state JSON 格式不变 (loop.ps1 内部改) |
| lightyear `Time<Fixed>` 跟 avian3d 物理步节奏不一致 | server 用 lightyear 的 Fixed 跑逻辑 + 物理预测; client 物理是 visual-only, 跟 interpolation 解耦 |
| 现有 `pvp::systems_server.rs` 是 "伪 server" (同进程跑的), 真接 lightyear 要重写 | 接受, 这次不保留兼容, 旧文件直接删 |

---

## 11. 不在本次范围 (out of scope)

- 完整的 PvP prediction / rollback
- WebTransport / Steam Sockets transport
- Save / load (之前 P3)
- 大厅匹配 / 房间系统
- 国战 / 战争迷雾同步 (那是再下一轮)

---

## 12. Next Step

**你拍 Q1~Q5 (5 个决策点)**, 然后我把这个 plan 扔给 `.harness/reins/developer` 开干。

如果想省事, **直接回 "按推荐方案走"**, 我就当 5 个 Q 都按 §7 的推荐拍板了。
