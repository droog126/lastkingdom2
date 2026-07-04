# 万国起源：最后一国 · Bevy ECS 游戏内容架构
## 基于 Bevy + Rust 的完整技术实现方案

---

## 一、核心理念：架构即玩法

《万国起源：最后一国》的设计文档定义了一套极其复杂的系统：
- **全局资源池**：有限、守恒、可追溯
- **国家系统**：政治、战争、外交、经济
- **职业系统**：40+ 战斗职业 + 8 种后勤专精
- **生态系统**：浆果丛林、怪物王国、动物迁徙
- **信息系统**：视野、斥候塔、信号干扰器
- **商队系统**：物理化后勤、护送与阻截

**传统 OOP 架构无法支撑这些系统的并发运行**。Bevy ECS 的"数据连续存储 + 系统并行执行 + 确定性模拟"恰好是这些玩法的天然载体。

**本架构的核心原则**：
1. **每一个游戏设计概念 = 一个或多个 ECS Component**
2. **每一个游戏规则 = 一个或多个 ECS System**
3. **每一个系统间交互 = 通过 Event 或 Query 完成**
4. **全局资源池 = 单一 Resource，所有系统读写通过它**

---

## 二、ECS 实体映射表：设计概念 → 代码

### 2.1 核心实体类型

| 设计概念 | ECS 实体 | 核心组件 | 说明 |
|---------|---------|---------|------|
| **玩家** | `Player` | `PlayerId`, `Position`, `Velocity`, `Health`, `NationId`, `Profession`, `ClassLoadout`, `Vision` | 同时是战斗单位和资源采集者 |
| **国家** | `Nation` | `NationId`, `NationName`, `KingPlayerId`, `MemberList`, `WarExhaustion`, `Treasury`, `Treaties` | 无位置，纯逻辑实体 |
| **国家旗帜** | `NationFlag` | `NationId`, `Position`, `Health`, `IsVulnerable` | 被摧毁 = 国家灭亡 |
| **国家核心** | `NationCore` | `NationId`, `Position`, `BuildProgress`, `UpgradeLevel` | 建筑升级中心 |
| **区块** | `Chunk` | `ChunkCoord`, `ChunkData`, `DirtyChunk`, `BiomeType` | 16×16×16 方块数据 |
| **怪物王国** | `MonsterKingdom` | `Position`, `Population`, `BiomeType`, `IsDestroyed` | 全局上限 5 个 |
| **怪物巢穴** | `MonsterNest` | `Position`, `Population`, `DormancyTimer`, `DecayCounter` | 全局上限 80 个 |
| **怪物个体** | `Monster` | `MonsterType`, `Position`, `Health`, `AiState`, `HomeKingdom` | 全局上限 1500 个 |
| **商队** | `Caravan` | `OwnerNationId`, `TraderPlayerId`, `PackAnimalEntity`, `CargoInventory`, `RoutePath` | 物理化移动单位 |
| **驮兽** | `PackAnimal` | `Position`, `Health`, `Inventory`, `MovementSpeed`, `IsBeingLed` | 商人专属 |
| **斥候塔** | `ScoutTower` | `NationId`, `Position`, `DetectionRadius`, `IsActive` | 视野节点 |
| **信号干扰器** | `SignalJammer` | `NationId`, `Position`, `JamRadius`, `Health` | 反侦察 |
| **浆果丛林** | `BerryThicket` | `Position`, `WitherCounter`, `LastBerryTick`, `IsOvergrazed` | 生态生产者 |
| **共鸣收集器** | `ResonanceCollector` | `Position`, `TargetResource`, `ProductionTimer`, `Health` | 特殊资源采集建筑 |
| **位面传送门** | `AetherPortal` | `NationId`, `Position`, `BuildProgress`, `State`, `Health` | 终局入口 |
| **以太幽魂** | `AetherWraith` | `Position`, `Health`, `TeleportCooldown`, `TargetPlayer` | 终局怪物 |
| **任务卡** | `MissionCard` | `IssuerKingId`, `TargetPosition`, `MissionType`, `ExpiryTime`, `Adopters` | 国王指挥系统 |
| **条约** | `Treaty` | `NationA`, `NationB`, `TreatyType`, `Duration`, `BreakPenalty` | 外交实体 |

### 2.2 全局资源池（单一 Resource）

```rust
// shared/src/resources/global_pool.rs
use bevy::prelude::*;

#[derive(Resource, Clone, Debug)]
pub struct GlobalResourcePool {
    pub wood: u32,              // 10,000
    pub hardened_wood: u32,     // 500
    pub apple: u32,             // 5,000
    pub wheat_seeds: u32,       // 1,000
    pub carrot: u32,            // 2,000
    pub potato: u32,            // 2,000
    pub bloodthistle_seeds: u32, // 200
    pub frostleaf_seeds: u32,    // 200
    pub food: u32,              // 20,000
    pub soul: u32,              // 1,000
    pub sunstone: u32,          // 200
    pub frostcore: u32,         // 200
    pub living_root: u32,      // 200
    pub void_essence: u32,     // 100
    pub monster_kingdom: u8,    // 上限 5
    pub monster_nest: u8,       // 上限 80
    pub monster_population: u16, // 上限 1500
    pub berry_thicket: u16,     // 上限 200
    pub berry: u16,             // 上限 4,000
    pub nation_flags_remaining: u8, // 上限 8
}

impl GlobalResourcePool {
    pub fn can_consume(&self, resource: &str, amount: u32) -> bool {
        match resource {
            "wood" => self.wood >= amount,
            "soul" => self.soul >= amount,
            // ... 所有资源
            _ => false,
        }
    }

    pub fn consume(&mut self, resource: &str, amount: u32) -> Result<(), PoolError> {
        if !self.can_consume(resource, amount) {
            return Err(PoolError::Insufficient);
        }
        match resource {
            "wood" => self.wood -= amount,
            "soul" => self.soul -= amount,
            // ...
            _ => return Err(PoolError::UnknownResource),
        }
        Ok(())
    }

    pub fn produce(&mut self, resource: &str, amount: u32) -> Result<(), PoolError> {
        match resource {
            "wood" => {
                if self.wood + amount > 10_000 {
                    return Err(PoolError::PoolFull);
                }
                self.wood += amount;
            }
            // ... 所有资源，含上限检查
            _ => return Err(PoolError::UnknownResource),
        }
        Ok(())
    }
}
```

---

## 三、核心系统架构

### 3.1 系统分类与调度

```rust
// shared/src/systems/mod.rs
use bevy::prelude::*;

pub struct GameSystemsPlugin;

impl Plugin for GameSystemsPlugin {
    fn build(&self, app: &mut App) {
        // === 固定 Tick 系统（20 TPS）===
        app.insert_resource(Time::<Fixed>::from_seconds(0.05));

        app.add_systems(FixedUpdate, (
            // 阶段 1：输入处理（客户端预测 / 服务端权威）
            input_processing_system,

            // 阶段 2：生态循环（并行）
            berry_thicket_tick_system,
            monster_kingdom_tick_system,
            monster_nest_decay_system,
            crop_growth_system,

            // 阶段 3：国家逻辑（串行，避免竞态）
            nation_management_system,
            war_exhaustion_system,
            treaty_expiry_system,

            // 阶段 4：经济与物流
            caravan_movement_system,
            supply_line_system,
            market_transaction_system,

            // 阶段 5：战斗与物理
            combat_system,
            physics_system,
            damage_application_system,

            // 阶段 6：建筑与升级
            building_construction_system,
            building_upgrade_system,

            // 阶段 7：视野与信息
            vision_update_system,
            scout_tower_detection_system,
            signal_jammer_system,

            // 阶段 8：终局检查
            victory_condition_system,
        )
        .chain()  // 显式依赖顺序
        .in_set(GameTickSet::Logic));

        // === 慢速 Tick 系统（每 60 秒）===
        app.add_systems(FixedUpdate, (
            world_ecosystem_diffusion_system,
            resource_regeneration_system,
            late_nation_relief_system,
        )
        .run_if(on_timer(Duration::from_secs(60))));

        // === 事件系统 ===
        app.add_event::<NationDestroyedEvent>();
        app.add_event::<TreatyBrokenEvent>();
        app.add_event::<CaravanAmbushedEvent>();
        app.add_event::<PlayerStageTransitionEvent>();
        app.add_event::<AetherWraithKilledEvent>();
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameTickSet {
    Input,
    Ecology,
    Nation,
    Economy,
    Combat,
    Building,
    Vision,
    Endgame,
}
```

### 3.2 关键系统详解

#### 系统 A：全局资源池守恒系统

```rust
// shared/src/systems/resource_pool.rs
use bevy::prelude::*;

/// 所有资源变动必须经过此系统，确保守恒
fn resource_transaction_system(
    mut pool: ResMut<GlobalResourcePool>,
    mut transaction_events: EventReader<ResourceTransactionEvent>,
    mut audit_log: ResMut<AuditLog>,
) {
    for event in transaction_events.read() {
        match event.transaction_type {
            TransactionType::Consume => {
                if let Err(e) = pool.consume(&event.resource, event.amount) {
                    // 事务失败，回滚或报错
                    warn!("Resource transaction failed: {:?}", e);
                    continue;
                }
            }
            TransactionType::Produce => {
                if let Err(e) = pool.produce(&event.resource, event.amount) {
                    warn!("Resource production failed: {:?}", e);
                    continue;
                }
            }
            TransactionType::Transfer { from, to } => {
                // 内部转账：从一个池子到另一个，总量不变
                if let (Ok(()), Ok(())) = (
                    pool.consume(&from, event.amount),
                    pool.produce(&to, event.amount)
                ) {
                    // 成功
                } else {
                    warn!("Transfer failed: {} -> {}", from, to);
                    continue;
                }
            }
        }

        // 审计日志
        audit_log.entries.push(AuditEntry {
            tick: event.tick,
            resource: event.resource.clone(),
            amount: event.amount,
            reason: event.reason.clone(),
            source: event.source.clone(),
        });
    }
}
```

#### 系统 B：浆果丛林生态循环

```rust
// shared/src/systems/ecology/berry_thicket.rs
use bevy::prelude::*;

/// 每 30 秒 Tick 一次
fn berry_thicket_tick_system(
    time: Res<Time>,
    mut pool: ResMut<GlobalResourcePool>,
    mut thickets: Query<&mut BerryThicket>,
    mut transaction_events: EventWriter<ResourceTransactionEvent>,
) {
    let berry_total = pool.berry;
    let max_berry = 4000u16;

    for mut thicket in thickets.iter_mut() {
        // 检查是否过饱和
        if berry_total >= max_berry {
            // 推进枯萎
            if thicket.time_since_last_consumption > 300.0 {
                thicket.wither_counter += 1;
                if thicket.wither_counter >= 10 {
                    // 丛林死亡
                    transaction_events.send(ResourceTransactionEvent {
                        resource: "berry_thicket".to_string(),
                        amount: 1,
                        transaction_type: TransactionType::Consume,
                        reason: "Thicket withered due to over-saturation".to_string(),
                        source: format!("Thicket at {:?}", thicket.position),
                        tick: 0, // 由系统填充
                    });
                    // 标记销毁
                    thicket.marked_for_destruction = true;
                }
            }
        } else {
            // 尝试结浆果
            let amount = fastrand::u32(1..=3);
            let new_total = berry_total.saturating_add(amount as u16);
            if new_total <= max_berry {
                transaction_events.send(ResourceTransactionEvent {
                    resource: "berry".to_string(),
                    amount,
                    transaction_type: TransactionType::Produce,
                    reason: "Berry production".to_string(),
                    source: format!("Thicket at {:?}", thicket.position),
                    tick: 0,
                });
                thicket.wither_counter = 0;
                thicket.time_since_last_consumption = 0.0;
            }
        }
    }
}

/// 玩家消耗浆果事件监听
fn on_berry_consumed(
    mut events: EventReader<BerryConsumedEvent>,
    mut pool: ResMut<GlobalResourcePool>,
    mut thickets: Query<&mut BerryThicket>,
) {
    for event in events.read() {
        pool.berry = pool.berry.saturating_sub(event.amount as u16);

        // 如果浆果总量低于 70%，重置所有丛林的枯萎计数
        if pool.berry < (4000.0 * 0.7) as u16 {
            for mut thicket in thickets.iter_mut() {
                thicket.wither_counter = 0;
            }
        }
    }
}
```

#### 系统 C：怪物王国生态

```rust
// shared/src/systems/ecology/monster_kingdom.rs
use bevy::prelude::*;

/// 怪物王国被摧毁后的散落逻辑
fn monster_kingdom_destruction_system(
    mut events: EventReader<NationDestroyedEvent>,  // 复用，或自定义
    mut pool: ResMut<GlobalResourcePool>,
    mut commands: Commands,
    mut transaction_events: EventWriter<ResourceTransactionEvent>,
) {
    for event in events.read() {
        // 这里应该是 MonsterKingdomDestroyedEvent
        // 简化示例

        // 减少王国计数
        transaction_events.send(ResourceTransactionEvent {
            resource: "monster_kingdom".to_string(),
            amount: 1,
            transaction_type: TransactionType::Consume,
            reason: "Kingdom destroyed".to_string(),
            source: event.nation_id.to_string(),
            tick: 0,
        });

        // 散落为 3-6 个小巢穴
        let nest_count = fastrand::u32(3..=6);
        for _ in 0..nest_count {
            if pool.monster_nest >= 80 {
                break;
            }

            let initial_pop = fastrand::u32(15..=26);

            // Spawn 新巢穴实体
            commands.spawn((
                MonsterNest {
                    position: event.position + random_offset(50.0),
                    population: initial_pop as u16,
                    dormancy_timer: 0.0,
                    decay_counter: 0,
                },
            ));

            pool.monster_nest += 1;
            pool.monster_population += initial_pop as u16;
        }
    }
}

/// 巢穴衰亡检查（每 60 秒）
fn monster_nest_decay_system(
    mut pool: ResMut<GlobalResourcePool>,
    mut nests: Query<(Entity, &mut MonsterNest)>,
    mut commands: Commands,
) {
    for (entity, mut nest) in nests.iter_mut() {
        if nest.population == 0 && nest.dormancy_timer > 300.0 {
            // 25% 几率衰亡
            if fastrand::f32() < 0.25 {
                commands.entity(entity).despawn();
                pool.monster_nest = pool.monster_nest.saturating_sub(1);
            }
        }
    }
}
```

#### 系统 D：国家系统（战争疲劳 + 条约 + 接管）

```rust
// shared/src/systems/nation/mod.rs
use bevy::prelude::*;

/// 战争疲劳系统
fn war_exhaustion_system(
    time: Res<Time>,
    mut nations: Query<&mut Nation>,
    mut events: EventReader<CombatEvent>,
) {
    for mut nation in nations.iter_mut() {
        // 自然下降：每 10 分钟 -5（如果未处于战争状态）
        if nation.last_combat_time.elapsed() > Duration::from_secs(600) {
            nation.war_exhaustion = nation.war_exhaustion.saturating_sub(5);
        }

        // 门槛效果应用
        if nation.war_exhaustion > 60 && nation.war_exhaustion <= 80 {
            nation.healing_efficiency = 0.5;
        } else if nation.war_exhaustion > 80 && nation.war_exhaustion < 100 {
            nation.can_sign_treaties = false;
        } else if nation.war_exhaustion >= 100 {
            nation.flag_vulnerable = true;
            nation.flag_damage_multiplier = 1.25;
        }
    }

    // 战斗事件增加疲劳
    for event in events.read() {
        if let Ok(mut nation) = nations.get_mut(event.victim_nation) {
            nation.war_exhaustion = (nation.war_exhaustion + 1).min(100);
        }
    }
}

/// 条约系统
fn treaty_system(
    mut treaties: Query<&mut Treaty>,
    mut nations: Query<&mut Nation>,
    mut broken_events: EventWriter<TreatyBrokenEvent>,
    time: Res<Time>,
) {
    for mut treaty in treaties.iter_mut() {
        treaty.remaining_duration -= time.delta_seconds();

        if treaty.remaining_duration <= 0.0 {
            // 条约自然到期
            treaty.is_active = false;
        }

        // 检查违约条件（如造成伤害）
        // ...
    }
}

/// 国家接管窗口系统
fn nation_takeover_system(
    mut destroyed_events: EventReader<NationDestroyedEvent>,
    mut commands: Commands,
    mut pool: ResMut<GlobalResourcePool>,
) {
    for event in destroyed_events.read() {
        // 创建接管窗口实体
        commands.spawn((
            TakeoverWindow {
                center_position: event.core_position,
                radius: 50.0,
                expiry: time.elapsed() + Duration::from_secs(600), // 10 分钟
                original_nation: event.nation_id,
            },
        ));

        // 遗址建筑标记为可打捞
        // ...
    }
}

/// 晚建国家补偿系统（每 60 秒）
fn late_nation_relief_system(
    mut nations: Query<&mut Nation>,
    match_timer: Res<MatchTimer>,
) {
    for mut nation in nations.iter_mut() {
        let order = nation.founding_order; // 1-8
        let match_minutes = match_timer.elapsed_minutes();

        // 计算时代补偿系数
        let order_discount = match order {
            1..=2 => 0.0,
            3..=4 => 0.05..0.10,
            5..=6 => 0.10..0.20,
            7..=8 => 0.20..0.30,
            _ => 0.0,
        };

        let time_discount = if match_minutes > 40 {
            0.10..0.15
        } else if match_minutes > 30 {
            0.05..0.10
        } else {
            0.0
        };

        nation.build_cost_discount = (order_discount + time_discount).min(0.5);

        // 初始福利
        if nation.age < Duration::from_secs(600) {
            nation.maintenance_cost_multiplier = 0.8; // -20%
            nation.supply_line_efficiency = 1.1; // +10%
        }
    }
}
```

#### 系统 E：商队与物流系统

```rust
// shared/src/systems/economy/caravan.rs
use bevy::prelude::*;

/// 商队移动系统
fn caravan_movement_system(
    time: Res<Time>,
    mut caravans: Query<(&mut Position, &mut Caravan, &PackAnimal)>,
    players: Query<&Position, With<Player>>,
) {
    for (mut pos, mut caravan, pack_animal) in caravans.iter_mut() {
        if !pack_animal.is_being_led {
            continue;
        }

        // 获取牵引者位置
        if let Ok(leader_pos) = players.get(caravan.trader_player_id) {
            let direction = leader_pos.0 - pos.0;
            let distance = direction.length();

            if distance > 10.0 {
                // 驮兽移动速度为玩家的 70%
                let speed = pack_animal.movement_speed * 0.7;
                let move_dir = direction.normalize();
                pos.0 += move_dir * speed * time.delta_seconds();
            }
        }
    }
}

/// 商队护送光环系统
fn escort_aura_system(
    caravans: Query<(Entity, &Position, &Caravan)>,
    players: Query<(Entity, &Position, &Player)>,
    mut damage_events: EventWriter<DamageModifierEvent>,
) {
    for (caravan_entity, caravan_pos, caravan) in caravans.iter() {
        // 检查 10 格范围内是否有护送者
        let has_escort = players.iter().any(|(_, player_pos, _)| {
            (player_pos.0 - caravan_pos.0).length() < 10.0
        });

        if has_escort {
            damage_events.send(DamageModifierEvent {
                target: caravan_entity,
                modifier: 0.8, // 20% 减伤
                duration: Duration::from_secs(1),
            });
        }
    }
}

/// 商队被击杀掉落系统
fn caravan_death_system(
    mut death_events: EventReader<EntityDeathEvent>,
    caravans: Query<&Caravan>,
    pack_animals: Query<&PackAnimal>,
    mut commands: Commands,
) {
    for event in death_events.read() {
        if let Ok(caravan) = caravans.get(event.entity) {
            // 驮兽死亡，掉落所有货物
            if let Ok(pack_animal) = pack_animals.get(caravan.pack_animal_entity) {
                for item in &pack_animal.inventory {
                    commands.spawn((
                        DroppedItem {
                            item: item.clone(),
                            position: event.position,
                            expiry: Timer::from_seconds(300.0, TimerMode::Once),
                        },
                    ));
                }
            }

            // 发送商队被伏击事件
            // ...
        }
    }
}
```

#### 系统 F：视野与信息系统

```rust
// shared/src/systems/vision/mod.rs
use bevy::prelude::*;

/// 玩家基础视野更新
fn player_vision_system(
    mut players: Query<(&Position, &mut Vision, &NationId), With<Player>>,
    obstacles: Query<&Position, With<Obstacle>>,
) {
    for (pos, mut vision, nation_id) in players.iter_mut() {
        // 基础视野半径 60-90 格
        let base_radius = 75.0;

        // 地形遮挡计算（简化：射线检测）
        vision.visible_entities.clear();

        // 视野内的所有实体
        // 实际实现需要空间哈希加速
    }
}

/// 斥候塔侦测系统
fn scout_tower_detection_system(
    towers: Query<(&Position, &ScoutTower, &NationId)>,
    targets: Query<(Entity, &Position, &NationId), Or<(With<Player>, With<Monster>)>>,
    mut detected_events: EventWriter<DetectionEvent>,
) {
    for (tower_pos, tower, tower_nation) in towers.iter() {
        if !tower.is_active {
            continue;
        }

        for (target_entity, target_pos, target_nation) in targets.iter() {
            if tower_nation.0 == target_nation.0 {
                continue; // 友军
            }

            let distance = (target_pos.0 - tower_pos.0).length();
            if distance < tower.detection_radius {
                // 模糊情报：只标记数量和方向，不暴露详情
                detected_events.send(DetectionEvent {
                    detector_nation: tower_nation.0,
                    detected_position: target_pos.0,
                    detected_count: 1, // 简化
                    direction: (target_pos.0 - tower_pos.0).normalize(),
                    is_precise: false,
                });
            }
        }
    }
}

/// 信号干扰器系统
fn signal_jammer_system(
    jammers: Query<(&Position, &SignalJammer, &NationId)>,
    mut towers: Query<(&Position, &mut ScoutTower)>,
) {
    for (jammer_pos, jammer, jammer_nation) in jammers.iter() {
        for (tower_pos, mut tower) in towers.iter_mut() {
            let distance = (tower_pos.0 - jammer_pos.0).length();
            if distance < jammer.jam_radius {
                // 削减侦测半径 20-40%
                tower.effective_radius = tower.detection_radius * 0.7;
            }
        }
    }
}

/// 战略地图与任务卡系统
fn strategic_map_system(
    kings: Query<(&PlayerId, &NationId), With<King>>,
    nations: Query<&Nation>,
    mut mission_events: EventWriter<MissionCardEvent>,
    mut commands: Commands,
) {
    // 国王可以在战略地图上绘制任务卡
    // 任务卡 = 实体，有过期时间
    for (king_id, nation_id) in kings.iter() {
        // 检查是否有新的任务卡请求
        // ...
    }
}
```

#### 系统 G：职业与战斗系统

```rust
// shared/src/systems/combat/mod.rs
use bevy::prelude::*;

/// 职业切换系统
fn class_switch_system(
    mut commands: Commands,
    mut players: Query<(Entity, &mut ClassLoadout, &mut CurrentClass, &Position)>,
    mut switch_events: EventReader<ClassSwitchEvent>,
    mut cooldowns: Query<&mut ClassSwitchCooldown>,
) {
    for event in switch_events.read() {
        if let Ok((entity, mut loadout, mut current, pos)) = players.get_mut(event.player) {
            // 检查冷却
            if let Ok(mut cooldown) = cooldowns.get_mut(entity) {
                if !cooldown.0.finished() {
                    continue;
                }
                cooldown.0 = Timer::from_seconds(30.0, TimerMode::Once);
            }

            // 检查目标职业是否在 loadout 中
            if loadout.available_classes.contains(&event.target_class) {
                // 移除旧职业组件
                remove_class_components(&mut commands, entity, current.0);

                // 添加新职业组件
                apply_class_components(&mut commands, entity, event.target_class);

                current.0 = event.target_class;
            }
        }
    }
}

/// 刺客"暗影之跃"技能系统
fn assassin_leap_system(
    mut players: Query<(&mut Position, &mut Velocity, &mut Stealth, &AssassinClass), With<Player>>,
    mut skill_events: EventReader<AssassinLeapEvent>,
    time: Res<Time>,
) {
    for event in skill_events.read() {
        if let Ok((mut pos, mut vel, mut stealth, assassin)) = players.get_mut(event.player) {
            // 向前跃进 20 格
            let leap_dir = event.direction.normalize() * 20.0;
            pos.0 += leap_dir;

            // 进入隐身 6 秒
            stealth.is_active = true;
            stealth.duration = Timer::from_seconds(6.0, TimerMode::Once);

            // 速度爆发
            vel.0 = event.direction.normalize() * 15.0;

            // 箭矢免疫（简化）
            // ...
        }
    }
}

/// 狂战士"嗜血渴望"被动系统
fn berserker_bloodlust_system(
    mut players: Query<(&mut MaxHealth, &BerserkerClass, &KillCount), With<Player>>,
) {
    for (mut max_health, berserker, kills) in players.iter_mut() {
        // 每击杀一个敌人，最大生命值增加（边际递减）
        let bonus = calculate_health_bonus(kills.0, berserker.bloodlust_curve);
        max_health.0 = (20.0 + bonus).min(40.0); // 上限 40 颗心
    }
}

/// 吟游诗人"增益音箱"系统
fn bard_buffbox_system(
    mut buffboxes: Query<(&Position, &mut BuffBox, &NationId)>,
    mut players: Query<(&Position, &mut StatusEffects, &NationId), With<Player>>,
    time: Res<Time>,
) {
    for (box_pos, mut buffbox, box_nation) in buffboxes.iter_mut() {
        if buffbox.is_destroyed {
            continue;
        }

        // 15 格范围内的效果
        for (player_pos, mut effects, player_nation) in players.iter_mut() {
            let distance = (player_pos.0 - box_pos.0).length();
            if distance > 15.0 {
                continue;
            }

            if box_nation.0 == player_nation.0 {
                // 友军增益
                match buffbox.current_song {
                    Song::Invigorate => effects.add(StatusEffect::Regeneration(2)),
                    Song::Enlighten => effects.add(StatusEffect::Speed(2)),
                }
            } else {
                // 敌军减益
                match buffbox.current_song {
                    Song::Intimidate => effects.add(StatusEffect::Weakness(3)),
                    Song::Shackle => effects.add(StatusEffect::Slowness(2)),
                }
            }
        }

        // 歌曲冷却
        buffbox.song_cooldown.tick(time.delta());
    }
}
```

#### 系统 H：AI 人口与附身系统

```rust
// shared/src/systems/nation/ai_population.rs
use bevy::prelude::*;

/// AI 人口购买系统
fn ai_purchase_system(
    mut purchase_events: EventReader<AIPurchaseEvent>,
    mut nations: Query<&mut Nation>,
    mut pool: ResMut<GlobalResourcePool>,
    mut commands: Commands,
) {
    for event in purchase_events.read() {
        if let Ok(mut nation) = nations.get_mut(event.nation) {
            // 检查 AI 名额
            if nation.ai_slots_used >= nation.ai_slots_unlocked {
                continue;
            }

            // 消耗灵魂（成本递增）
            let cost = 3 + (nation.ai_slots_used as u32 * 1); // 线性递增
            if pool.soul < cost {
                continue;
            }
            pool.soul -= cost;

            // 生成 AI 实体
            let ai_entity = commands.spawn((
                AIPlayer {
                    assigned_nation: event.nation,
                    assigned_profession: event.profession,
                    maintenance_food: 1.0, // 1 食物/分钟
                },
                Position(event.spawn_position),
                Health(20.0),
                // 继承职业组件
            )).id();

            nation.ai_slots_used += 1;
            nation.members.push(ai_entity);
        }
    }
}

/// 附身系统
fn possession_system(
    mut possession_events: EventReader<PossessionEvent>,
    mut ai_players: Query<(Entity, &mut AIPlayer, &Position)>,
    mut human_players: Query<&mut Player>,
    mut commands: Commands,
) {
    for event in possession_events.read() {
        if let Ok((ai_entity, mut ai, ai_pos)) = ai_players.get_mut(event.ai_entity) {
            if ai.is_idle {
                // 玩家附身 AI
                ai.possessed_by = Some(event.human_player);
                ai.possession_timer = Timer::from_seconds(600.0, TimerMode::Once); // 5-10 分钟

                // 广播给国家成员
                // ...

                // 被攻击时 2 秒硬直
                commands.entity(ai_entity).insert(PossessionChannel {
                    harden_duration: Duration::from_secs(2),
                });
            }
        }
    }
}

/// AI 维护消耗系统（每分钟）
fn ai_maintenance_system(
    time: Res<Time>,
    mut ai_players: Query<&mut AIPlayer>,
    mut nations: Query<&mut Nation>,
    mut pool: ResMut<GlobalResourcePool>,
) {
    // 每 60 秒执行一次
    for mut ai in ai_players.iter_mut() {
        if pool.food >= ai.maintenance_food as u32 {
            pool.food -= ai.maintenance_food as u32;
        } else {
            // 饥饿，AI 死亡
            // ...
        }
    }
}
```

#### 系统 I：终局系统（以太界 + 神器）

```rust
// shared/src/systems/endgame/mod.rs
use bevy::prelude::*;

/// 位面传送门建造系统
fn aether_portal_construction_system(
    mut build_events: EventReader<PortalBuildEvent>,
    mut nations: Query<&mut Nation>,
    mut pool: ResMut<GlobalResourcePool>,
    mut commands: Commands,
) {
    for event in build_events.read() {
        if let Ok(mut nation) = nations.get_mut(event.nation) {
            // 检查资源：50 阳炎石 + 50 霜心晶体 + 50 活根
            let required = [
                ("sunstone", 50),
                ("frostcore", 50),
                ("living_root", 50),
            ];

            for (res, amount) in &required {
                if !pool.can_consume(res, *amount) {
                    return;
                }
            }

            // 扣除资源
            for (res, amount) in &required {
                pool.consume(res, *amount).unwrap();
            }

            // 创建传送门实体（建造中状态）
            commands.spawn((
                AetherPortal {
                    nation_id: event.nation,
                    position: event.position,
                    build_progress: 0.0,
                    state: PortalState::Building,
                    health: 500.0,
                },
            ));
        }
    }
}

/// 以太界重力扭曲系统
fn aether_gravity_system(
    time: Res<Time>,
    mut gravity_timer: ResMut<AetherGravityTimer>,
    mut players: Query<&mut Gravity, With<InAether>>,
) {
    gravity_timer.0.tick(time.delta());

    if gravity_timer.0.just_finished() {
        // 60-120 秒周期切换重力
        let new_multiplier = if fastrand::bool() { 0.5 } else { 1.5 };

        for mut gravity in players.iter_mut() {
            gravity.multiplier = new_multiplier;
        }

        gravity_timer.0 = Timer::from_seconds(
            fastrand::f32() * 60.0 + 60.0,
            TimerMode::Once,
        );
    }
}

/// 以太幽魂生成与掉落系统
fn aether_wraith_system(
    mut spawn_timer: ResMut<WraithSpawnTimer>,
    time: Res<Time>,
    mut pool: ResMut<GlobalResourcePool>,
    mut commands: Commands,
) {
    spawn_timer.0.tick(time.delta());

    if spawn_timer.0.just_finished() {
        // 在以太界随机位置生成幽魂
        commands.spawn((
            AetherWraith {
                position: random_aether_position(),
                health: 100.0,
                teleport_cooldown: Timer::from_seconds(5.0, TimerMode::Once),
                target_player: None,
            },
            MonsterType::AetherWraith,
        ));

        spawn_timer.0 = Timer::from_seconds(30.0, TimerMode::Once);
    }
}

/// 幽魂击杀掉落
fn aether_wraith_death_system(
    mut death_events: EventReader<MonsterDeathEvent>,
    mut pool: ResMut<GlobalResourcePool>,
    mut drop_events: EventWriter<ItemDropEvent>,
) {
    for event in death_events.read() {
        if event.monster_type == MonsterType::AetherWraith {
            // 1-2 个虚空精华
            let amount = fastrand::u32(1..=2);

            if pool.void_essence + amount as u32 <= 100 {
                pool.void_essence += amount as u32;
                drop_events.send(ItemDropEvent {
                    item: Item::VoidEssence(amount),
                    position: event.position,
                });
            }
        }
    }
}

/// 胜利条件检查
fn victory_condition_system(
    nations: Query<&Nation>,
    flags: Query<&NationFlag>,
    mut game_state: ResMut<NextState<GameState>>,
) {
    let active_nations: Vec<_> = nations.iter()
        .filter(|n| !n.is_destroyed)
        .collect();

    if active_nations.len() == 1 {
        let winner = active_nations[0];
        game_state.set(GameState::Victory(winner.id));
    }
}
```

---

## 四、网络架构：确定性 + 客户端预测

### 4.1 技术选型

- **lightyear**：Bevy 原生网络库，支持客户端预测、服务端复制、输入延迟、确定性回滚
- **bincode**：Rust 原生二进制序列化，零拷贝、高性能

### 4.2 消息协议

```rust
// shared/src/net/protocol.rs
use lightyear::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Channel)]
pub struct ReliableChannel;

#[derive(Channel)]
pub struct UnreliableChannel;

#[message_protocol(protocol = "WanGuoProtocol")]
pub enum Messages {
    // === 玩家输入 ===
    PlayerInput(PlayerInput),

    // === 世界状态同步 ===
    ChunkDataSync(ChunkDataSync),
    EntityStateSync(EntityStateSync),

    // === 事件通知 ===
    NationDestroyed(NationDestroyedNotify),
    TreatySigned(TreatySignedNotify),
    CaravanAmbushed(CaravanAmbushedNotify),
    MissionCardIssued(MissionCardNotify),

    // === 系统消息 ===
    ChatMessage(ChatMessage),
    SystemBroadcast(SystemBroadcast),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlayerInput {
    pub tick: u32,
    pub actions: Vec<PlayerAction>,
    pub camera_direction: [f32; 3],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum PlayerAction {
    Move { dx: i8, dy: i8, dz: i8 },
    BreakBlock { x: i32, y: i32, z: i32 },
    PlaceBlock { x: i32, y: i32, z: i32, block_id: u16 },
    UseSkill { skill_id: u16, target: Option<[i32; 3]> },
    SwitchClass { target_class: ClassType },
    Interact { target_entity: Option<Entity> },
    OpenInventory,
    CloseInventory,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChunkDataSync {
    pub coord: ChunkCoord,
    pub blocks: Vec<u16>, // 压缩后的区块数据
    pub checksum: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct EntityStateSync {
    pub network_id: u64,
    pub position: [i32; 3],
    pub velocity: [i32; 3],
    pub health: u16,
    pub status_effects: Vec<StatusEffect>,
}
```

### 4.3 服务端权威架构

```rust
// server/src/main.rs
use bevy::prelude::*;

fn main() {
    App::new()
        // 最小化插件集（无渲染、无窗口、无音频）
        .add_plugins(MinimalPlugins)

        // 网络服务端
        .add_plugins(lightyear::server::ServerPlugins)

        // 共享游戏逻辑
        .add_plugins(GameSystemsPlugin)

        // 服务端专属系统
        .add_plugins(ServerPlugin)

        // 异步持久化
        .add_plugins(PersistencePlugin)

        .run();
}

// server/src/server_plugin.rs
pub struct ServerPlugin;

impl Plugin for ServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            // 输入验证：检查距离、权限、冷却
            validate_player_input,

            // 权威状态更新
            apply_authoritative_state,

            // 状态广播：只发送变化的部分
            broadcast_delta_state,

            // 反作弊：异常检测
            anti_cheat_detection,
        ).chain());
    }
}
```

### 4.4 客户端预测架构

```rust
// client/src/main.rs
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(lightyear::client::ClientPlugins)
        .add_plugins(GameSystemsPlugin)  // 共享逻辑用于预测
        .add_plugins(RenderPlugin)
        .add_plugins(UiPlugin)
        .run();
}

// client/src/prediction.rs
/// 客户端预测系统
fn client_prediction_system(
    mut local_player: Query<&mut PredictedState, With<LocalPlayer>>,
    input_history: Res<InputHistory>,
    server_snapshots: Res<ServerSnapshots>,
) {
    // 1. 本地立即响应输入
    // 2. 发送输入到服务端
    // 3. 收到 snapshot 后回滚到 snapshot 帧
    // 4. 重放 snapshot 之后的所有本地输入
    // 5. 显示插值后的平滑状态
}
```

---

## 五、持久化与异步 IO

### 5.1 设计原则

- **游戏逻辑线程**（Bevy FixedUpdate）：只做内存操作，绝不阻塞
- **IO 线程**（Tokio）：存档、数据库、日志全部异步

### 5.2 实现

```rust
// server/src/persistence/mod.rs
use tokio::sync::mpsc;
use sled::Db;

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        let (tx, mut rx) = mpsc::unbounded_channel::<PersistRequest>();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = sled::open("world/saves").unwrap();

                while let Some(req) = rx.recv().await {
                    match req {
                        PersistRequest::Chunk { coord, data } => {
                            let key = bincode::serialize(&coord).unwrap();
                            let value = zstd::encode_all(
                                &bincode::serialize(&data).unwrap()[..],
                                3, // 压缩级别
                            ).unwrap();
                            db.insert(key, value).unwrap();
                        }
                        PersistRequest::Player { id, state } => {
                            let key = format!("player:{}", id);
                            db.insert(key, bincode::serialize(&state).unwrap()).unwrap();
                        }
                        PersistRequest::Nation { id, state } => {
                            let key = format!("nation:{}", id);
                            db.insert(key, bincode::serialize(&state).unwrap()).unwrap();
                        }
                        PersistRequest::AuditLog { entries } => {
                            let key = format!("audit:{}", chrono::Utc::now().timestamp());
                            db.insert(key, bincode::serialize(&entries).unwrap()).unwrap();
                        }
                    }
                }
            });
        });

        app.insert_resource(PersistenceChannel { tx });
    }
}

#[derive(Resource)]
pub struct PersistenceChannel {
    pub tx: mpsc::UnboundedSender<PersistRequest>,
}

pub enum PersistRequest {
    Chunk { coord: ChunkCoord, data: ChunkData },
    Player { id: u64, state: PlayerState },
    Nation { id: u64, state: NationState },
    AuditLog { entries: Vec<AuditEntry> },
}
```

---

## 六、体素渲染（Client）

### 6.1 网格生成策略

| 层级 | 距离 | 策略 | 实体数量 |
|------|------|------|---------|
| L0 | 0-4 区块 | Greedy Meshing + 完整光照 + 动画 | 每区块 1 个 Mesh 实体 |
| L1 | 4-16 区块 | 简化 Mesh + 静态光照 | 每 2×2 区块合并为 1 个 Mesh |
| L2 | 16-64 区块 | 仅表面体素 | 每 4×4 区块合并 |
| L3 | 64+ 区块 | 高度图 Impostor | 每 8×8 区块 1 个 Billboard |

### 6.2 关键代码

```rust
// client/src/render/chunk_mesh.rs
fn generate_chunk_mesh(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    chunk_query: Query<(Entity, &ChunkCoord, &ChunkData), Changed<ChunkData>>,
    atlas: Res<BlockTextureAtlas>,
) {
    for (entity, coord, data) in chunk_query.iter() {
        let mut mesh_builder = GreedyMeshBuilder::new();

        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let block_id = data.blocks[x + z * 16 + y * 256];
                    if block_id == 0 { continue; }

                    // 检查六个邻居
                    let neighbors = [
                        (x > 0).then(|| data.blocks[(x-1) + z*16 + y*256]),
                        (x < 15).then(|| data.blocks[(x+1) + z*16 + y*256]),
                        (y > 0).then(|| data.blocks[x + z*16 + (y-1)*256]),
                        (y < 15).then(|| data.blocks[x + z*16 + (y+1)*256]),
                        (z > 0).then(|| data.blocks[x + (z-1)*16 + y*256]),
                        (z < 15).then(|| data.blocks[x + (z+1)*16 + y*256]),
                    ];

                    for (i, neighbor) in neighbors.iter().enumerate() {
                        let should_render = match neighbor {
                            Some(n) => *n == 0 || is_transparent(*n),
                            None => true, // 边界，需要渲染
                        };

                        if should_render {
                            mesh_builder.add_face(i, x, y, z, block_id);
                        }
                    }
                }
            }
        }

        let mesh = mesh_builder.build();

        commands.entity(entity).insert(PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(atlas.material.clone()),
            transform: Transform::from_xyz(
                (coord.x * 16) as f32,
                (coord.y * 16) as f32,
                (coord.z * 16) as f32,
            ),
            ..default()
        });
    }
}
```

---

## 七、项目完整 Cargo.toml

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["shared", "server", "client"]
resolver = "2"

[workspace.dependencies]
bevy = "0.18"
lightyear = { version = "0.23", features = ["bevy"] }
serde = { version = "1.0", features = ["derive"] }
bincode = "1.3"
fastrand = "2.0"

# shared/Cargo.toml
[package]
name = "wanguo-shared"
version = "0.1.0"
edition = "2021"

[dependencies]
bevy = { workspace = true, default-features = false, features = ["bevy_ecs", "bevy_asset", "bevy_time"] }
lightyear = { workspace = true }
serde = { workspace = true }
bincode = { workspace = true }
fastrand = { workspace = true }

# server/Cargo.toml
[package]
name = "wanguo-server"
version = "0.1.0"
edition = "2021"

[dependencies]
wanguo-shared = { path = "../shared" }
bevy = { workspace = true, default-features = false, features = ["bevy_ecs", "bevy_time", "bevy_app"] }
lightyear = { workspace = true, features = ["server"] }
tokio = { version = "1.40", features = ["full"] }
sled = "0.34"
tracing = "0.1"
chrono = "0.4"
zstd = "0.13"

# client/Cargo.toml
[package]
name = "wanguo-client"
version = "0.1.0"
edition = "2021"

[dependencies]
wanguo-shared = { path = "../shared" }
bevy = { workspace = true, features = ["bevy_pbr", "bevy_render", "bevy_ui", "bevy_window", "x11"] }
lightyear = { workspace = true, features = ["client"] }
bevy_egui = "0.30"
```

---

## 八、实现路线图

### Phase 1：核心框架（4 周）
- [ ] ECS 组件定义（玩家、区块、国家、资源池）
- [ ] 基础体素世界生成（噪声地形 + 简单矿物）
- [ ] 玩家移动、方块放置/破坏
- [ ] Headless 服务端 + lightyear 网络连接
- [ ] 客户端预测 + 服务端权威验证

### Phase 2：国家与经济（4 周）
- [ ] 国家创建、成员管理、旗帜系统
- [ ] 全局资源池 + 守恒验证
- [ ] 基础职业系统（5-10 个 MVP 职业）
- [ ] 商队系统（移动、护送、掉落）
- [ ] 国家市场（上架、购买、税收）

### Phase 3：生态与信息战（3 周）
- [ ] 浆果丛林生态循环
- [ ] 怪物王国/巢穴层级系统
- [ ] 视野系统（玩家 + 斥候塔）
- [ ] 信号干扰器 + 侦测符印
- [ ] 国王战略地图 + 任务卡

### Phase 4：终局与优化（3 周）
- [ ] 位面传送门 + 以太界
- [ ] 以太幽魂 + 虚空精华掉落
- [ ] 神器合成系统
- [ ] 胜利条件判定
- [ ] 性能优化（多核并行、LOD、视距管理）

### Phase 5：王冠世界（持续）
- [ ] 长线经营世界（皮卡堂风格）
- [ ] 轻职业系统（店长、布置师、工匠）
- [ ] 社交活动与交易
- [ ] 非数值增值内容

---

## 九、与传统 Minecraft 架构对比

| 维度 | Minecraft Java | 万国起源 (Bevy) |
|------|---------------|----------------|
| **并发模型** | 单主线程 Tick | ECS 自动并行 + Job System |
| **实体处理** | OOP 对象，分散内存 | ECS Archetype，连续内存 |
| **资源管理** | 无全局池，各系统独立 | 单一 GlobalResourcePool，强制守恒 |
| **国家系统** | 插件层实现，与核心耦合 | 原生 ECS 实体 + 系统 |
| **网络同步** | 状态同步（NBT 包） | 输入同步 + 确定性模拟 |
| **生态系统** | 简单随机刷怪 | 全局资源池驱动的真实生态 |
| **信息战** | 无原生支持 | 视野组件 + 信号干扰系统 |
| **商队物流** | 无原生支持 | 物理化移动实体 + 护送系统 |
| **AI 人口** | 无原生支持 | AIPlayer 实体 + 附身系统 |
| **终局内容** | 末地龙（单人目标） | 以太界 + 国家级终局战争 |

---

## 十、关键设计决策

### 10.1 为什么用 i32 定点数而不是 f32？

- **确定性**：跨平台（x86/ARM）浮点行为一致
- **网络同步**：输入同步要求所有客户端计算结果完全相同
- **性能**：整数运算比浮点更快，且支持 SIMD 向量化

### 10.2 为什么区块数据不用 Entity-per-block？

- **内存**：4096 个 Entity/区块 × 1000 区块 = 400 万 Entity，ECS 查询爆炸
- **性能**：连续数组 `Box<[u16; 4096]>` 缓存命中率极高
- **并行**：区块级并行处理，而非方块级

### 10.3 为什么条约、任务卡也是 Entity？

- **生命周期**：它们有创建时间、过期时间、状态变化
- **查询**：可以按类型、按国家、按时间范围查询
- **事件**：自然支持 Event 驱动（条约到期、任务完成）

### 10.4 全局资源池为什么是单一 Resource 而不是分布式？

- **守恒验证**：所有变动集中在一个地方，便于审计
- **死锁避免**：无需跨系统锁
- **快照**：便于保存/加载世界状态
