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
        attacker: u32,
        victim: u32,
        attack: AttackType,
        damage: f32,
    },
    /// 攻击被格挡
    Blocked { attacker: u32, defender: u32 },
    /// 攻击被招架
    Parried { attacker: u32, defender: u32 },
    /// 硬直触发
    Stunned { entity: u32, source: StunSource, duration_secs: f32 },
    /// 击退施加
    Knockback { victim: u32, magnitude: f32 },
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
    attacker_id: u32,
    defender_id: u32,
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
            attacker: attacker_id,
            defender: defender_id,
        });
        events.push(CombatEvent::Stunned {
            entity: attacker_id,
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
            attacker: attacker_id,
            defender: defender_id,
        });
        events.push(CombatEvent::DamageDealt {
            attacker: attacker_id,
            victim: defender_id,
            attack,
            damage: actual,
        });
        // 重击被格挡 → 仍然施加小击退 (防止格挡 100% 抵消位移)
        if matches!(attack, AttackType::Heavy) {
            let dir = (defender_pos - attacker_pos).normalize_or_zero();
            defender_knockback.apply(dir, 0.5, 0.20);
            events.push(CombatEvent::Knockback {
                victim: defender_id,
                magnitude: 0.5,
            });
        }
        return (events, actual);
    }

    // 3) 命中 (无招架无格挡)
    events.push(CombatEvent::DamageDealt {
        attacker: attacker_id,
        victim: defender_id,
        attack,
        damage: raw,
    });

    // 击退
    if attack.knockback_strength() > 0.0 {
        let dir = (defender_pos - attacker_pos).normalize_or_zero();
        defender_knockback.apply(dir, attack.knockback_strength() * weapon_damage * 0.5, 0.30);
        events.push(CombatEvent::Knockback {
            victim: defender_id,
            magnitude: attack.knockback_strength() * weapon_damage * 0.5,
        });
    }

    // 重击命中 → defender stun 0.4s
    if matches!(attack, AttackType::Heavy) {
        defender_stun.apply(0.4, StunSource::HeavyHit);
        events.push(CombatEvent::Stunned {
            entity: defender_id,
            source: StunSource::HeavyHit,
            duration_secs: 0.4,
        });
    }

    (events, raw)
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册战斗 messages
///
/// 用法:
/// ```ignore
/// app.add_plugins(CombatPlugin);
/// ```
///
/// 不挂 system (MVP),`Stamina` / `BlockState` / `ParryWindow` / `StunState` /
/// `Knockback` 由 scenario / worldgen 直接 spawn 在玩家 entity 上。
/// 命中判定 `resolve_hit` / `sweep_hits` 是公开函数,由 server task 拆时调用。
pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatEvent>();
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
            1, 2,
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
            1, 2,
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
            1, 2,
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
            1, 2,
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
}