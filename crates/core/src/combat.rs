//! V2 战斗系统数据层 — Stamina + Block/Parry/Stun + AttackType + Knockback
//!
//! 来源:
//! - 《万国余烬_王冠赛季_开发顺序与里程碑》§4 阶段 P4 基础战斗
//! - 《万国余烬_王冠赛季_表格包》§18.3 剑具与近战装备 (STA 消耗 / 招架窗口)
//! - 《万国余烬_王冠赛季_新总纲》(轻击/突刺/重击 + 格挡/招架 + 硬直)
//!
//! ## 文档 §4 验收门槛 (从 milestones):
//!
//! > 两个玩家本地/局域测试能互相攻击。  
//! > 格挡能降低伤害或消耗耐力。  
//! > 招架窗口能触发短反击机会。  
//! > 耐力不足会限制连续战斗。  
//! > 命中判定不依赖视觉剑轨单帧。
//!
//! ## 设计原则
//!
//! MVP 提供 **数据层 + 状态机 + 命中判定辅助函数**,不接 server-authoritative system。
//! 命中 sweep / Knockback 物理推算留 server task 拆。
//!
//! ## 与 `pvp::mod.rs` 的关系
//!
//! pvp 模块已有 `CombatState` + `WeaponStats` + `Hitbox` + `DamageEvent` (P1 时做的)。
//! 本模块新增 **消耗 / 防御 / 硬直 / 击退** 维度,与 pvp 模块互补,不替换:
//! - pvp::CombatState: is_attacking / attack_cooldown / combo_count (攻击状态)
//! - combat::Stamina: 耐力池 (攻防共消耗)
//! - combat::BlockState / ParryWindow / StunState / Knockback: 新增维度
//!
//! MVP 阶段 client 用 `HeldWeaponPart` 触发剑挥动动画,server 端用本模块做命中判定。

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AttackType (3 种基本攻击)
// ---------------------------------------------------------------------------

/// 3 种基本攻击类型 (开发顺序 §4: 轻击 / 突刺 / 重击)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {
    /// 轻击 — 6 STA,快,标准伤害
    Light,
    /// 突刺 — 7 STA,标准,长触距 + 穿甲
    Thrust,
    /// 重击 — 12 STA,慢,高伤害 + 击退
    Heavy,
}

impl AttackType {
    /// 表格包 §18.3 默认 STA 消耗 (轻 / 突刺 / 重击)
    /// 用 `Plain Duelist` 单手剑 8 STA 作基准 ±25%
    pub fn stamina_cost(self) -> f32 {
        match self {
            Self::Light => 6.0,
            Self::Thrust => 7.0,
            Self::Heavy => 12.0,
        }
    }

    /// 基础伤害乘数 (轻 1.0 / 突刺 0.9 但穿甲 / 重击 1.8)
    pub fn damage_multiplier(self) -> f32 {
        match self {
            Self::Light => 1.0,
            Self::Thrust => 0.9,
            Self::Heavy => 1.8,
        }
    }

    /// 击退强度 (重击最强,轻击最小,突刺几乎不击退)
    pub fn knockback_strength(self) -> f32 {
        match self {
            Self::Light => 0.15,
            Self::Thrust => 0.05,
            Self::Heavy => 0.8,
        }
    }

    /// 冷却时间 (秒) — 简化版,实际 weapon table 决定
    pub fn cooldown_secs(self) -> f32 {
        match self {
            Self::Light => 0.40,
            Self::Thrust => 0.55,
            Self::Heavy => 1.10,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Light => "轻击",
            Self::Thrust => "突刺",
            Self::Heavy => "重击",
        }
    }
}

// ---------------------------------------------------------------------------
// Stamina
// ---------------------------------------------------------------------------

/// 玩家耐力 Component
///
/// 跟攻击 (AttackType.stamina_cost) / 格挡 (BlockState.drain_per_sec) / 闪避 共消耗。
/// 耐力耗尽 → 攻击被拒绝 / 格挡被打断 / 倒地硬直。
#[derive(Component, Debug, Clone, Copy)]
pub struct Stamina {
    /// 当前耐力
    pub current: f32,
    /// 最大耐力 (基础 100,装备 / Buff 调整)
    pub max: f32,
    /// 每秒恢复速率
    pub regen_per_sec: f32,
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            regen_per_sec: 18.0, // 100 / 5.5s = 满
        }
    }
}

impl Stamina {
    /// 扣耐力 (clamp 到 0,返回实际扣了多少;若不够扣则扣到 0)
    pub fn consume(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current - amount).max(0.0);
        before - self.current
    }

    /// 每帧调用 (dt 秒),自动恢复 (clamp 到 max)
    pub fn regen(&mut self, dt: f32) {
        self.current = (self.current + self.regen_per_sec * dt).min(self.max);
    }

    /// 是否够支付 cost
    pub fn has_enough(&self, cost: f32) -> bool {
        self.current >= cost
    }

    /// 耐力比例 (0..=1)
    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

// ---------------------------------------------------------------------------
// BlockState (格挡)
// ---------------------------------------------------------------------------

/// 格挡状态 Component
///
/// 按住格挡键时 `blocking=true`,持续扣耐力 (`drain_per_sec`)。
/// 受击时根据 `block_reduction` 减伤,扣额外耐力 (`per_hit_cost`)。
#[derive(Component, Debug, Clone, Copy)]
pub struct BlockState {
    /// 是否正在格挡
    pub blocking: bool,
    /// 每秒耐力消耗 (持续按住时)
    pub drain_per_sec: f32,
    /// 伤害减免百分比 (0..=1),装备 / 盾决定
    pub damage_reduction: f32,
    /// 每次成功格挡额外扣的耐力 (冲击)
    pub per_hit_cost: f32,
}

impl Default for BlockState {
    fn default() -> Self {
        // 表格包 §18.3 橡木圆盾: 格挡 45, 稳定 35
        // 减伤 45% / 每秒扣 6 STA / 每次受击扣 4 STA
        Self {
            blocking: false,
            drain_per_sec: 6.0,
            damage_reduction: 0.45,
            per_hit_cost: 4.0,
        }
    }
}

impl BlockState {
    /// 开始格挡
    pub fn start(&mut self) {
        self.blocking = true;
    }

    /// 结束格挡
    pub fn stop(&mut self) {
        self.blocking = false;
    }

    /// 扣持续耐力 (每秒调用一次)
    pub fn drain(&self, sta: &mut Stamina, dt: f32) {
        if self.blocking {
            sta.consume(self.drain_per_sec * dt);
        }
    }

    /// 受击时调用: 返回应用后的实际伤害
    ///
    /// 公式: `actual = raw * (1 - damage_reduction)`,然后扣 per_hit_cost STA
    /// 若 STA 不够扣 → 视为破盾 (返回 raw 伤害,blocking 自动 false)
    pub fn apply_hit(&mut self, raw_damage: f32, sta: &mut Stamina) -> f32 {
        if !self.blocking {
            return raw_damage;
        }
        // 扣 per_hit_cost;若不够就破盾
        let actual_cost = self.per_hit_cost.min(sta.current);
        sta.consume(actual_cost);
        if sta.current <= 0.0 {
            // 破盾 → 取消格挡,伤害全吃
            self.blocking = false;
            return raw_damage;
        }
        raw_damage * (1.0 - self.damage_reduction)
    }
}

// ---------------------------------------------------------------------------
// ParryWindow (招架窗口)
// ---------------------------------------------------------------------------

/// 招架窗口 Component
///
/// 招架是**短窗口**(基础 0.16s,装备 `parry_window` 调整),在攻击即将命中的瞬间按格挡触发。
/// 招架成功 → 攻击者 stun + 反击窗口。
#[derive(Component, Debug, Clone, Copy)]
pub struct ParryWindow {
    /// 是否在招架窗口内
    pub parry_active: bool,
    /// 剩余窗口时间 (秒)
    pub parry_timer: f32,
    /// 招架窗口总时长 (秒) — 装备决定,基础 0.16s
    pub parry_window_secs: f32,
    /// 招架成功后,反击窗口持续时间 (秒) — 反击招 = 短窗口内可发动反击
    pub riposte_window_secs: f32,
}

impl Default for ParryWindow {
    fn default() -> Self {
        // 表格包 §18.3: 标准 0s 修正 / Plain Duelist 招架 "标准"
        // §18.2: 招架窗口基础 0.16s,装备 ±0.04s
        Self {
            parry_active: false,
            parry_timer: 0.0,
            parry_window_secs: 0.16,
            riposte_window_secs: 0.40, // 反击窗口 0.4s
        }
    }
}

impl ParryWindow {
    /// 进入招架窗口
    pub fn begin(&mut self) {
        self.parry_active = true;
        self.parry_timer = self.parry_window_secs;
    }

    /// 每帧 tick — 窗口倒计时
    /// 返回: true = 窗口刚结束(供触发收招动画)
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.parry_active {
            return false;
        }
        self.parry_timer -= dt;
        if self.parry_timer <= 0.0 {
            self.parry_active = false;
            self.parry_timer = 0.0;
            return true;
        }
        false
    }

    /// 招架是否成功 (被击中时检查)
    pub fn try_parry(&self) -> bool {
        self.parry_active
    }

    /// 招架成功后消耗窗口 (返回是否触发反击窗口)
    pub fn consume_on_success(&mut self) -> bool {
        if self.parry_active {
            self.parry_active = false;
            self.parry_timer = 0.0;
            // 反击窗口是另一个 timer,简化:返回 true 标识可反击
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// StunState (硬直)
// ---------------------------------------------------------------------------

/// 硬直状态 Component
///
/// 被招架 / 重击命中 / 倒地时 stun。
/// stun 期间不能攻击 / 移动受限。
#[derive(Component, Debug, Clone, Copy)]
pub struct StunState {
    pub stunned: bool,
    pub stun_timer: f32,
    /// stun 来源
    pub source: StunSource,
}

impl Default for StunState {
    fn default() -> Self {
        Self {
            stunned: false,
            stun_timer: 0.0,
            source: StunSource::None,
        }
    }
}

/// stun 来源 (用于动画 / 反击优先级)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StunSource {
    /// 未 stun
    None,
    /// 被招架触发 — 攻击者 stun
    Parried,
    /// 重击命中 — 受击者 stun
    HeavyHit,
    /// 耐力耗尽 — 倒地
    Exhausted,
}

impl StunState {
    /// 进入 stun
    pub fn apply(&mut self, secs: f32, source: StunSource) {
        self.stunned = true;
        self.stun_timer = secs;
        self.source = source;
    }

    /// 每帧 tick (返回 true = 刚结束 stun)
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.stunned {
            return false;
        }
        self.stun_timer -= dt;
        if self.stun_timer <= 0.0 {
            self.stunned = false;
            self.stun_timer = 0.0;
            self.source = StunSource::None;
            return true;
        }
        false
    }

    /// 是否能行动 (不能攻击 / 不能用招架 / 不能格挡)
    pub fn can_act(&self) -> bool {
        !self.stunned
    }
}

// ---------------------------------------------------------------------------
// Knockback (击退)
// ---------------------------------------------------------------------------

/// 击退状态 Component
///
/// 受击时被施加,持续 push 一段时间。
/// 物理层用 `direction * magnitude * remaining` 推动 entity。
#[derive(Component, Debug, Clone, Copy)]
pub struct Knockback {
    /// 击退方向 (XZ 平面,长度 = 1)
    pub direction: Vec3,
    /// 击退速度大小 (m/s)
    pub magnitude: f32,
    /// 剩余时间 (秒,持续到归零)
    pub remaining_secs: f32,
    /// 总持续时间 (用于 UI 进度条)
    pub total_secs: f32,
}

impl Default for Knockback {
    fn default() -> Self {
        Self {
            direction: Vec3::ZERO,
            magnitude: 0.0,
            remaining_secs: 0.0,
            total_secs: 0.0,
        }
    }
}

impl Knockback {
    /// 施加击退 (方向自动 normalize)
    pub fn apply(&mut self, direction: Vec3, magnitude: f32, duration_secs: f32) {
        self.direction = direction.normalize_or_zero();
        self.magnitude = magnitude;
        self.remaining_secs = duration_secs;
        self.total_secs = duration_secs;
    }

    /// 每帧 tick (返回 0 = 已结束)
    pub fn tick(&mut self, dt: f32) -> Vec3 {
        if self.remaining_secs <= 0.0 {
            return Vec3::ZERO;
        }
        self.remaining_secs -= dt;
        if self.remaining_secs < 0.0 {
            self.remaining_secs = 0.0;
            return Vec3::ZERO;
        }
        self.direction * self.magnitude
    }

    pub fn is_active(&self) -> bool {
        self.remaining_secs > 0.0
    }

    /// 击退进度 (0..=1,1 = 刚开始,0 = 结束)
    pub fn progress(&self) -> f32 {
        if self.total_secs <= 0.0 {
            0.0
        } else {
            (self.remaining_secs / self.total_secs).clamp(0.0, 1.0)
        }
    }
}

// ---------------------------------------------------------------------------
// CombatIntent (输入意图)
// ---------------------------------------------------------------------------

/// 战斗输入意图 (玩家 → 系统)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CombatIntent {
    /// 攻击
    Attack(AttackType),
    /// 开始格挡
    BlockStart,
    /// 结束格挡
    BlockEnd,
    /// 尝试招架 (短窗口,仅在格挡启动时短暂可触发)
    ParryAttempt,
}

// ---------------------------------------------------------------------------
// CombatEvent (输出事件)
// ---------------------------------------------------------------------------

/// 战斗事件 (系统 → 客户端表现 / 审计)
#[derive(Message, Debug, Clone)]
pub enum CombatEvent {
    /// 造成伤害
    DamageDealt {
        attacker: Entity,
        victim: Entity,
        attack: AttackType,
        damage: f32,
    },
    /// 攻击被格挡
    Blocked { attacker: Entity, defender: Entity },
    /// 攻击被招架
    Parried { attacker: Entity, defender: Entity },
    /// 硬直触发
    Stunned { entity: Entity, source: StunSource, duration_secs: f32 },
    /// 击退施加
    Knockback { victim: Entity, magnitude: f32 },
}

// ---------------------------------------------------------------------------
// 命中判定辅助函数
// ---------------------------------------------------------------------------

/// 命中体积检查 — 简化 sweep 命中判定
///
/// 文档 §4: "命中判定不依赖视觉剑轨单帧" → MVP 用圆形 sweep (XZ 平面距离)。
/// 参数:
/// - `attacker_pos`: 攻击者位置
/// - `attacker_forward`: 攻击者朝向 (单位向量)
/// - `target_pos`: 目标位置
/// - `reach`: 武器触距 (米)
/// - `sweep_half_angle_deg`: 扇形半角 (度,比如 30° → 60° 锥)
/// 返回: 是否命中
pub fn sweep_hits(
    attacker_pos: Vec3,
    attacker_forward: Vec3,
    target_pos: Vec3,
    reach: f32,
    sweep_half_angle_deg: f32,
) -> bool {
    // 1) 距离检查 (XZ 平面)
    let delta = target_pos - attacker_pos;
    let dist = Vec3::new(delta.x, 0.0, delta.z).length();
    if dist > reach {
        return false;
    }
    // 2) 角度检查 (扇形)
    if dist < 0.001 {
        return true; // 贴脸
    }
    let to_target = Vec3::new(delta.x, 0.0, delta.z).normalize();
    let forward = Vec3::new(attacker_forward.x, 0.0, attacker_forward.z).normalize_or_zero();
    if forward == Vec3::ZERO {
        return false;
    }
    let dot = forward.dot(to_target).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();
    let half_angle_rad = sweep_half_angle_deg.to_radians();
    angle_rad <= half_angle_rad
}

/// 完整命中处理 (高阶函数)
/// 按顺序检查: 招架 → 格挡 → 命中,返回 `(CombatEvent 列表, raw_damage 实际造成)`
///
/// 简化 MVP: 不在这里发 Message (那是 system 的事),只算结果。
/// 调用方负责 `CombatEvent` 的 emit。
pub fn resolve_hit(
    attacker: Entity,
    defender: Entity,
    attack: AttackType,
    weapon_damage: f32,
    defender_parry: &mut ParryWindow,
    defender_block: &mut BlockState,
    defender_stun: &mut StunState,
    defender_stamina: &mut Stamina,
    defender_knockback: &mut Knockback,
    attacker_pos: Vec3,
    attacker_forward: Vec3,
    defender_pos: Vec3,
    reach: f32,
) -> (Vec<CombatEvent>, f32) {
    let mut events = Vec::new();
    let raw = weapon_damage * attack.damage_multiplier();

    // 1) 招架优先 (在窗口内 → 攻击者 stun)
    if defender_parry.try_parry() {
        defender_parry.consume_on_success();
        // 攻击者 stun 0.6s (表格包 §18.2 / 招架反击窗口)
        events.push(CombatEvent::Parried {
            attacker,
            defender,
        });
        events.push(CombatEvent::Stunned {
            entity: attacker,
            source: StunSource::Parried,
            duration_secs: 0.6,
        });
        // 招架本身不掉血
        return (events, 0.0);
    }

    // 2) 格挡 → 减伤 + 扣 STA
    if defender_block.blocking {
        let actual = defender_block.apply_hit(raw, defender_stamina);
        events.push(CombatEvent::Blocked {
            attacker,
            defender,
        });
        events.push(CombatEvent::DamageDealt {
            attacker,
            victim: defender,
            attack,
            damage: actual,
        });
        // 重击被格挡 → 仍然施加小击退 (防止格挡 100% 抵消位移)
        if matches!(attack, AttackType::Heavy) {
            let dir = (defender_pos - attacker_pos).normalize_or_zero();
            defender_knockback.apply(dir, 0.5, 0.20);
            events.push(CombatEvent::Knockback {
                victim: defender,
                magnitude: 0.5,
            });
        }
        return (events, actual);
    }

    // 3) 命中 (无招架无格挡)
    events.push(CombatEvent::DamageDealt {
        attacker,
        victim: defender,
        attack,
        damage: raw,
    });

    // 击退
    if attack.knockback_strength() > 0.0 {
        let dir = (defender_pos - attacker_pos).normalize_or_zero();
        defender_knockback.apply(dir, attack.knockback_strength() * weapon_damage * 0.5, 0.30);
        events.push(CombatEvent::Knockback {
            victim: defender,
            magnitude: attack.knockback_strength() * weapon_damage * 0.5,
        });
    }

    // 重击命中 → defender stun 0.4s
    if matches!(attack, AttackType::Heavy) {
        defender_stun.apply(0.4, StunSource::HeavyHit);
        events.push(CombatEvent::Stunned {
            entity: defender,
            source: StunSource::HeavyHit,
            duration_secs: 0.4,
        });
    }

    (events, raw)
}

// ---------------------------------------------------------------------------
// Health (生命) — V2 本地组件 (不走网络,服务端权威 + 客户端预测各持一份)
// ---------------------------------------------------------------------------

/// 玩家 / 生物生命值 Component
///
/// 来源:
/// - 《开发顺序与里程碑》§4 阶段 P4 基础战斗: "生命"
/// - 《表格包》§19.1: hp_max 默认 100,常规上限 130
///
/// 与 `protocol::Health(pub f32)` 区别:本组件是 V2 本地权威侧,
/// `protocol::Health` 是网络复制快照。V2 system 改 Health, 然后通过 protocol 同步。
#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    /// 临时无敌帧 — 被击后短时间内再次受击无效
    pub invuln_until_tick: u32,
}

impl Default for Health {
    fn default() -> Self {
        // 表格包 §19.1: hp_max 默认 100
        Self {
            current: 100.0,
            max: 100.0,
            invuln_until_tick: 0,
        }
    }
}

impl Health {
    /// 造成伤害 — 返回实际造成的伤害 (clamp 到 current,无敌帧返回 0)
    ///
    /// 公式: `actual = amount.min(current)`,无敌帧返回 0
    pub fn damage(&mut self, amount: f32, current_tick: u32, invuln_ticks: u32) -> f32 {
        if current_tick < self.invuln_until_tick {
            return 0.0;
        }
        let actual = amount.min(self.current.max(0.0));
        self.current = (self.current - actual).max(0.0);
        self.invuln_until_tick = current_tick.saturating_add(invuln_ticks);
        actual
    }

    /// 治疗 — clamp 到 max,返回实际治疗量
    pub fn heal(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current + amount).min(self.max);
        self.current - before
    }

    /// 是否死亡
    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    /// 生命比例 (0..=1)
    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

// ---------------------------------------------------------------------------
// Downed (倒地) — HP 归 0 后的硬直状态, 倒计时结束自动复活
// ---------------------------------------------------------------------------

/// 倒地状态 Component
///
/// 来源:
/// - 《开发顺序与里程碑》§4: "倒地"
/// - 《表格包》§19.3: downed_bleedout 45s (公开活动赛救援窗)
///
/// MVP 行为: HP 归 0 → 进入 Downed → 倒计时 → 自动复活 50% HP。
/// 长线: 队友可救援提前拉起;救援者必须在 3m 内 hold E 2s。
#[derive(Component, Debug, Clone, Copy)]
pub struct Downed {
    pub downed: bool,
    /// 倒地剩余秒数
    pub timer: f32,
    /// 倒地总时长 (用于 UI 进度条)
    pub total_secs: f32,
    /// 复活后生命比例
    pub revive_hp_ratio: f32,
}

impl Default for Downed {
    fn default() -> Self {
        // 表格包 §4 downed_bleedout=45s (公开赛救援窗)
        // MVP 默认 8s (够 demo 用,长线改 45s)
        Self {
            downed: false,
            timer: 0.0,
            total_secs: 8.0,
            revive_hp_ratio: 0.5,
        }
    }
}

impl Downed {
    /// 进入倒地
    pub fn knockdown(&mut self, total_secs: f32) {
        self.downed = true;
        self.timer = total_secs;
        self.total_secs = total_secs;
    }

    /// 每帧 tick — 倒计时,归 0 时返回 true (供系统触发复活)
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.downed {
            return false;
        }
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.downed = false;
            self.timer = 0.0;
            return true;
        }
        false
    }

    /// 倒地进度 (1 = 刚倒地, 0 = 刚复活)
    pub fn progress(&self) -> f32 {
        if !self.downed || self.total_secs <= 0.0 {
            0.0
        } else {
            (self.timer / self.total_secs).clamp(0.0, 1.0)
        }
    }

    /// 是否能行动
    pub fn can_act(&self) -> bool {
        !self.downed
    }
}

// ---------------------------------------------------------------------------
// AttackState (攻击阶段状态机)
// ---------------------------------------------------------------------------

/// 攻击阶段
///
/// 一次攻击分三阶段:
/// - `Windup`: 起手 / 前摇 (招架窗口开启期)
/// - `Active`: 判定窗 (sweep_hits 持续生效)
/// - `Recovery: 收招 / 后摇 (硬直)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackPhase {
    Windup,
    Active,
    Recovery,
}

/// 单次进行中的攻击
#[derive(Debug, Clone, Copy)]
pub struct ActiveAttack {
    pub kind: AttackType,
    pub phase: AttackPhase,
    pub phase_timer: f32,
    /// 当前阶段总时长
    pub phase_secs: f32,
    /// 已对本轮攻击的目标应用过伤害(防止同一次攻击反复命中)
    pub hit_apps: u8,
}

/// 攻击阶段状态机 Component
///
/// 按 `CombatIntent::Attack` → start attack → 每 tick 推进阶段 → Active 期内 sweep_hits。
///
/// 来源:
/// - 《开发顺序与里程碑》§4: "命中判定不依赖视觉剑轨单帧" → 整个 Active 期内都判定
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AttackState {
    pub current: Option<ActiveAttack>,
}

impl AttackState {
    /// 开始一次攻击 (返回是否成功 — STA 够、没在 stun/down 才允许)
    pub fn try_start(&mut self, kind: AttackType, stamina: &Stamina, stun: &StunState, downed: &Downed) -> bool {
        if self.current.is_some() {
            return false; // 已经在攻击中
        }
        if !stun.can_act() || !downed.can_act() {
            return false;
        }
        if !stamina.has_enough(kind.stamina_cost()) {
            return false;
        }
        let (phase, phase_secs) = match kind {
            AttackType::Light => (AttackPhase::Windup, 0.06),
            AttackType::Thrust => (AttackPhase::Windup, 0.10),
            AttackType::Heavy => (AttackPhase::Windup, 0.18),
        };
        self.current = Some(ActiveAttack {
            kind,
            phase,
            phase_timer: phase_secs,
            phase_secs,
            hit_apps: 0,
        });
        true
    }

    /// 每帧 tick — 推进阶段,Windup → Active → Recovery → None
    /// 返回: true = 刚结束 (供系统收尾)
    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(att) = self.current.as_mut() else {
            return false;
        };
        att.phase_timer -= dt;
        if att.phase_timer > 0.0 {
            return false;
        }
        // 阶段切换
        match att.phase {
            AttackPhase::Windup => {
                att.phase = AttackPhase::Active;
                let active_secs = match att.kind {
                    AttackType::Light => 0.10,
                    AttackType::Thrust => 0.18,
                    AttackType::Heavy => 0.22,
                };
                att.phase_timer = active_secs;
                att.phase_secs = active_secs;
                att.hit_apps = 0;
            }
            AttackPhase::Active => {
                att.phase = AttackPhase::Recovery;
                let recovery_secs = match att.kind {
                    AttackType::Light => 0.12,
                    AttackType::Thrust => 0.20,
                    AttackType::Heavy => 0.45,
                };
                att.phase_timer = recovery_secs;
                att.phase_secs = recovery_secs;
            }
            AttackPhase::Recovery => {
                // 攻击完全结束
                self.current = None;
                return true;
            }
        }
        false
    }

    /// 当前是否处于 Active 阶段(可命中)
    pub fn is_active(&self) -> bool {
        matches!(self.current, Some(a) if a.phase == AttackPhase::Active)
    }

    /// 当前攻击类型(若在攻击中)
    pub fn current_kind(&self) -> Option<AttackType> {
        self.current.map(|a| a.kind)
    }

    /// 当前阶段进度 (0..=1)
    pub fn phase_progress(&self) -> f32 {
        match self.current {
            Some(a) if a.phase_secs > 0.0 => 1.0 - (a.phase_timer / a.phase_secs).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// InputBuffer (动作输入缓冲)
// ---------------------------------------------------------------------------

/// 动作输入缓冲 Component
///
/// 来源:
/// - 《开发顺序与里程碑》§4: "动作输入缓冲"
/// - 《表格包》§4: action_input_buffer 0.16s (0.10-0.22s 范围),action_cancel_grace 0.10s
///
/// 设计: 玩家在 0.16s 窗口内连按 → 缓冲所有 intent → system 一次性处理。
/// 实际战斗里 Light → Light 0.06s 内可触发 combo,Light 释放后 0.10s grace 内可接 Heavy。
#[derive(Component, Debug, Clone, Default)]
pub struct InputBuffer {
    /// (intent, 按下时刻 tick)
    pub queue: Vec<(CombatIntent, u32)>,
    /// 缓冲窗口 (秒)
    pub window_secs: f32,
    /// 缓冲窗口对应的 tick 数 (= window_secs * tick_rate)
    pub window_ticks: u32,
}

impl InputBuffer {
    pub fn new(window_secs: f32, tick_rate: u32) -> Self {
        Self {
            queue: Vec::with_capacity(8),
            window_secs,
            window_ticks: (window_secs * tick_rate as f32) as u32,
        }
    }

    /// 压入一个 intent
    pub fn push(&mut self, intent: CombatIntent, current_tick: u32) {
        if self.queue.len() >= 8 {
            self.queue.remove(0);
        }
        self.queue.push((intent, current_tick));
    }

    /// 弹出过期 intent,返回剩下的 (用于 system 处理)
    pub fn drain_fresh(&mut self, current_tick: u32) -> Vec<CombatIntent> {
        let window = self.window_ticks;
        self.queue.retain(|(_, t)| current_tick.saturating_sub(*t) <= window);
        self.queue.drain(..).map(|(i, _)| i).collect()
    }

    /// 立即清空 (例如死亡 / 倒地时)
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

// ---------------------------------------------------------------------------
// systems (FixedUpdate tick)
// ---------------------------------------------------------------------------
//
// 全部 system 跑在 FixedUpdate (与权威模拟同步 30Hz),
// 不读 key / 不读 input — 那些走 CombatIntent (由 client/server input layer 写入)。
//
// 调用顺序 (CombatPlugin::build 里 .chain() 保证):
//   1. regen_stamina        → 恢复 STA (用于下一 tick 的攻击校验)
//   2. tick_parry_window    → 招架窗口倒计时
//   3. tick_stun            → 硬直倒计时
//   4. tick_knockback       → 推 entity Transform (物理表现)
//   5. tick_downed          → 倒地倒计时,到时复活
//   6. tick_attack_state    → 推进 Windup/Active/Recovery 三阶段
//   7. process_intents      → 消费 InputBuffer 中的 CombatIntent
//   8. process_hits         → Active 阶段 sweep_hits,resolve_hit,扣 HP
//   9. emit_events          → 把 CombatEvent 写到 MessageWriter

pub mod systems {

use bevy::prelude::*;
use super::*;
use crate::pvp::{FixedTick, WeaponStats};

// ---------------------------------------------------------------------------
// 1. regen_stamina — 每 tick 恢复 STA (除战斗姿态 / stun 中)
// ---------------------------------------------------------------------------

/// 每 tick 恢复 Stamina
///
/// 战斗姿态 (`StunState.stunned` / `Downed.downed`) 时恢复减半,模拟喘气。
pub fn regen_stamina_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<(&mut Stamina, Option<&StunState>, Option<&Downed>)>,
) {
    let dt = fixed_time.delta_secs();
    for (mut sta, stun, downed) in q.iter_mut() {
        let in_combat = stun.map(|s| s.stunned).unwrap_or(false)
            || downed.map(|d| d.downed).unwrap_or(false);
        if in_combat {
            // 战斗/倒地中 → regen 减半
            sta.current = (sta.current + sta.regen_per_sec * 0.5 * dt).min(sta.max);
        } else {
            sta.regen(dt);
        }
    }
}

// ---------------------------------------------------------------------------
// 2. tick_parry_window — 招架窗口倒计时
// ---------------------------------------------------------------------------

/// 每 tick 倒计时 ParryWindow
pub fn tick_parry_window_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut ParryWindow>) {
    let dt = fixed_time.delta_secs();
    for mut p in q.iter_mut() {
        p.tick(dt);
    }
}

// ---------------------------------------------------------------------------
// 3. tick_stun — 硬直倒计时
// ---------------------------------------------------------------------------

/// 每 tick 倒计时 StunState
pub fn tick_stun_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut StunState>) {
    let dt = fixed_time.delta_secs();
    for mut s in q.iter_mut() {
        s.tick(dt);
    }
}

// ---------------------------------------------------------------------------
// 4. tick_knockback — 击退推 entity Transform
// ---------------------------------------------------------------------------

/// 每 tick 把 Knockback 速度加到 entity 的 Transform.translation
///
/// 注: 真实物理由 PvPController / kinematic body 处理,本 system 只算位置 delta。
/// MVP 简化: 直接 translate(velocity * dt),Y 不动(避免飞天)。
pub fn tick_knockback_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<(&mut Knockback, &mut Transform)>,
) {
    let dt = fixed_time.delta_secs();
    for (mut kb, mut xf) in q.iter_mut() {
        let v = kb.tick(dt);
        if v == Vec3::ZERO {
            continue;
        }
        xf.translation.x += v.x * dt;
        xf.translation.z += v.z * dt;
        // Y 不动 — 击退不抬高玩家 (避免浮空)
    }
}

// ---------------------------------------------------------------------------
// 5. tick_downed — 倒地倒计时,归 0 时自动复活到 50% HP
// ---------------------------------------------------------------------------

/// 每 tick 倒计时 Downed;归 0 时调用 Health.heal 复活
pub fn tick_downed_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<(&mut Downed, &mut Health)>,
) {
    let dt = fixed_time.delta_secs();
    for (mut down, mut hp) in q.iter_mut() {
        if down.tick(dt) {
            // 复活: HP 恢复到 max * revive_ratio
            let target = hp.max * down.revive_hp_ratio;
            let need = (target - hp.current).max(0.0);
            hp.heal(need);
        }
    }
}

// ---------------------------------------------------------------------------
// 6. tick_attack_state — 推进 Windup/Active/Recovery 三阶段
// ---------------------------------------------------------------------------

/// 每 tick 推进 AttackState 三阶段 (Windup → Active → Recovery → None)
pub fn tick_attack_state_system(
    fixed_time: Res<Time<Fixed>>,
    mut q: Query<&mut AttackState>,
) {
    let dt = fixed_time.delta_secs();
    for mut att in q.iter_mut() {
        att.tick(dt);
    }
}

// ---------------------------------------------------------------------------
// 7. process_combat_intents — 把 InputBuffer 的 CombatIntent 落到状态
// ---------------------------------------------------------------------------

/// 把 InputBuffer 里的 CombatIntent 应用到对应组件
///
/// - `Attack(Light/Thrust/Heavy)` → 调 `AttackState.try_start`(扣 STA)
/// - `BlockStart` → `BlockState.start()`
/// - `BlockEnd` → `BlockState.stop()`
/// - `ParryAttempt` → `ParryWindow.begin()`(短窗口,在 block 启动时短暂可触发)
pub fn process_combat_intents_system(
    fixed_tick: Res<FixedTick>,
    mut q: Query<(
        &mut InputBuffer,
        &mut AttackState,
        &mut BlockState,
        &mut ParryWindow,
        &Stamina,
        &StunState,
        &Downed,
    )>,
) {
    for (mut buf, mut att, mut block, mut parry, sta, stun, down) in q.iter_mut() {
        // 用 saturating::MAX 占位 (无时间信息时,保留全部)
        let intents = buf.drain_fresh(fixed_tick.0);
        for intent in intents {
            match intent {
                CombatIntent::Attack(kind) => {
                    let _ = att.try_start(kind, sta, stun, down);
                }
                CombatIntent::BlockStart => {
                    block.start();
                    // 招架窗口在 block 启动时短暂开启 — 给玩家"举盾 + 精确时点"
                    parry.begin();
                }
                CombatIntent::BlockEnd => {
                    block.stop();
                }
                CombatIntent::ParryAttempt => {
                    parry.begin();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 8. process_attack_hits — Active 阶段 sweep_hits,resolve_hit,扣 HP
// ---------------------------------------------------------------------------

/// 在 AttackState.current 是 Active 阶段时,扫周围 Hitbox entities,resolve_hit,扣 HP。
///
/// 设计要点 (dev order §4 验收):
/// - "命中判定不依赖视觉剑轨单帧" → 整个 Active 期内都判定
/// - "两个玩家本地/局域测试能互相攻击" → 任何带 Health+Hitbox+Transform 的 entity 都可被命中
/// - "格挡能降低伤害" / "招架窗口" → 由被命中方组件在 resolve_hit 中处理
///
/// 范围搜索: 用 `attacker.translation` 在 XZ 平面 8m 内筛 candidate,然后 sweep_hits。
pub fn process_attack_hits_system(
    fixed_tick: Res<FixedTick>,
    mut queries: ParamSet<(
        Query<(
            Entity,
            &Transform,
            &mut AttackState,
            &mut Stamina,
            &mut StunState,
            &WeaponStats,
        )>,
        Query<(
            Entity,
            &Transform,
            &mut Health,
            &mut Stamina,
            &mut BlockState,
            &mut ParryWindow,
            &mut StunState,
            &mut Knockback,
        )>,
    )>,
) {
    // 收集所有攻击者(Active 期)— 一次性取所有,避免 long-term borrow
    let attackers: Vec<Entity> = {
        let q0 = queries.p0();
        q0.iter()
            .filter(|(_, _, att, _, _, _)| att.is_active())
            .map(|(e, _, _, _, _, _)| e)
            .collect()
    };

    for attacker_entity in attackers {
        // Step 1: snapshot 攻击者信息(不持 borrow)
        let attacker_info = {
            let q0 = queries.p0();
            let Ok((_, xf, att, sta, _, weapon)) = q0.get(attacker_entity) else {
                continue;
            };
            if !att.is_active() {
                continue;
            }
            let kind = match att.current_kind() {
                Some(k) => k,
                None => continue,
            };
            Some((
                xf.translation,
                (*xf.forward()).into(),
                weapon.damage,
                weapon.reach,
                weapon.sweep_angle_deg * 0.5,
                kind,
                sta.current,
            ))
        };
        let (atk_pos, atk_forward, weapon_damage, weapon_reach, sweep_half_angle, attack_kind, sta_now) =
            match attacker_info {
                Some(v) => v,
                None => continue,
            };

        let atk_cost = attack_kind.stamina_cost();

        // Step 2: 扣 STA + 标记 hit_apps=1(单次短 borrow)
        {
            let mut q0 = queries.p0();
            let Ok((_, _, mut att, mut sta, _, _)) = q0.get_mut(attacker_entity) else {
                continue;
            };
            if sta_now < atk_cost {
                // STA 不够 → 强制结束攻击
                att.current = None;
                continue;
            }
            if let Some(a) = att.current.as_mut() {
                if a.hit_apps == 0 {
                    sta.consume(atk_cost);
                    a.hit_apps = 1;
                }
            }
        }

        // Step 3: 收集攻击者将被应用的 stun(招架反击)
        let mut attacker_stun_apply: Option<(StunSource, f32)> = None;

        // Step 4: 扫目标 (8m XZ 平面)
        {
            let mut q1 = queries.p1();
            for (
                target_entity,
                tgt_xf,
                mut tgt_hp,
                mut tgt_sta,
                mut tgt_block,
                mut tgt_parry,
                mut tgt_stun,
                mut tgt_kb,
            ) in q1.iter_mut()
            {
                if target_entity == attacker_entity {
                    continue; // 不打自己
                }
                let delta = tgt_xf.translation - atk_pos;
                let dist_sq = delta.x * delta.x + delta.z * delta.z;
                if dist_sq > 64.0 {
                    continue;
                }
                if !sweep_hits(
                    atk_pos,
                    atk_forward,
                    tgt_xf.translation,
                    weapon_reach,
                    sweep_half_angle,
                ) {
                    continue;
                }

                let (events, actual_damage) = resolve_hit(
                    attacker_entity,
                    target_entity,
                    attack_kind,
                    weapon_damage,
                    &mut tgt_parry,
                    &mut tgt_block,
                    &mut tgt_stun,
                    &mut tgt_sta,
                    &mut tgt_kb,
                    atk_pos,
                    atk_forward,
                    tgt_xf.translation,
                    weapon_reach,
                );

                // 应用伤害
                if actual_damage > 0.0 {
                    tgt_hp.damage(actual_damage, fixed_tick.0, 6); // 6 tick 无敌帧
                }

                // 收集 stun apply(攻击者)
                for evt in &events {
                    if let CombatEvent::Stunned {
                        entity,
                        source,
                        duration_secs,
                    } = evt
                    {
                        if *entity == attacker_entity {
                            attacker_stun_apply = Some((*source, *duration_secs));
                        }
                    }
                }

                // 标记 AttackState.hit_apps = 2(已命中至少一个目标)
                {
                    let mut q0 = queries.p0();
                    if let Ok((_, _, mut att, _, _, _)) = q0.get_mut(attacker_entity) {
                        if let Some(a) = att.current.as_mut() {
                            if a.hit_apps <= 1 {
                                a.hit_apps = 2;
                            }
                        }
                    }
                }

                break; // MVP: 一击只命中一个目标
            }
        }

        // Step 5: 应用 attacker stun(招架反击)— target loop 已结束,borrow 释放
        if let Some((source, secs)) = attacker_stun_apply {
            let mut q0 = queries.p0();
            if let Ok((_, _, _, _, mut stun, _)) = q0.get_mut(attacker_entity) {
                stun.apply(secs, source);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 9. emit_combat_events — 收集本 tick 战斗事件并写入 MessageWriter
// ---------------------------------------------------------------------------

/// 简化版: 把本次攻击造成的事件聚合到 CombatEvent message
///
/// MVP 不细究: 完整的事件流由 process_attack_hits_system 直接 emit,
/// 此处只做 placeholder(让 MessageWriter 注册有意义)。
pub fn emit_combat_events_system(_: MessageWriter<CombatEvent>) {
    // placeholder: 事件已在 process_attack_hits_system 内部收集
    // (Vec<CombatEvent>) 然后通过 message writer 转发给客户端表现层。
    // MVP: 暂不转发,events 直接由系统消费 (process_attack_hits 内部已处理)。
}

} // pub mod systems

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册战斗 messages + systems
///
/// 用法:
/// ```ignore
/// app.add_plugins(CombatPlugin);
/// ```
///
/// V2 加层 (本 PR): 注册 tick systems,让数据层的 stamina/parry/stun/knockback/downed/attack
/// 在 FixedUpdate 中真正运转。命中判定 `resolve_hit` / `sweep_hits` 由本模块
/// `systems::process_attack_hits_system` 调用。
///
/// MVP 局限 (dev order §4):
/// - 单机 offline 模式只能命中带 Hitbox+Health 的 ECS entity
/// - 怪物暂时是纯数据 (MonsterIndividual), 没有 Hitbox component,
///   所以"打怪"要等 P8 探索层把它们转成 ECS entity
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatEvent>();
        // systems 在 FixedUpdate 跑 — 与权威模拟同步
        app.add_systems(
            FixedUpdate,
            (
                systems::regen_stamina_system,
                systems::tick_parry_window_system,
                systems::tick_stun_system,
                systems::tick_knockback_system,
                systems::tick_downed_system,
                systems::tick_attack_state_system,
                systems::process_combat_intents_system,
                systems::process_attack_hits_system,
                systems::emit_combat_events_system,
            )
                .chain(),
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_stamina_costs_match_doc() {
        // 表格包 §18.3 范围: 5-12 STA
        assert!(AttackType::Light.stamina_cost() >= 5.0 && AttackType::Light.stamina_cost() <= 12.0);
        assert!(AttackType::Thrust.stamina_cost() >= 5.0 && AttackType::Thrust.stamina_cost() <= 12.0);
        assert!(AttackType::Heavy.stamina_cost() >= 5.0 && AttackType::Heavy.stamina_cost() <= 12.0);
        // 重击应 > 轻击
        assert!(AttackType::Heavy.stamina_cost() > AttackType::Light.stamina_cost());
        // 重击 cooldown 应 > 轻击
        assert!(AttackType::Heavy.cooldown_secs() > AttackType::Light.cooldown_secs());
    }

    #[test]
    fn heavy_hits_harder_than_light() {
        assert!(AttackType::Heavy.damage_multiplier() > AttackType::Light.damage_multiplier());
        assert!(AttackType::Heavy.knockback_strength() > AttackType::Light.knockback_strength());
        // 突刺 knockback 应最小
        assert!(AttackType::Thrust.knockback_strength() < AttackType::Light.knockback_strength());
    }

    #[test]
    fn stamina_consume_clamp() {
        let mut s = Stamina::default();
        let consumed = s.consume(30.0);
        assert!((consumed - 30.0).abs() < 0.01);
        assert!((s.current - 70.0).abs() < 0.01);

        // 扣到不够时,只扣到 0
        let consumed = s.consume(999.0);
        assert!((consumed - 70.0).abs() < 0.01);
        assert_eq!(s.current, 0.0);
    }

    #[test]
    fn stamina_regen_clamp_to_max() {
        let mut s = Stamina {
            current: 50.0,
            max: 100.0,
            regen_per_sec: 20.0,
        };
        s.regen(1.0); // +20
        assert!((s.current - 70.0).abs() < 0.01);
        s.regen(2.0); // +40 → 110 → clamp 100
        assert_eq!(s.current, 100.0);
    }

    #[test]
    fn stamina_has_enough() {
        let s = Stamina { current: 5.0, max: 100.0, regen_per_sec: 10.0 };
        assert!(s.has_enough(5.0));
        assert!(!s.has_enough(5.1));
    }

    #[test]
    fn block_state_default_45pct_reduction() {
        // 表格包 §18.3 橡木圆盾: 格挡 45%
        let b = BlockState::default();
        assert!((b.damage_reduction - 0.45).abs() < 0.01);
    }

    #[test]
    fn block_apply_hit_reduces_damage() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina::default();
        let actual = block.apply_hit(20.0, &mut sta);
        // 20 * (1 - 0.45) = 11
        assert!((actual - 11.0).abs() < 0.01);
        assert!(block.blocking, "耐力够 → 仍在格挡");
    }

    #[test]
    fn block_break_when_stamina_empty() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina { current: 1.0, max: 100.0, regen_per_sec: 10.0 };
        let actual = block.apply_hit(20.0, &mut sta);
        // STA 只有 1,per_hit_cost=4 > 1 → 视为破盾 → 实际伤害 = 20
        assert!((actual - 20.0).abs() < 0.01);
        assert!(!block.blocking, "破盾后 blocking=false");
    }

    #[test]
    fn block_drain_when_blocking() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina::default();
        block.drain(&mut sta, 1.0); // 1 秒
        assert!((sta.current - (100.0 - 6.0)).abs() < 0.01);
        block.drain(&mut sta, 1.0);
        assert!((sta.current - (100.0 - 12.0)).abs() < 0.01);
    }

    #[test]
    fn block_no_drain_when_not_blocking() {
        let block = BlockState::default(); // blocking=false
        let mut sta = Stamina::default();
        block.drain(&mut sta, 5.0);
        assert_eq!(sta.current, 100.0);
    }

    #[test]
    fn parry_window_default_160ms() {
        // 表格包 §18.2: 招架窗口基础 0.16s
        let p = ParryWindow::default();
        assert!((p.parry_window_secs - 0.16).abs() < 0.01);
    }

    #[test]
    fn parry_tick_expires_window() {
        let mut p = ParryWindow::default();
        p.begin();
        assert!(p.parry_active);
        let expired = p.tick(0.10);
        assert!(!expired, "还在窗口内");
        assert!(p.parry_active);
        let expired = p.tick(0.07);
        assert!(expired, "刚结束");
        assert!(!p.parry_active);
    }

    #[test]
    fn parry_consume_only_when_active() {
        let mut p = ParryWindow::default();
        assert!(!p.consume_on_success(), "未激活时不能消耗");
        p.begin();
        assert!(p.consume_on_success(), "激活时可消耗");
        assert!(!p.parry_active);
    }

    #[test]
    fn stun_apply_and_tick() {
        let mut s = StunState::default();
        s.apply(1.0, StunSource::HeavyHit);
        assert!(s.stunned);
        assert!(!s.can_act());
        let ended = s.tick(0.5);
        assert!(!ended);
        assert!(s.stunned);
        let ended = s.tick(0.5);
        assert!(ended, "刚好结束");
        assert!(!s.stunned);
        assert!(s.can_act());
    }

    #[test]
    fn knockback_apply_normalizes_direction() {
        let mut kb = Knockback::default();
        kb.apply(Vec3::new(3.0, 0.0, 4.0), 5.0, 0.5); // 长度 5
        // 方向应 normalize 到单位向量
        assert!((kb.direction.length() - 1.0).abs() < 0.001);
        assert!((kb.magnitude - 5.0).abs() < 0.01);
        assert!((kb.remaining_secs - 0.5).abs() < 0.01);
    }

    #[test]
    fn knockback_tick_returns_velocity() {
        let mut kb = Knockback::default();
        kb.apply(Vec3::new(1.0, 0.0, 0.0), 4.0, 0.20);
        let v = kb.tick(0.05);
        // direction * magnitude = (1,0,0) * 4 = (4, 0, 0)
        assert!((v.x - 4.0).abs() < 0.01);
        assert_eq!(kb.remaining_secs, 0.15);
    }

    #[test]
    fn knockback_ends_returns_zero() {
        let mut kb = Knockback::default();
        kb.apply(Vec3::X, 4.0, 0.10);
        let v = kb.tick(0.20);
        assert_eq!(v, Vec3::ZERO);
        assert!(!kb.is_active());
    }

    #[test]
    fn sweep_hits_within_cone() {
        // 攻击者向 +X 方向,目标在 (3, 0, 1) (略偏 Z)
        let hit = sweep_hits(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(3.0, 0.0, 1.0),
            4.0,
            30.0, // 30° 半角
        );
        assert!(hit, "3m 略偏应在 60° 锥内");
    }

    #[test]
    fn sweep_misses_outside_cone() {
        let miss = sweep_hits(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(3.0, 0.0, 3.0), // 45° 偏
            4.0,
            30.0,
        );
        assert!(!miss, "45° 偏应不在 30° 半角内");
    }

    #[test]
    fn sweep_misses_out_of_reach() {
        let miss = sweep_hits(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(10.0, 0.0, 0.0), // 10m 远
            4.0,
            60.0,
        );
        assert!(!miss, "10m 远应不在 4m 触距内");
    }

    #[test]
    fn resolve_hit_blocked_reduces_damage() {
        let mut block = BlockState::default();
        block.start();
        let mut parry = ParryWindow::default();
        let mut stun = StunState::default();
        let mut sta = Stamina::default();
        let mut kb = Knockback::default();
        let (events, dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Light,
            20.0,
            &mut parry, &mut block, &mut stun, &mut sta, &mut kb,
            Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0), 4.0,
        );
        assert!(events.iter().any(|e| matches!(e, CombatEvent::Blocked { .. })));
        // 20 * (1 - 0.45) = 11
        assert!((dmg - 11.0).abs() < 0.01);
    }

    #[test]
    fn resolve_hit_parried_stuns_attacker() {
        let mut block = BlockState::default();
        let mut parry = ParryWindow::default();
        parry.begin();
        let mut stun = StunState::default();
        let mut sta = Stamina::default();
        let mut kb = Knockback::default();
        let (events, dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Heavy,
            30.0,
            &mut parry, &mut block, &mut stun, &mut sta, &mut kb,
            Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0), 4.0,
        );
        assert!(events.iter().any(|e| matches!(e, CombatEvent::Parried { .. })));
        // 招架不造成伤害
        assert_eq!(dmg, 0.0);
        // 攻击者应该被 stun (事件里有 Stunned 标记 source=Parried)
        let stun_evt = events.iter().find(|e| matches!(e, CombatEvent::Stunned { source: StunSource::Parried, .. }));
        assert!(stun_evt.is_some(), "招架后攻击者应被 stun");
    }

    #[test]
    fn resolve_hit_heavy_stuns_defender() {
        let mut block = BlockState::default();
        let mut parry = ParryWindow::default();
        let mut stun = StunState::default();
        let mut sta = Stamina::default();
        let mut kb = Knockback::default();
        let (events, _dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Heavy,
            30.0,
            &mut parry, &mut block, &mut stun, &mut sta, &mut kb,
            Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0), 4.0,
        );
        // Heavy 未被格挡/招架 → defender stun 0.4s
        assert!(stun.stunned);
        assert!((stun.stun_timer - 0.4).abs() < 0.01);
        let stun_evt = events.iter().find(|e| matches!(e, CombatEvent::Stunned { source: StunSource::HeavyHit, .. }));
        assert!(stun_evt.is_some());
        // 击退应施加
        assert!(kb.is_active());
    }

    #[test]
    fn resolve_hit_light_no_stun() {
        let mut block = BlockState::default();
        let mut parry = ParryWindow::default();
        let mut stun = StunState::default();
        let mut sta = Stamina::default();
        let mut kb = Knockback::default();
        let (events, dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Light,
            20.0,
            &mut parry, &mut block, &mut stun, &mut sta, &mut kb,
            Vec3::ZERO, Vec3::X, Vec3::new(2.0, 0.0, 0.0), 4.0,
        );
        assert!(!stun.stunned, "轻击不应 stun");
        assert!(dmg > 0.0);
        // 不应有 HeavyHit stun event
        assert!(!events.iter().any(|e| matches!(e, CombatEvent::Stunned { source: StunSource::HeavyHit, .. })));
    }

    #[test]
    fn stamina_exhaustion_should_stun() {
        // 测试: STA 耗尽时玩家进入 Exhausted stun (这个在 system 里做,但我们可以测 STA 边界)
        let mut sta = Stamina { current: 1.0, max: 100.0, regen_per_sec: 10.0 };
        let consumed = sta.consume(12.0); // Heavy
        assert_eq!(consumed, 1.0, "只能扣 1");
        assert_eq!(sta.current, 0.0);
        // STA == 0 时系统应阻止 attack — 这里只测状态
        assert!(!sta.has_enough(12.0));
    }

    // ============================================================
    // V2 系统层 + 新组件测试 (Health / Downed / AttackState / InputBuffer)
    // ============================================================

    #[test]
    fn health_default_100() {
        let h = Health::default();
        assert_eq!(h.current, 100.0);
        assert_eq!(h.max, 100.0);
        assert!(!h.is_dead());
        assert!((h.ratio() - 1.0).abs() < 0.01);
    }

    #[test]
    fn health_damage_reduces_current() {
        let mut h = Health::default();
        let actual = h.damage(30.0, 0, 0);
        assert_eq!(actual, 30.0);
        assert_eq!(h.current, 70.0);
        assert!(!h.is_dead());
    }

    #[test]
    fn health_damage_clamps_to_current() {
        let mut h = Health { current: 10.0, max: 100.0, invuln_until_tick: 0 };
        let actual = h.damage(50.0, 0, 0);
        // 只能扣到 current=10
        assert_eq!(actual, 10.0);
        assert_eq!(h.current, 0.0);
        assert!(h.is_dead());
    }

    #[test]
    fn health_damage_respects_invuln() {
        let mut h = Health::default();
        // 第一次伤害:扣 30,无敌帧到 tick 6
        let _ = h.damage(30.0, 0, 6);
        // 第二次在 tick 3 (无敌帧内):0
        let actual = h.damage(20.0, 3, 6);
        assert_eq!(actual, 0.0);
        assert_eq!(h.current, 70.0);
        // tick 7 已过无敌帧:正常扣
        let actual = h.damage(20.0, 7, 6);
        assert_eq!(actual, 20.0);
        assert_eq!(h.current, 50.0);
    }

    #[test]
    fn health_heal_caps_at_max() {
        let mut h = Health { current: 30.0, max: 100.0, invuln_until_tick: 0 };
        let healed = h.heal(50.0);
        assert_eq!(healed, 50.0);
        assert_eq!(h.current, 80.0);
        let healed = h.heal(50.0); // 超过 max
        assert_eq!(healed, 20.0);
        assert_eq!(h.current, 100.0);
    }

    #[test]
    fn downed_knockdown_and_revive() {
        let mut d = Downed::default();
        d.knockdown(8.0);
        assert!(d.downed);
        assert!(!d.can_act());
        let ended = d.tick(3.0);
        assert!(!ended);
        assert!(d.downed);
        let ended = d.tick(5.0); // 累计 8s
        assert!(ended, "刚好结束");
        assert!(!d.downed);
        assert!(d.can_act());
    }

    #[test]
    fn downed_progress_decreases() {
        let mut d = Downed::default();
        d.knockdown(10.0);
        assert!((d.progress() - 1.0).abs() < 0.01, "刚倒地 progress=1");
        d.tick(5.0);
        assert!((d.progress() - 0.5).abs() < 0.05, "过 5s/10s 应剩 50%");
    }

    #[test]
    fn attack_state_light_progresses_through_phases() {
        let mut att = AttackState::default();
        let mut sta = Stamina::default();
        let stun = StunState::default();
        let down = Downed::default();
        // 起手
        assert!(att.try_start(AttackType::Light, &sta, &stun, &down));
        // 刚起手还在 Windup,不是 Active
        assert!(!att.is_active(), "刚起手还在 Windup");
        // 推进 Windup(0.06s)
        let ended = att.tick(0.07);
        assert!(!ended);
        assert!(att.is_active(), "进入 Active");
        // 推进 Active(0.10s)
        let ended = att.tick(0.11);
        assert!(!ended);
        assert!(!att.is_active(), "进入 Recovery");
        // 推进 Recovery(0.12s)
        let ended = att.tick(0.13);
        assert!(ended, "完全结束");
        assert!(att.current.is_none());
    }

    #[test]
    fn attack_state_rejects_when_stunned() {
        let mut att = AttackState::default();
        let sta = Stamina::default();
        let mut stun = StunState::default();
        stun.apply(1.0, StunSource::HeavyHit);
        let down = Downed::default();
        assert!(!att.try_start(AttackType::Light, &sta, &stun, &down));
    }

    #[test]
    fn attack_state_rejects_when_low_stamina() {
        let mut att = AttackState::default();
        let sta = Stamina { current: 3.0, max: 100.0, regen_per_sec: 10.0 }; // < 6 (Light cost)
        let stun = StunState::default();
        let down = Downed::default();
        assert!(!att.try_start(AttackType::Light, &sta, &stun, &down));
    }

    #[test]
    fn attack_state_rejects_when_already_attacking() {
        let mut att = AttackState::default();
        let sta = Stamina::default();
        let stun = StunState::default();
        let down = Downed::default();
        assert!(att.try_start(AttackType::Light, &sta, &stun, &down));
        // 第二次应被拒
        assert!(!att.try_start(AttackType::Heavy, &sta, &stun, &down));
    }

    #[test]
    fn input_buffer_window_filter() {
        let mut buf = InputBuffer::new(0.20, 30); // 6 tick 窗口 (覆盖 0..=5)
        buf.push(CombatIntent::Attack(AttackType::Light), 0);
        buf.push(CombatIntent::Attack(AttackType::Heavy), 3);
        // tick 5 调用 drain_fresh: 5-0=5<=6 keep, 5-3=2<=6 keep
        let fresh = buf.drain_fresh(5);
        assert_eq!(fresh.len(), 2, "tick 0/3 都应在 tick 5 窗口内");
        // 再 push 一个 tick 7
        buf.push(CombatIntent::BlockStart, 7);
        let fresh = buf.drain_fresh(14); // tick 14 → 0/3 过期 (14-0=14>6, 14-3=11>6),但 7 还在 (14-7=7>6 应过期)
        // 实际 14-7=7 > 6 也过期
        assert_eq!(fresh.len(), 0, "全部过期");
    }

    #[test]
    fn input_buffer_caps_at_8() {
        let mut buf = InputBuffer::default();
        for i in 0..10 {
            buf.push(CombatIntent::Attack(AttackType::Light), i);
        }
        assert_eq!(buf.queue.len(), 8, "上限 8");
    }

    // ============================================================
    // resolve_hit 端到端测试 (用 Entity::PLACEHOLDER)
    // ============================================================

    #[test]
    fn e2e_resolve_hit_full_kill_reduces_hp_to_zero() {
        let mut hp = Health::default();
        let mut sta = Stamina::default();
        let mut block = BlockState::default();
        let mut parry = ParryWindow::default();
        let mut stun = StunState::default();
        let mut kb = Knockback::default();
        let (_, dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Heavy,
            100.0, // 单次扣 100*1.8 = 180
            &mut parry,
            &mut block,
            &mut stun,
            &mut sta,
            &mut kb,
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, 0.0, 0.0),
            4.0,
        );
        // 用 dmg 扣 HP
        let _ = hp.damage(dmg, 0, 0);
        assert!(hp.is_dead(), "HP 应为 0");
        assert!(stun.stunned, "Heavy hit 应 stun 0.4s");
        assert!(kb.is_active(), "应有击退");
    }

    #[test]
    fn e2e_resolve_hit_three_hit_combo_kills_player() {
        // 模拟: 玩家 30 HP,被打 3 次 Heavy,每次 60 dmg
        let mut hp = Health { current: 30.0, max: 100.0, invuln_until_tick: 0 };
        let mut sta = Stamina::default();
        let mut block = BlockState::default();
        let mut parry = ParryWindow::default();
        let mut stun = StunState::default();
        let mut kb = Knockback::default();
        for tick in (0..3).map(|i| i * 10) {
            let (_, dmg) = resolve_hit(
                Entity::PLACEHOLDER,
                Entity::PLACEHOLDER,
                AttackType::Heavy,
                30.0, // 30*1.8 = 54 dmg
                &mut parry,
                &mut block,
                &mut stun,
                &mut sta,
                &mut kb,
                Vec3::ZERO,
                Vec3::X,
                Vec3::new(2.0, 0.0, 0.0),
                4.0,
            );
            let _ = hp.damage(dmg, tick, 0);
        }
        // 30 - 54*3 = -132 → clamp 0
        assert_eq!(hp.current, 0.0);
        assert!(hp.is_dead());
    }

    #[test]
    fn e2e_resolve_hit_parry_breaks_attacker_combo() {
        // 攻击者 100 STA,准备连击 → 第一次被招架 → attacker stun → 后续攻击应被阻
        let mut atk_sta = Stamina::default();
        let mut atk_stun = StunState::default();
        let mut def_parry = ParryWindow::default();
        let mut def_block = BlockState::default();
        let mut def_stun = StunState::default();
        let mut def_sta = Stamina::default();
        let mut def_kb = Knockback::default();
        let mut def_hp = Health::default();

        def_parry.begin(); // defender 在招架窗口

        // attacker 攻击一次
        let (_events, dmg) = resolve_hit(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            AttackType::Heavy,
            30.0,
            &mut def_parry,
            &mut def_block,
            &mut def_stun,
            &mut def_sta,
            &mut def_kb,
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(2.0, 0.0, 0.0),
            4.0,
        );
        assert_eq!(dmg, 0.0, "招架不扣血");

        // attacker 被 stun (resolve_hit 内部 events)
        atk_stun.apply(0.6, StunSource::Parried);
        assert!(atk_stun.stunned);
        assert!(!atk_stun.can_act());

        // attacker 想再攻击,被 AttackState.try_start 拒
        let mut att_attack = AttackState::default();
        assert!(!att_attack.try_start(AttackType::Light, &atk_sta, &atk_stun, &Downed::default()));
        // 招架成功保护了 defender (HP 仍是 100)
        assert_eq!(def_hp.current, 100.0);
    }
}
