# `crates/core/src/protocol.rs` — lightyear 0.26 API drift report

Date: 2026-06-09
Source of truth (read directly, not web search):
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\lightyear-0.26.4\src\{client,server,lib,protocol}.rs`
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\lightyear_messages-0.26.4\src\{lib,registry}.rs`
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\lightyear_replication-0.26.4\src\{lib,registry\replication,registry\registry}.rs`
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\lightyear_inputs_leafwing-0.26.4\src\lib.rs`
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\leafwing-input-manager-0.20.0\src\lib.rs`
- `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\bevy_ecs_macros-0.18.1\src\component.rs`

API速查 + 完整路径表: `C:\Users\98185\.mavis\plans\fix-core-protocol-drift.api.md`

---

## API 摘要 (5 行)

```text
ClientPlugins.tick_duration: Duration                     // lightyear-0.26.4/src/client.rs:16
ServerPlugins.tick_duration: Duration                     // lightyear-0.26.4/src/server.rs:36
app.register_message::<M>().add_direction(D)             // lightyear_messages-0.26.4/src/registry.rs:344
app.register_component::<C>()                            // lightyear_replication-0.26.4/src/registry/registry.rs:404
app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<A>::default())
```

---

## protocol.rs 实际改了什么 (旧 → 新对比)

**改了 4 处注释 (顶部 doc + 3 处 inline 注释块)**：

### 改 #1 — 顶部 doc comment (lines 1-50)

**旧 (3.7 KB, 描述的是假设的旧 0.20 写法)**:
```rust
//! ## lightyear 0.26 API 说明
//!
//! TODO(lightyear 0.27+): lightyear 0.26 已经没有 `protocol!` 宏了（之前的 plan
//! §6 和 task 描述是按老版本写的）。0.26 走的是 **Plugin 风格**：
//!
//! ```ignore
//! // 旧版 (lightyear 0.20 之前):
//! protocol! {
//!     #[input]
//!     PlayerActions(actions: PlayerAction),
//!     AttackInput { tick: u32, ... } => Reliable,
//!     PlayerPos(pub Vec3),
//!     ...
//! }
//! ```
//!
//! ```ignore
//! // 0.26 新版:
//! pub struct MyProtocol;
//! impl Plugin for MyProtocol {
//!     fn build(&self, app: &mut App) {
//!         app.register_message::<AttackInput>().add_direction(NetworkDirection::ClientToServer);
//!         app.register_message::<HitConfirm>().add_direction(NetworkDirection::ServerToClient);
//!         app.register_component::<PlayerPos>().add_prediction().add_linear_interpolation();
//!         ...
//!     }
//! }
//! ```
//!
//! Message 注册需要 `Message + Serialize + DeserializeOwned + Clone + Debug + PartialEq + Reflect`。
//! Component 注册需要 `Component + Serialize + DeserializeOwned + Clone + Debug + PartialEq`。
//! 等 lightyear 出现 `protocol!` 宏后...
```

**新 (4.0 KB, 引用真实 0.26.4 源码 + 真实 trait 路径)**:
```rust
//! ## lightyear 0.26 实际 API（2026-06-09 读 `lightyear-0.26.4` 源码确认）
//!
//! lightyear 0.26 **没有** `protocol!` 宏（0.27+ 才有）。0.26 走 **Plugin 风格**...
//!
//! ### 实际 API 关键点
//!
//! - `lightyear_messages::Message` 是 **blanket impl**：
//!   `impl<T: Send + Sync + 'static> Message for T {}`。消息类型 **不需要**
//!   derive `Message`（`bevy::prelude::Message` derive 是另一回事 — 只
//!   是把类型注册为 bevy 的本地事件，跟 lightyear 互不干扰）。
//! - `app.register_message::<M>()` 来自 `AppMessageExt` trait
//!   （`lightyear_messages-0.26.4/src/registry.rs:344`），约束为
//!   `M: Message + Serialize + DeserializeOwned`，返回**owned** 的
//!   `MessageRegistration<'_, M>`，上面有 `.add_direction(NetworkDirection::*)`。
//! - `app.register_component::<C>()` 来自 `AppComponentExt` trait
//!   （`lightyear_replication-0.26.4/src/registry/registry.rs:404`），
//!   约束更严：`C: Component<Mutability: GetWriteFns<C>> + Serialize +
//!   DeserializeOwned`。`GetWriteFns<C>` 由
//!   `lightyear_replication-0.26.4/src/registry/replication.rs:253` 提供：
//!   `impl<C: Component<Mutability = Self> + PartialEq> GetWriteFns<C> for
//!   Mutable {}`，所以 `#[derive(Component, PartialEq)]` 默认 `Mutability =
//!   Mutable` 就自动满足。
//! - 可选 chain：`.add_prediction()` (`PredictionRegistrationExt`,
//!   lightyear_prediction-0.26.4/src/registry.rs:312`)、`.add_interpolation()` /
//!   `.add_linear_interpolation()` (`InterpolationRegistrationExt`)。本文件
//!   **不**调用 — 用默认 `ComponentReplicationConfig`...
//! - `lightyear::prelude::PeerId` 存在（来自
//!   `lightyear_core-0.26.4/src/lib.rs:47`）。
//! - `lightyear_inputs_leafwing::prelude::InputPlugin::<A>::default()` 是
//!   正确路径（`lightyear_inputs_leafwing-0.26.4/src/lib.rs:54-57`）；也可
//!   走 `lightyear::prelude::input::leafwing::InputPlugin::<A>`。
//! - `leafwing-input-manager` 0.20 的 `Actionlike` trait
//!   （`leafwing-input-manager-0.20.0/src/lib.rs:101-106`）需要
//!   `Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed + TypePath +
//!   FromReflect + 'static`。`#[derive(Actionlike)]` 宏会**自动**加这些，
//!   不用手写。
//!
//! 完整 drift 报告（对照 0.26 源码逐项核对）：`plans/fix-core-protocol-drift.drift.md`
```

### 改 #2 — PlayerAction 注释 (lines 60-74)

**旧**:
```rust
// leafwing-input-manager 0.20 的 `Actionlike` derive 自动给 `ActionState<A>` 用的
// trait bound (`A: Actionlike + Hash`)，还需要用户自己 derive `Hash + Eq + Reflect +
// FromReflect + Typed`（来自 bevy_reflect）。
```

**新**:
```rust
// `Actionlike` derive 宏 (`leafwing-input-manager-0.20.0/src/lib.rs:101-106`)
// 要求 trait bound `Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed
// + TypePath + FromReflect + 'static`, derive 宏会**自动**加上这些, 所以下面
// derive 列表里**只**写 `Actionlike` 即可 (其他 derive 是给 `Message` /
// serialization / `bevy::prelude` 用的, 跟 lightwing 无关).
```

### 改 #3 — Messages 注释 (lines 91-108)

**旧**:
```rust
// 注意: lightyear 0.26 仍然保留了 `lightyear::prelude::Message` trait 作为
// 标记 trait, 但不在 `derive` 列表里 — `Message` 是 blanket impl: `impl<T: Send +
// Sync + 'static> Message for T {}`. 所以消息 derive 时 **不需要** 显式 `Message`。
// 但 `app.register_message::<M>()` 在 0.26 要求 `M: Message + Serialize +
// DeserializeOwned + Clone + Debug + PartialEq`。我们 derive 全套。
```

**新**:
```rust
// lightyear 0.26 仍然保留了 `lightyear_messages::Message` trait 作为标记
// trait, 但它是 **blanket impl** (`lightyear_messages-0.26.4/src/lib.rs:61-62`):
// `pub trait Message: Send + Sync + 'static {}` + `impl<T: Send + Sync + 'static>
// Message for T {}`. 所以消息 struct derive 时 **不需要** 显式 `Message` —
// 任何 `Send + Sync + 'static` 类型都自动实现。
//
// `bevy::prelude::Message` derive (上面 derive 列表里的那个) 是 **bevy 本地
// event 系统** 的 derive 宏, 跟 lightyear 互不干扰 — 加上它只是顺手把消息
// 类型注册为 bevy 事件 (供本地 reader / observer 用), 跟 lightyear 的
// `Message` blanket impl 是两码事。
//
// `app.register_message::<M>()` 实际约束是 `M: Message + Serialize +
// DeserializeOwned` (`lightyear_messages-0.26.4/src/registry.rs:344`), 我
// 们 derive 全套 (Serialize/Deserialize/Clone/Debug/PartialEq) 满足它。
```

### 改 #4 — Components 注释 (lines 160-180)

**旧**:
```rust
// `lightyear::prelude::Component` 是 blanket impl: `impl<T: bevy::prelude::Component>
// Component for T {}`. 同样 **不需要** 显式 derive。
// `app.register_component::<C>()` 要求 `C: Component + Serialize + DeserializeOwned
// + Clone + Debug + PartialEq`。derive 全套。
// 部分组件带 `#[component(storage = "SparseSet")]` attribute — 这是 bevy 原生
// 0.18 的 attribute, 0.26 lightyear 直接透传, 不冲突。
```

**新**:
```rust
// lightyear 0.26 走的是 `AppComponentExt::register_component<C: Component<Mutability:
// GetWriteFns<C>> + Serialize + DeserializeOwned>` (`lightyear_replication-0.26.4/
// src/registry/registry.rs:404`)。这个 `GetWriteFns<C>` 由
// `lightyear_replication-0.26.4/src/registry/replication.rs:253` 提供:
// `impl<C: Component<Mutability = Self> + PartialEq> GetWriteFns<C> for Mutable {}`。
// 我们的 struct 都 derive `Component, PartialEq` 且默认 `Mutability = Mutable`
// (bevy 0.18 `#[derive(Component)]` 默认值), 自动满足。
//
// `Health` 上的 `#[component(storage = "SparseSet")]` 是 bevy 0.18 的 attribute
// 写法, lightyear 0.26 不修改这些 attribute。如果 cargo check 报 unknown
// attribute, 改成 bevy 0.18 的新写法 (一般是 `#[component(sparse_set)]` 或
// 删掉 — sparse set 已是默认)。
//
// 没在这里调 `add_prediction()` / `add_interpolation()` / `add_linear_interpolation()` —
// 默认 `ComponentReplicationConfig::default()` 已经够用 (server→client 复制, 不
// 带 prediction / interpolation)。等 task-2 真接 PvP 客户端预测时, 在
// `PlayerPos` / `PlayerRot` 上按需加。
```

### 改 #5 — ProtocolPlugin 注释 (lines 241-253)

**旧**:
```rust
// lightyear 0.26 在 add_plugins(ClientPlugins) / add_plugins(ServerPlugins) 之后才
// 调 add_plugins(ProtocolPlugin)；且必须在 spawn Client/Server entity 之前。
```

**新**:
```rust
// lightyear 0.26 在 add_plugins(ClientPlugins) / add_plugins(ServerPlugins) 之后才
// 调 add_plugins(ProtocolPlugin)；且必须在 spawn Client/Server entity 之前。
// (见 lightyear-0.26.4/src/lib.rs:96 注释)
//
// 顺序: `add_plugins(InputPlugin)` 先 (只依赖 leafwing), 再
// `register_message::<M>().add_direction(NetworkDirection::X)`, 最后
// `register_component::<C>()` — 跟 lib.rs 文档示例一致。
```

---

## 没改的 (因为已经是正确的)

- `app.register_message::<messages::AttackInput>().add_direction(NetworkDirection::ClientToServer)` — 签名匹配
  `AppMessageExt::register_message<M: Message+Serialize+DeserializeOwned>(&mut self) -> MessageRegistration<'_, M>`,
  `.add_direction(&mut self, ...) -> &mut Self` 也匹配 (`lightyear_messages-0.26.4/src/registry.rs:344,331`).
- `app.register_component::<components::Health>()` — 匹配 `AppComponentExt::register_component<C: Component<Mutability: GetWriteFns<C>>+Serialize+DeserializeOwned>` (`lightyear_replication-0.26.4/src/registry/registry.rs:404`).
  `Health` derive `Component, PartialEq` 默认 `Mutability=Mutable` → `GetWriteFns<Health>` 满足 (`replication.rs:253`).
- `app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<PlayerAction>::default())` — 路径正确
  (`lightyear_inputs_leafwing-0.26.4/src/lib.rs:54-57`).
- `lightyear::prelude::PeerId` — 通过 `lightyear_core::prelude::PeerId` 再到 `lightyear::prelude::*` (`lightyear_core-0.26.4/src/lib.rs:47`).
- `bevy::prelude::Message` derive on message structs — 跟 lightyear `Message` blanket impl 互不干扰 (bevy 0.18 derive 宏只生成 `bevy::prelude::Message` 的实现, 不影响 `lightyear_messages::Message`).
- `#[component(storage = "SparseSet")]` on `Health` — bevy 0.18 正确语法 (`bevy_ecs_macros-0.18.1/src/component.rs:539-573` 字符串字面量匹配 "SparseSet").

---

## 留给 task-2 / task-3 的项

1. **真正跑 `cargo check -p lk2-core` 验证 cold compile** — 本次任务被
   engine 砍掉 (30 min cap 装不下 15-22 min 冷编)。静态层面我们已经证明
   API 签名都对, 但**还没有 type checker 的最终确认**。建议后续 task 在
   资源充足时 (>=30 min) 跑一次 `cargo check -p lk2-core
   --message-format=short > log/check_core.log 2>&1`。
2. **真接 PvP 客户端预测时** 在 `PlayerPos` / `PlayerRot` 上加
   `.add_prediction(SyncComponent)` — `SyncComponent` blanket impl
   (`lightyear_prediction-0.26.4/src/lib.rs:49-50`) 自动满足
   (`Clone + PartialEq + Debug + Component<Mutability=Mutable>`).
3. **当前 `lightyear_inputs_leafwing` 路径在 server / client bin
   crate 里的 init 顺序** — `ProtocolPlugin.build` 里 `add_plugins(InputPlugin)` 跟
   `ClientPlugins` / `ServerPlugins` 的 feature flag 依赖 (server bin 可能
   不启 `leafwing` feature), task-2 接 client/server binary 时要按 `cfg`
   切。
4. **不要 bump `compt = ">=1.9, <1.10"`** — Cargo.toml 的 pin 是为了
   broccoli 0.6 兼容 (见 AGENTS.md)。

---

## 工程备注

- 本次**不**跑 `cargo check` (engine 明确要求砍掉, 物理上 30 min cap 装
  不下 lightyear 0.26 的冷编译, 上次已经验证了)。所有 API 结论都基于
  静态读源码 (cargo metadata 路径 + 直接 file read), 真实编译需要在
  后续 task 验证。
- 本次也不改 `crates/client` / `crates/server` (task-2 边界)。
- 本次也不改 `crates/core/src/{pvp,controller,world,nation,...}`
  (其他模块是 task-1 跑过的 50 测试通过的代码)。
