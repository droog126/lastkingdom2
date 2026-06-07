# 万国起源 · 系统内容架构
## 资源转化 × 记忆约束 × 涌现规则

---

## 一、资源系统总纲

### 1.1 资源分类

```
基础资源（不可再分）
├─ 物质类
│   ├─ 元素：氢、氧、碳、铁、金...（原子级别）
│   ├─ 化合物：水(H₂O)、二氧化碳(CO₂)、硅酸盐...
│   └─ 混合物：土壤、矿石、空气
├─ 能量类
│   ├─ 热能（温度）
│   ├─ 光能（光照强度）
│   ├─ 化学能（燃料、食物）
│   └─ 魔法能（以太能量）
└─ 信息类
    ├─ 基因（物种特征）
    ├─ 知识（科技/魔法）
    └─ 记忆（世界历史）

合成资源（由基础资源转化）
├─ 材料：木材、石材、金属锭、布料
├─ 工具：斧头、镐子、剑
├─ 建筑：墙壁、门、机器
├─ 食物：面包、药水
└─ 神器：魔法物品

抽象资源（社会/政治）
├─ 灵魂（生命本质）
├─ 信誉（外交）
├─ 影响力（文化）
└─ 时间（时间流速控制）
```

### 1.2 资源守恒方程

```
世界总质量 = Σ(所有物质资源) = 常量
世界总能量 = Σ(所有能量形式) = 常量（热力学第一定律）
世界总信息 = Σ(所有信息熵) = 单调递增（热力学第二定律）

任何转化：
输入资源A + 输入资源B + 能量 → 输出资源C + 输出资源D + 废热

效率 = 有用输出 / 总输入 < 100%（永远有损耗）
```

### 1.3 资源转化图谱

```rust
pub struct TransformationGraph {
    // 节点 = 资源类型
    nodes: HashMap<ResourceId, ResourceNode>,

    // 边 = 转化规则
    edges: Vec<TransformationRule>,
}

pub struct TransformationRule {
    id: RuleId,
    name: String,

    // 输入（可以有多种组合）
    inputs: Vec<(ResourceId, QuantityRange)>,

    // 输出
    outputs: Vec<(ResourceId, QuantityRange)>,

    // 催化剂（不参与消耗，但影响效率）
    catalysts: Vec<(ResourceId, f32)>,

    // 条件
    conditions: Vec<Condition>,

    // 效率参数
    base_efficiency: f32,        // 基础效率
    efficiency_scaling: Vec<(ResourceId, f32)>, // 效率缩放因子

    // 时间
    duration: DurationRange,

    // 副产品
    byproducts: Vec<(ResourceId, f32, Probability)>, // 资源, 数量, 概率

    // 废热
    waste_heat: f32,
}

pub enum Condition {
    Temperature(Range<f32>),      // 温度范围
    Pressure(Range<f32>),         // 压力范围
    Humidity(Range<f32>),         // 湿度范围
    LightIntensity(Range<f32>),   // 光照强度
    Presence(ResourceId),         // 需要特定资源存在
    Absence(ResourceId),          // 需要特定资源不存在
    TechLevel(TechId, u32),       // 需要科技等级
    MagicLevel(MagicId, u32),     // 需要魔法等级
    TimeOfDay(Range<f32>),        // 一天中的时间
    Season(Season),               // 季节
}
```

---

## 二、物理系统

### 2.1 物理规则

```rust
pub struct PhysicsRules {
    // 重力
    gravity: Vec3,                // 默认 (0, -9.8, 0)
    gravity_varies: bool,         // 是否随位置变化

    // 时间
    time_dilation: f32,         // 时间流速倍率
    time_dilation_varies: bool,   // 是否随区域变化

    // 热力学
    ambient_temperature: f32,     // 环境温度
    heat_conduction: f32,         // 热传导系数

    // 流体
    fluid_density: f32,           // 流体密度
    viscosity: f32,              // 粘度

    // 光学
    light_speed: f32,            // 光速
    light_attenuation: f32,      // 光衰减系数
}
```

### 2.2 方块物理属性

```rust
pub struct BlockPhysics {
    id: BlockId,

    // 力学
    density: f32,                // 密度 (kg/m³)
    hardness: f32,               // 硬度 (挖掘难度)
    elasticity: f32,             // 弹性
    friction: f32,               // 摩擦系数

    // 热学
    thermal_conductivity: f32,   // 热导率
    specific_heat: f32,          // 比热容
    melting_point: f32,          // 熔点
    boiling_point: f32,          // 沸点

    // 光学
    opacity: f32,                // 不透明度
    reflectivity: f32,           // 反射率
    emissivity: f32,             // 发射率

    // 电学
    conductivity: f32,           // 电导率
    permittivity: f32,           // 介电常数

    // 化学
    reactivity: f32,             // 反应活性
    flammability: f32,           // 可燃性

    // 状态
    states: Vec<MaterialState>,   // 固态/液态/气态/等离子态
}

pub struct MaterialState {
    state: StateOfMatter,
    transition_temperature: f32,  // 相变温度
    transition_energy: f32,     // 相变潜热
    properties: StateProperties,
}
```

### 2.3 物理模拟系统

```rust
/// 热传导系统
fn heat_conduction_system(
    mut chunks: Query<&mut ChunkData>,
    time: Res<Time>,
) {
    for mut chunk in chunks.iter_mut() {
        for y in 1..15 {
            for z in 1..15 {
                for x in 1..15 {
                    let idx = x + z*16 + y*256;
                    let block = chunk.blocks[idx];
                    let physics = BLOCK_REGISTRY.get(block).physics;

                    // 与6个邻居热交换
                    let neighbors = [
                        idx - 1, idx + 1,
                        idx - 16, idx + 16,
                        idx - 256, idx + 256,
                    ];

                    for &neighbor_idx in &neighbors {
                        let neighbor_block = chunk.blocks[neighbor_idx];
                        let neighbor_physics = BLOCK_REGISTRY.get(neighbor_block).physics;

                        let temp_diff = chunk.temperature[idx] - chunk.temperature[neighbor_idx];
                        let heat_transfer = temp_diff * physics.thermal_conductivity * neighbor_physics.thermal_conductivity * time.delta_seconds();

                        chunk.temperature[idx] -= heat_transfer / (physics.density * physics.specific_heat);
                        chunk.temperature[neighbor_idx] += heat_transfer / (neighbor_physics.density * neighbor_physics.specific_heat);
                    }

                    // 相变检查
                    check_phase_transition(&mut chunk, idx, &physics);
                }
            }
        }
    }
}

/// 流体模拟系统（简化版）
fn fluid_simulation_system(
    mut chunks: Query<&mut ChunkData>,
) {
    for mut chunk in chunks.iter_mut() {
        // 简单的重力驱动流体
        for y in (1..15).rev() {
            for z in 1..15 {
                for x in 1..15 {
                    let idx = x + z*16 + y*256;
                    let block = chunk.blocks[idx];

                    if is_fluid(block) {
                        let below_idx = idx - 256;
                        let below_block = chunk.blocks[below_idx];

                        if is_air(below_block) {
                            // 下落
                            chunk.blocks[below_idx] = block;
                            chunk.blocks[idx] = AIR_BLOCK;
                        } else if is_fluid(below_block) && fluid_level(below_block) < MAX_LEVEL {
                            // 填充下方
                            let transfer = min(fluid_level(block), MAX_LEVEL - fluid_level(below_block));
                            chunk.blocks[below_idx] = set_fluid_level(below_block, fluid_level(below_block) + transfer);
                            chunk.blocks[idx] = set_fluid_level(block, fluid_level(block) - transfer);
                        } else {
                            // 水平扩散
                            spread_fluid(&mut chunk, idx, x, y, z);
                        }
                    }
                }
            }
        }
    }
}
```

---

## 三、生态系统

### 3.1 物种定义

```rust
pub struct Species {
    id: SpeciesId,
    name: String,

    // 分类
    kingdom: Kingdom,        // 动物/植物/真菌/细菌
    diet: Diet,            // 食性

    // 代谢
    base_metabolism: f32,  // 基础代谢率 (能量/秒)
    energy_efficiency: f32, // 能量转化效率

    // 繁殖
    reproduction_strategy: ReproductionStrategy,
    gestation_period: Duration,
    offspring_count: Range<u32>,
    maturity_age: Duration,
    lifespan: Duration,

    // 行为
    behavior_type: BehaviorType,
    territory_size: f32,
    social_structure: SocialStructure,

    // 环境适应
    preferred_biome: Vec<BiomeType>,
    temperature_tolerance: Range<f32>,
    humidity_tolerance: Range<f32>,

    // 外观
    model: ModelId,
    textures: Vec<TextureId>,

    // 基因
    genome: Genome,
}

pub struct Genome {
    // 基因位点
    genes: HashMap<GeneId, Gene>,

    // 突变率
    mutation_rate: f32,

    // 遗传特征
    traits: Vec<Trait>,
}

pub struct Gene {
    id: GeneId,
    alleles: Vec<Allele>,
    dominance: DominanceType,

    // 表型影响
    phenotype_effects: Vec<PhenotypeEffect>,
}
```

### 3.2 生态实体组件

```rust
/// 生物实体基础组件
#[derive(Component)]
pub struct Organism {
    species: SpeciesId,
    age: Duration,

    // 状态
    health: f32,
    energy: f32,
    hydration: f32,

    // 繁殖
    is_mature: bool,
    reproduction_cooldown: Timer,

    // 基因
    genome: Genome,

    // 记忆（个体学习）
    memory: Vec<Memory>,
}

#[derive(Component)]
pub struct Predator {
    prey_preferences: Vec<SpeciesId>,
    hunting_efficiency: f32,
    last_hunt_time: Instant,
}

#[derive(Component)]
pub struct Herbivore {
    graze_efficiency: f32,
    preferred_plants: Vec<SpeciesId>,
    stomach_capacity: f32,
    current_food: f32,
}

#[derive(Component)]
pub struct Plant {
    growth_stage: GrowthStage,
    photosynthesis_rate: f32,
    root_depth: f32,

    // 繁殖
    seed_production_rate: f32,
    seed_dispersal_range: f32,
}
```

### 3.3 生态模拟系统

```rust
/// 光合作用系统
fn photosynthesis_system(
    mut plants: Query<(&mut Plant, &mut Organism, &Position)>,
    chunks: Query<&ChunkData>,
    time: Res<Time>,
) {
    for (mut plant, mut organism, pos) in plants.iter_mut() {
        // 获取环境光照
        let light = get_light_intensity(pos, &chunks);

        // 光合作用：光能 + CO₂ + H₂O → 葡萄糖 + O₂
        let glucose_produced = light * plant.photosynthesis_rate * time.delta_seconds();

        organism.energy += glucose_produced;

        // 生长
        if organism.energy > plant.growth_stage.energy_threshold() {
            plant.growth_stage = plant.growth_stage.next();
            organism.energy -= plant.growth_stage.energy_cost();
        }
    }
}

/// 捕食系统
fn predation_system(
    mut predators: Query<(&mut Predator, &mut Organism, &Position)>,
    mut prey: Query<(&mut Organism, &Position), Without<Predator>>,
    time: Res<Time>,
) {
    for (mut predator, mut predator_org, predator_pos) in predators.iter_mut() {
        if predator_org.energy > predator_org.species.base_metabolism * 10.0 {
            continue; // 不饿
        }

        // 寻找猎物
        let nearest_prey = find_nearest_prey(&prey, predator_pos, predator.prey_preferences);

        if let Some((prey_entity, prey_pos, distance)) = nearest_prey {
            if distance < 5.0 {
                // 攻击
                if fastrand::f32() < predator.hunting_efficiency {
                    // 成功捕食
                    if let Ok((mut prey_org, _)) = prey.get_mut(prey_entity) {
                        let meat_energy = prey_org.energy * 0.8;
                        predator_org.energy += meat_energy;
                        prey_org.health = 0.0; // 死亡

                        // 尸体转化为分解者资源
                        // ...
                    }
                }
            } else {
                // 追击
                let direction = (prey_pos - predator_pos).normalize();
                // 移动逻辑...
            }
        }
    }
}

/// 繁殖系统
fn reproduction_system(
    mut organisms: Query<&mut Organism>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for mut organism in organisms.iter_mut() {
        if !organism.is_mature {
            continue;
        }

        organism.reproduction_cooldown.tick(time.delta());

        if organism.reproduction_cooldown.finished() {
            // 寻找配偶
            // ...

            // 产生后代
            let offspring_count = organism.species.offspring_count.sample();
            for _ in 0..offspring_count {
                let child_genome = crossover(&organism.genome, &mate_genome);
                let mutated_genome = mutate(child_genome, organism.species.genome.mutation_rate);

                commands.spawn((
                    Organism {
                        species: organism.species,
                        age: Duration::ZERO,
                        health: 1.0,
                        energy: organism.energy * 0.1, // 父母投入能量
                        hydration: 1.0,
                        is_mature: false,
                        reproduction_cooldown: Timer::from_seconds(0.0, TimerMode::Once),
                        genome: mutated_genome,
                        memory: Vec::new(),
                    },
                    // 根据物种添加特定组件
                ));
            }

            organism.energy *= 0.5; // 繁殖消耗大量能量
            organism.reproduction_cooldown = Timer::from_seconds(
                organism.species.gestation_period.as_secs_f32(),
                TimerMode::Once,
            );
        }
    }
}

/// 迁徙系统
fn migration_system(
    mut animals: Query<(&mut Organism, &mut Position, &mut Velocity)>,
    chunks: Query<&ChunkData>,
    time: Res<Time>,
) {
    for (mut organism, mut pos, mut vel) in animals.iter_mut() {
        // 检查当前环境是否适宜
        let current_biome = get_biome_at(pos.0, &chunks);
        let is_suitable = organism.species.preferred_biome.contains(&current_biome);

        if !is_suitable {
            // 寻找更适宜的区域
            let target = find_suitable_habitat(pos.0, &organism.species, &chunks);
            if let Some(target) = target {
                let direction = (target - pos.0).normalize();
                vel.0 += direction * 2.0 * time.delta_seconds();
            }
        }

        // 群体行为（如果社会性动物）
        if organism.species.social_structure != SocialStructure::Solitary {
            let flock_center = calculate_flock_center(&animals, pos.0, 20.0);
            let separation = calculate_separation(&animals, pos.0, 5.0);
            let alignment = calculate_alignment(&animals, pos.0, 20.0);

            vel.0 += (flock_center + separation + alignment) * time.delta_seconds();
        }
    }
}
```

---

## 四、记忆与约束系统

### 4.1 世界记忆

```rust
/// 世界记忆 = 所有已经发生的事件记录
#[derive(Resource)]
pub struct WorldMemory {
    // 按时间索引的事件日志
    events: BTreeMap<u64, Vec<WorldEvent>>, // tick -> events

    // 按类型索引
    events_by_type: HashMap<EventType, Vec<u64>>, // type -> ticks

    // 按实体索引
    events_by_entity: HashMap<Entity, Vec<u64>>, // entity -> ticks

    // 摘要（用于快速查询）
    summaries: Vec<TimePeriodSummary>,
}

pub struct WorldEvent {
    tick: u64,
    event_type: EventType,

    // 参与者
    primary_actor: Option<Entity>,
    secondary_actors: Vec<Entity>,

    // 位置
    location: Vec3,

    // 详细数据
    payload: EventPayload,

    // 结果
    consequences: Vec<EventConsequence>,
}

pub enum EventType {
    BlockPlaced,
    BlockBroken,
    EntitySpawned,
    EntityDied,
    Combat,
    Trade,
    TreatySigned,
    TreatyBroken,
    NationFounded,
    NationDestroyed,
    Discovery,
    Invention,
    NaturalDisaster,
}
```

### 4.2 记忆约束

```rust
/// 记忆约束 = 过去的状态限制未来的可能性
#[derive(Resource)]
pub struct MemoryConstraints {
    // 不可逆操作
    irreversible_actions: Vec<IrreversibleAction>,

    // 承诺与义务
    active_commitments: Vec<Commitment>,

    // 历史路径依赖
    path_dependencies: Vec<PathDependency>,
}

pub struct IrreversibleAction {
    action_type: ActionType,
    actor: Entity,
    tick: u64,

    // 为什么不能撤销
    reason: String,

    // 影响范围
    affected_entities: Vec<Entity>,
    affected_resources: Vec<ResourceId>,
}

pub struct Commitment {
    id: CommitmentId,
    made_by: Entity,
    made_at: u64,

    // 承诺内容
    obligation: Obligation,

    // 违约后果
    breach_consequences: Vec<Consequence>,

    // 有效期
    expiry: Option<u64>,
}

pub struct PathDependency {
    // 早期选择
    initial_choice: Choice,
    chosen_at: u64,

    // 后续锁定
    locked_options: Vec<Choice>,

    // 转换成本
    switch_cost: ResourceCost,
}
```

### 4.3 记忆查询系统

```rust
/// 查询历史事件
fn query_memory_system(
    memory: Res<WorldMemory>,
    query: MemoryQuery,
) -> Vec<&WorldEvent> {
    let mut results = Vec::new();

    // 按时间范围过滤
    let time_range = query.time_range.unwrap_or(0..u64::MAX);

    for (tick, events) in memory.events.range(time_range) {
        for event in events {
            // 按类型过滤
            if let Some(ref types) = query.event_types {
                if !types.contains(&event.event_type) {
                    continue;
                }
            }

            // 按参与者过滤
            if let Some(ref actors) = query.actors {
                let mut actor_match = false;
                if let Some(primary) = event.primary_actor {
                    if actors.contains(&primary) {
                        actor_match = true;
                    }
                }
                for actor in &event.secondary_actors {
                    if actors.contains(actor) {
                        actor_match = true;
                    }
                }
                if !actor_match {
                    continue;
                }
            }

            // 按位置过滤
            if let Some(ref region) = query.region {
                if !region.contains(event.location) {
                    continue;
                }
            }

            results.push(event);
        }
    }

    results
}
```

---

## 五、涌现规则系统

### 5.1 规则定义

```rust
/// 涌现规则 = 当条件满足时，自动触发的系统级事件
pub struct EmergenceRule {
    id: RuleId,
    name: String,
    description: String,

    // 触发条件
    trigger: TriggerCondition,

    // 效果
    effects: Vec<EmergenceEffect>,

    // 冷却
    cooldown: Duration,
    last_triggered: Option<Instant>,

    // 概率
    probability: f32,

    // 只能触发一次？
    one_time: bool,
    already_triggered: bool,
}

pub enum TriggerCondition {
    // 资源阈值
    ResourceThreshold {
        resource: ResourceId,
        threshold: f32,
        comparison: Comparison,
    },

    // 实体数量
    EntityCount {
        entity_type: EntityType,
        count: u32,
        comparison: Comparison,
    },

    // 复合条件
    All(Vec<TriggerCondition>),
    Any(Vec<TriggerCondition>),
    Not(Box<TriggerCondition>),
}

pub enum EmergenceEffect {
    // 生成实体
    SpawnEntity {
        entity_type: EntityType,
        count: Range<u32>,
        location: SpawnLocation,
    },

    // 修改全局参数
    ModifyGlobal {
        parameter: GlobalParameter,
        value: f32,
        duration: Option<Duration>,
    },

    // 触发事件
    TriggerEvent {
        event_type: EventType,
        payload: EventPayload,
    },

    // 创建新规则
    CreateRule {
        rule: EmergenceRule,
    },

    // 解锁科技
    UnlockTech {
        tech_id: TechId,
    },
}
```

### 5.2 涌现规则示例

```rust
/// 过度放牧 → 草原退化 → 沙尘暴
fn overgrazing_rule() -> EmergenceRule {
    EmergenceRule {
        id: rule_id("overgrazing_cascade"),
        name: "过度放牧连锁反应".to_string(),
        description: "当某个区域的食草动物密度超过阈值时，触发草原退化和沙尘暴".to_string(),

        trigger: TriggerCondition::All(vec![
            TriggerCondition::EntityCount {
                entity_type: EntityType::Herbivore,
                count: 60,
                comparison: Comparison::GreaterThan,
            },
            TriggerCondition::ResourceThreshold {
                resource: resource_id("grass_coverage"),
                threshold: 0.3,
                comparison: Comparison::LessThan,
            },
        ]),

        effects: vec![
            // 草原退化
            EmergenceEffect::ModifyGlobal {
                parameter: GlobalParameter::SoilQuality,
                value: -0.5,
                duration: Some(Duration::from_secs(3600)),
            },

            // 沙尘暴概率增加
            EmergenceEffect::ModifyGlobal {
                parameter: GlobalParameter::SandstormProbability,
                value: 0.8,
                duration: Some(Duration::from_secs(7200)),
            },

            // 生成沙尘暴事件
            EmergenceEffect::TriggerEvent {
                event_type: EventType::NaturalDisaster,
                payload: EventPayload::Sandstorm {
                    location: Location::RegionAverage,
                    intensity: 0.7,
                    duration: Duration::from_secs(1800),
                },
            },
        ],

        cooldown: Duration::from_secs(3600),
        last_triggered: None,
        probability: 0.3,
        one_time: false,
        already_triggered: false,
    }
}

/// 文明崛起 → 新技术涌现
fn civilization_emergence_rule() -> EmergenceRule {
    EmergenceRule {
        id: rule_id("civilization_emergence"),
        name: "文明技术涌现".to_string(),
        description: "当某个国家的人口和科技积累达到阈值时，涌现新的科技".to_string(),

        trigger: TriggerCondition::All(vec![
            TriggerCondition::EntityCount {
                entity_type: EntityType::NationMember,
                count: 30,
                comparison: Comparison::GreaterThan,
            },
            TriggerCondition::ResourceThreshold {
                resource: resource_id("knowledge_points"),
                threshold: 1000.0,
                comparison: Comparison::GreaterThan,
            },
        ]),

        effects: vec![
            // 解锁随机高级科技
            EmergenceEffect::UnlockTech {
                tech_id: random_advanced_tech(),
            },

            // 触发文明事件
            EmergenceEffect::TriggerEvent {
                event_type: EventType::Discovery,
                payload: EventPayload::TechDiscovery {
                    discoverer: Entity::PLACEHOLDER,
                    tech: TechId::PLACEHOLDER,
                },
            },
        ],

        cooldown: Duration::from_secs(86400),
        last_triggered: None,
        probability: 0.5,
        one_time: false,
        already_triggered: false,
    }
}
```

### 5.3 涌现系统执行

```rust
fn emergence_system(
    mut rules: ResMut<Vec<EmergenceRule>>,
    world_state: Res<WorldState>,
    mut commands: Commands,
    mut events: EventWriter<WorldEvent>,
) {
    for rule in rules.iter_mut() {
        // 检查冷却
        if let Some(last) = rule.last_triggered {
            if last.elapsed() < rule.cooldown {
                continue;
            }
        }

        // 检查是否已触发（一次性规则）
        if rule.one_time && rule.already_triggered {
            continue;
        }

        // 检查触发条件
        if check_trigger(&rule.trigger, &world_state) {
            // 概率检查
            if fastrand::f32() < rule.probability {
                // 执行效果
                for effect in &rule.effects {
                    apply_emergence_effect(effect, &mut commands, &mut events);
                }

                rule.last_triggered = Some(Instant::now());
                rule.already_triggered = true;

                info!("Emergence rule triggered: {}", rule.name);
            }
        }
    }
}
```

---

## 六、转化自定义系统

### 6.1 玩家自定义转化

```rust
/// 玩家可以定义新的转化规则
pub struct CustomTransformation {
    id: TransformationId,
    creator: PlayerId,

    // 输入
    inputs: Vec<(ResourceId, Quantity)>,

    // 输出
    outputs: Vec<(ResourceId, Quantity)>,

    // 工具/设备要求
    required_tools: Vec<ToolId>,

    // 环境要求
    required_conditions: Vec<Condition>,

    // 效率（受技能影响）
    base_efficiency: f32,

    // 是否公开（其他玩家可用）
    is_public: bool,

    // 验证状态
    verified: bool,
    verification_tick: u64,
}

/// 验证自定义转化（确保守恒）
fn verify_transformation(transformation: &CustomTransformation) -> Result<(), VerificationError> {
    // 1. 计算输入总质量
    let input_mass: f32 = transformation.inputs.iter()
        .map(|(r, q)| get_resource_density(*r) * *q as f32)
        .sum();

    // 2. 计算输出总质量
    let output_mass: f32 = transformation.outputs.iter()
        .map(|(r, q)| get_resource_density(*r) * *q as f32)
        .sum();

    // 3. 检查质量守恒（允许微小误差）
    if (input_mass - output_mass).abs() > 0.01 * input_mass {
        return Err(VerificationError::MassNotConserved {
            input: input_mass,
            output: output_mass,
        });
    }

    // 4. 检查能量守恒
    // ...

    // 5. 检查是否产生悖论（如永动机）
    if is_perpetual_motion(transformation) {
        return Err(VerificationError::PerpetualMotion);
    }

    Ok(())
}
```

### 6.2 配方发现系统

```rust
/// 通过实验发现新配方
pub struct Experiment {
    id: ExperimentId,
    experimenter: PlayerId,

    // 实验设置
    inputs: Vec<(ResourceId, Quantity)>,
    conditions: Vec<Condition>,

    // 实验结果
    outputs: Vec<(ResourceId, Quantity)>,
    observed_at: u64,

    // 可重复性
    repeat_count: u32,
    success_count: u32,
}

/// 实验验证
fn experiment_system(
    mut experiments: Query<&mut Experiment>,
    mut discovered_recipes: ResMut<Vec<CustomTransformation>>,
) {
    for mut experiment in experiments.iter_mut() {
        if experiment.repeat_count >= 3 && experiment.success_count >= 2 {
            // 足够重复，可以注册为新配方
            let recipe = CustomTransformation {
                id: generate_id(),
                creator: experiment.experimenter,
                inputs: experiment.inputs.clone(),
                outputs: experiment.outputs.clone(),
                required_tools: vec![], // 从实验推断
                required_conditions: experiment.conditions.clone(),
                base_efficiency: experiment.success_count as f32 / experiment.repeat_count as f32,
                is_public: false, // 默认私有
                verified: true,
                verification_tick: experiment.observed_at,
            };

            discovered_recipes.push(recipe);
        }
    }
}
```

---

## 七、时间系统详细设计

### 7.1 时间分支管理

```rust
pub struct TimelineManager {
    // 主时间线
    main_timeline: TimelineId,

    // 所有时间线
    timelines: HashMap<TimelineId, Timeline>,

    // 分支关系图
    branch_graph: Graph<TimelineId, BranchRelationship>,
}

pub struct Timeline {
    id: TimelineId,

    // 父时间线（如果是分支）
    parent: Option<TimelineId>,
    branch_point: Option<u64>,

    // 状态
    current_tick: u64,
    world_state: WorldState,

    // 输入历史
    inputs: BTreeMap<u64, Vec<PlayerInput>>,

    // 快照
    snapshots: BTreeMap<u64, WorldSnapshot>,

    // 活跃玩家
    active_players: HashSet<PlayerId>,

    // 时间流速
    time_dilation: f32,
}

pub enum BranchRelationship {
    ParentChild,      // 父子
    Sibling,          // 兄弟（同父）
    Merge {          // 合并
        merged_at: u64,
        resolution: MergeResolution,
    },
}

pub enum MergeResolution {
    TakeA,           // 保留分支A
    TakeB,           // 保留分支B
    Merge {         // 合并两者
        conflict_resolution: HashMap<EntityId, EntityState>,
    },
}
```

### 7.2 分支创建与合并

```rust
impl TimelineManager {
    /// 创建分支
    pub fn create_branch(
        &mut self,
        parent: TimelineId,
        branch_point: u64,
        creator: PlayerId,
    ) -> Result<TimelineId, BranchError> {
        let parent_timeline = self.timelines.get(&parent)
            .ok_or(BranchError::ParentNotFound)?;

        // 获取分支点快照
        let snapshot = parent_timeline.snapshots.get(&branch_point)
            .ok_or(BranchError::SnapshotNotFound)?;

        let new_timeline = Timeline {
            id: generate_uuid(),
            parent: Some(parent),
            branch_point: Some(branch_point),
            current_tick: branch_point,
            world_state: snapshot.clone(),
            inputs: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            active_players: HashSet::new(),
            time_dilation: 1.0,
        };

        let new_id = new_timeline.id;
        self.timelines.insert(new_id, new_timeline);

        // 更新分支图
        self.branch_graph.add_edge(parent, new_id, BranchRelationship::ParentChild);

        Ok(new_id)
    }

    /// 合并分支
    pub fn merge_branch(
        &mut self,
        branch: TimelineId,
        into: TimelineId,
        resolution: MergeResolution,
    ) -> Result<(), MergeError> {
        let branch_timeline = self.timelines.get(&branch)
            .ok_or(MergeError::BranchNotFound)?;
        let into_timeline = self.timelines.get(&into)
            .ok_or(MergeError::TargetNotFound)?;

        // 检查是否可以合并（没有冲突或冲突已解决）
        let conflicts = detect_conflicts(&branch_timeline.world_state, &into_timeline.world_state);

        if !conflicts.is_empty() && !matches!(resolution, MergeResolution::Merge { .. }) {
            return Err(MergeError::UnresolvedConflicts(conflicts));
        }

        // 应用合并
        match resolution {
            MergeResolution::TakeA => {
                // 保留分支状态
                self.timelines.get_mut(&into).unwrap().world_state = branch_timeline.world_state.clone();
            }
            MergeResolution::TakeB => {
                // 保留目标状态，不做任何事
            }
            MergeResolution::Merge { conflict_resolution } => {
                // 应用冲突解决
                for (entity, state) in conflict_resolution {
                    into_timeline.world_state.entities.insert(entity, state);
                }
            }
        }

        // 标记分支为已合并
        self.branch_graph.add_edge(branch, into, BranchRelationship::Merge {
            merged_at: into_timeline.current_tick,
            resolution,
        });

        Ok(())
    }
}
```

---

## 八、系统交互图

```
┌─────────────────────────────────────────────────────────────┐
│                        核心循环                               │
│                                                             │
│   Tick N                                                    │
│     │                                                       │
│     ▼                                                       │
│   ┌─────────────┐                                           │
│   │ 输入收集     │ <── 客户端输入 / AI决策 / 生态自发行为       │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 物理系统     │ ──► 运动、碰撞、热传导、流体               │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 生态系统     │ ──► 生长、繁殖、捕食、迁徙、死亡           │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 经济系统     │ ──► 生产、交易、物流、税收                 │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 国家系统     │ ──► 政治、外交、战争、科技                 │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 涌现检查     │ ──► 检查所有涌现规则触发条件               │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 状态更新     │ ──► 应用所有变化，更新世界状态             │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 记忆记录     │ ──► 记录本Tick所有事件到世界记忆           │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 快照保存     │ ──► 定期保存世界快照（用于回滚/分支）       │
│   └──────┬──────┘                                           │
│          │                                                  │
│          ▼                                                  │
│   ┌─────────────┐                                           │
│   │ 广播同步     │ ──► 发送状态更新到所有客户端               │
│   └─────────────┘                                           │
│                                                             │
│   Tick N+1                                                  │
└─────────────────────────────────────────────────────────────┘
```

---

## 九、关键数据结构

### 9.1 世界状态

```rust
pub struct WorldState {
    // 时间
    tick: u64,
    real_time: Instant,

    // 空间
    chunks: HashMap<ChunkCoord, ChunkData>,
    entities: HashMap<Entity, EntityState>,

    // 资源
    resource_pool: GlobalResourcePool,

    // 生态
    ecosystem: EcosystemState,

    // 社会
    nations: HashMap<NationId, NationState>,
    treaties: Vec<Treaty>,

    // 经济
    markets: HashMap<MarketId, MarketState>,

    // 物理参数
    physics_rules: PhysicsRules,

    // 记忆
    memory: WorldMemory,

    // 约束
    constraints: MemoryConstraints,

    // 涌现规则
    emergence_rules: Vec<EmergenceRule>,
}
```

### 9.2 实体状态

```rust
pub struct EntityState {
    // 基础
    entity_type: EntityType,
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,

    // 物理
    mass: f32,
    health: f32,
    energy: f32,

    // 视觉
    model: ModelId,
    animation: AnimationState,

    // 交互
    inventory: Vec<ItemStack>,
    equipment: EquipmentSlots,

    // 关系
    owner: Option<Entity>,
    faction: Option<FactionId>,

    // 状态效果
    status_effects: Vec<StatusEffect>,

    // 记忆（如果是智能实体）
    memory: Option<EntityMemory>,
}
```

---

## 十、性能预算

### 10.1 Tick 时间分配（20 TPS = 50ms/Tick）

| 系统 | 预算 | 说明 |
|------|------|------|
| 输入处理 | 2ms | 收集、验证、分发 |
| 物理模拟 | 10ms | 运动、碰撞、热传导 |
| 生态系统 | 8ms | 生长、AI、繁殖 |
| 经济系统 | 5ms | 交易、物流、市场 |
| 国家系统 | 3ms | 政治、外交、战争 |
| 涌现检查 | 2ms | 规则匹配、触发 |
| 状态更新 | 5ms | 应用变化、事件生成 |
| 网络同步 | 10ms | 压缩、广播、序列化 |
| 持久化 | 5ms | 快照、日志、保存 |
| **总计** | **50ms** | |

### 10.2 内存预算

| 数据 | 每玩家 | 每区块 | 总计（100玩家/1000区块） |
|------|--------|--------|------------------------|
| 玩家状态 | 10KB | - | 1MB |
| 区块数据 | - | 64KB | 64MB |
| 实体 | - | - | 50MB |
| 生态状态 | - | 4KB | 4MB |
| 经济数据 | - | - | 10MB |
| 记忆日志 | - | - | 100MB（可清理） |
| **总计** | | | **~230MB** |

### 10.3 网络预算

| 数据类型 | 频率 | 大小 | 每玩家/秒 |
|---------|------|------|----------|
| 玩家输入 | 20Hz | 50B | 1KB/s 上行 |
| 状态快照 | 20Hz | 5KB（压缩后） | 100KB/s 下行 |
| 事件通知 | 即时 | 200B | ~10KB/s |
| 区块数据 | 按需 | 10KB | ~50KB/s |
| **总计** | | | **~160KB/s 下行** |
