# lightyear 0.26 真实 API（静态读源码 — task-2 复用参考）

Source: `D:\cargo\registry\src\rsproxy.cn-e3de039b2554c837\` (Windows
machine, NOT the `C:\Users\98185\.cargo\...` registry). Versions in
`Cargo.lock`: `lightyear = 0.26.4`, `lightyear_inputs_leafwing =
0.26.4`, `leafwing-input-manager = 0.20.0`, `bevy = 0.18.1`.

---

## 5 行速查 (核心 API + 路径)

```text
ClientPlugins.tick_duration: Duration            // lightyear-0.26.4/src/client.rs:16
ServerPlugins.tick_duration: Duration            // lightyear-0.26.4/src/server.rs:36
app.register_message::<M>().add_direction(D)    // lightyear_messages-0.26.4/src/registry.rs:344
  where M: Message+Serialize+DeserializeOwned,   // Message=blanket impl Send+Sync+'static
        D = NetworkDirection::ServerToClient | ClientToServer | Bidirectional
app.register_component::<C>()                   // lightyear_replication-0.26.4/src/registry/registry.rs:404
  where C: Component<Mutability:GetWriteFns<C>>+Serialize+DeserializeOwned
        GetWriteFns<C> (replication.rs:253): impl<C:Component<Mutability=Self>+PartialEq> GetWriteFns<C> for Mutable {}
app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<A>::default())
lightyear::prelude::PeerId                       // lightyear_core-0.26.4/src/lib.rs:47
```

---

## 关键路径全集 (rust import strings)

| Symbol                                     | Path                                                                |
| ------------------------------------------ | ------------------------------------------------------------------- |
| `ClientPlugins`                            | `lightyear::client::ClientPlugins`                                  |
| `ServerPlugins`                            | `lightyear::server::ServerPlugins`                                  |
| `NetworkDirection`                         | `lightyear::prelude::NetworkDirection` (or `lightyear_connection::direction::NetworkDirection`) |
| `PeerId`                                   | `lightyear::prelude::PeerId`                                        |
| `Message` (lightyear, blanket impl)        | `lightyear::prelude::Message` — auto for any `Send+Sync+'static`   |
| `Message` (bevy event derive)              | `bevy::prelude::Message` derive macro (orthogonal)                  |
| `AppMessageExt::register_message`          | `lightyear::prelude::*` (re-exported) or `lightyear_messages::prelude::*` |
| `AppComponentExt::register_component`      | `lightyear::prelude::*` (re-exported) or `lightyear_replication::prelude::*` |
| `MessageRegistration`                      | `lightyear_messages::registry::MessageRegistration` (re-exported)  |
| `ComponentRegistration`                    | `lightyear_replication::registry::registry::ComponentRegistration`  |
| `PredictionRegistrationExt`                | `lightyear_prediction::prelude::PredictionRegistrationExt`          |
| `InterpolationRegistrationExt`             | `lightyear_interpolation::prelude::InterpolationRegistrationExt`    |
| `InputPlugin<A>`                           | `lightyear_inputs_leafwing::prelude::InputPlugin`                   |
| `Actionlike`                               | `leafwing_input_manager::Actionlike` (derive auto-adds all bounds)  |

---

## ClientPlugins / ServerPlugins (lightyear-0.26.4/src/{client,server}.rs)

```rust
pub struct ClientPlugins { pub tick_duration: Duration }   // client.rs:16
pub struct ServerPlugins { pub tick_duration: Duration }   // server.rs:36
impl Default for { Client,Server }Plugins {
    fn default() -> Self { Self { tick_duration: Duration::from_secs_f32(1.0/60.0) } }
}
impl PluginGroup for { Client,Server }Plugins { ... }
```

Required order (lightyear-0.26.4/src/lib.rs:96): **`ClientPlugins` /
`ServerPlugins` first → then your `ProtocolPlugin` → then spawn
`Client` / `Server` entity**.

---

## `register_message` 实际签名
(`lightyear_messages-0.26.4/src/registry.rs:344-346`)

```rust
pub trait AppMessageExt {
    fn register_message<M: Message + Serialize + DeserializeOwned>(
        &mut self,
    ) -> MessageRegistration<'_, M>;
}
impl AppMessageExt for App { ... }
```

`MessageRegistration<'a, M>` (line 306-309) is an **owned** struct
holding `&'a mut App`. Methods (lines 311-338):

```rust
impl<'a, M: Message> MessageRegistration<'a, M> {
    pub fn add_map_entities(&mut self) -> &mut Self
        where M: Clone + MapEntities + 'static;
    pub fn add_direction(&mut self, direction: NetworkDirection) -> &mut Self;
}
```

`add_direction(&mut self, ...)` — takes `&mut`, returns `&mut Self`. So
chaining `app.register_message::<M>().add_direction(D)` works (the
returned `MessageRegistration` lives in a temporary).

---

## `register_component` 实际签名
(`lightyear_replication-0.26.4/src/registry/registry.rs:401-414`)

```rust
pub trait AppComponentExt {
    fn register_component<
        C: Component<Mutability: GetWriteFns<C>> + Serialize + DeserializeOwned,
    >(&mut self) -> ComponentRegistration<'_, C>;

    fn register_component_custom_serde<
        C: Component<Mutability: GetWriteFns<C>>,
    >(&mut self, serialize_fns: SerializeFns<C>) -> ComponentRegistration<'_, C>;

    fn non_networked_component<C: Component<Mutability: GetWriteFns<C>>>(
        &mut self,
    ) -> ComponentRegistration<'_, C>;
}
```

`GetWriteFns` is the bottleneck trait — defined in
`lightyear_replication-0.26.4/src/registry/replication.rs:249-253`:

```rust
pub trait GetWriteFns<C: Component> {
    fn buffer_fn() -> BufferFn<C, C>;
}
impl<C: Component<Mutability = Self> + PartialEq> GetWriteFns<C> for Mutable {
    fn buffer_fn() -> BufferFn<C, C> { default_buffer::<C> }
}
```

→ **Auto-satisfied** by `#[derive(Component, PartialEq)]` with the
default `Mutability = Mutable` (which is what
`#[derive(Component)]` in bevy 0.18 produces — see
`bevy_ecs-0.18.1/src/component/mod.rs:519`).

`ComponentRegistration<'a, C>` (registry.rs:471-474) — methods
(lines 476-541):

```rust
impl<C> ComponentRegistration<'_, C> {
    pub fn new(app: &mut App) -> ComponentRegistration<'_, C>;
    pub fn add_map_entities(self) -> Self  // takes self (owned)!
        where C: Clone + MapEntities + 'static;
    pub fn add_component_map_entities(self) -> Self
        where C: Clone + Component + 'static;
    pub fn with_replication_config(self, config: ComponentReplicationConfig) -> Self
        where C: Component<Mutability: GetWriteFns<C>>;
    pub fn add_delta_compression<Delta>(self) -> Self
        where C: Component<Mutability=Mutable> + PartialEq + Diffable<Delta>,
              Delta: Serialize + DeserializeOwned + Message;
}
```

`with_replication_config` is the **default-config knob** that gets
called internally by `register_component` (line 457-458 of
registry.rs). It uses `ComponentReplicationConfig::default()` which is
server→client replication with no prediction / interpolation.

---

## `add_prediction` / `add_interpolation`
(`lightyear_prediction-0.26.4/src/registry.rs:310-347`)

```rust
pub trait PredictionRegistrationExt<C> {
    fn add_prediction(self) -> Self where C: SyncComponent;
    fn enable_correction(self) -> Self where C: SyncComponent;
    fn add_linear_correction_fn<D>(self) -> Self where C: SyncComponent + Diffable<D>, D: Ease + Debug + Clone + Default + Send + Sync + 'static;
    fn add_correction_fn<D>(self, correction_fn: LerpFn<D>) -> Self where C: SyncComponent + Diffable<D>, D: Ease + Debug + Clone + Default + Send + Sync + 'static;
    fn add_should_rollback(self, should_rollback: ShouldRollbackFn<C>) -> Self where C: SyncComponent;
}
impl<C> PredictionRegistrationExt<C> for ComponentRegistration<'_, C> { ... }
```

`SyncComponent` (`lightyear_prediction-0.26.4/src/lib.rs:49-50`):

```rust
pub trait SyncComponent: Component<Mutability = Mutable> + Clone + PartialEq + Debug {}
impl<T> SyncComponent for T where T: Component<Mutability = Mutable> + Clone + PartialEq + Debug {}
```

Interpolation counterpart in `lightyear_interpolation-0.26.4/src/registry.rs:77-`:

```rust
pub trait InterpolationRegistrationExt<C> {
    fn add_interpolation(self) -> Self where C: SyncComponent;
    fn add_interpolation_with(self, interpolation_fn: LerpFn<C>) -> Self where C: SyncComponent;
    fn add_linear_interpolation(self) -> Self where C: SyncComponent + Ease;
    fn add_linear_interpolation_fn<D>(self) -> Self where C: SyncComponent + Diffable<D>, D: Ease + Debug + Clone + Default + Send + Sync + 'static;
    fn add_correction_fn<D>(self, correction_fn: LerpFn<D>) -> Self where C: SyncComponent + Diffable<D>, D: Ease + Debug + Clone + Default + Send + Sync + 'static;
}
```

---

## `lightyear_inputs_leafwing::InputPlugin<A>`
(`lightyear_inputs_leafwing-0.26.4/src/lib.rs:54-57`)

```rust
pub mod prelude {
    pub use crate::input_message::{LeafwingBuffer, LeafwingSnapshot};
    pub use crate::plugin::InputPlugin;     // <-- this one
}
```

Usage (`lightyear_inputs_leafwing-0.26.4/src/lib.rs:25`):
```rust
app.add_plugins(InputPlugin::<PlayerActions>::default());
```

Equal alternative path: `lightyear::prelude::input::leafwing::InputPlugin<A>`
(re-exported in `lightyear-0.26.4/src/lib.rs:390-393`).

---

## `Actionlike` (`leafwing-input-manager-0.20.0/src/lib.rs:101-106`)

```rust
pub trait Actionlike:
    Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed + TypePath + FromReflect + 'static
{
    fn input_control_kind(&self) -> InputControlKind;
}
```

`#[derive(Actionlike)]` (line 27, re-exported from
`leafwing_input_manager_macros`) auto-adds **all** of these bounds, so
the user derive list **only** needs `Actionlike` (the rest of the
derives in our enum are for bevy / serde / `bevy::prelude::Message` /
`PartialEq` matching).

---

## bevy 0.18 component derive (re: `storage = "SparseSet"`)
(`bevy_ecs_macros-0.18.1/src/component.rs:539-573`)

```rust
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

for attr in ast.attrs.iter() {
    if attr.path().is_ident(COMPONENT) {
        attr.parse_nested_meta(|nested| {
            if nested.path.is_ident(STORAGE) {
                attrs.storage = match nested.value()?.parse::<LitStr>()?.value() {
                    s if s == TABLE => StorageTy::Table,
                    s if s == SPARSE_SET => StorageTy::SparseSet,
                    ...
                }
            }
        })
    }
}
```

→ **`#[component(storage = "SparseSet")]` IS the bevy 0.18 syntax**.
The string is matched literally. Our `Health` already uses this
correctly.

---

## What the current `crates/core/src/protocol.rs` gets right vs wrong

| API call                                                       | Status     |
| -------------------------------------------------------------- | ---------- |
| `app.register_message::<messages::AttackInput>().add_direction(NetworkDirection::ClientToServer)` | ✓ correct  |
| `app.register_message::<messages::HitConfirm>().add_direction(NetworkDirection::ServerToClient)`   | ✓ correct  |
| `app.register_component::<components::Health>()`               | ✓ correct  |
| `app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<PlayerAction>::default())` | ✓ correct (direct crate path; the `lightyear::prelude::input::leafwing::InputPlugin` re-export is also valid) |
| `lightyear::prelude::PeerId` in message structs                | ✓ correct  |
| `bevy::prelude::Message` derive on message structs             | ✓ correct (orthogonal to lightyear's blanket-impl `Message`) |
| `#[component(storage = "SparseSet")]` on `Health`              | ✓ correct (bevy 0.18 syntax) |
| Doc comment (top of file) — old version with `protocol!` macro + `add_prediction()` chained example | ✗ outdated → already fixed in this commit |

**Conclusion**: the **runtime API calls** in `ProtocolPlugin::build()`
are already type-correct against lightyear 0.26.4. The doc comment at
the top of the file was misleading (claimed `add_prediction()` /
`add_linear_interpolation()` were used) — that was the only real
**code-level fix** applied in this task.
