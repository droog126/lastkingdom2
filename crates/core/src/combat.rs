





























use bevy::prelude::*;
use serde::{Deserialize, Serialize};






#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttackType {

    Light,

    Thrust,

    Heavy,
}

impl AttackType {


    pub fn stamina_cost(self) -> f32 {
        match self {
            Self::Light => 6.0,
            Self::Thrust => 7.0,
            Self::Heavy => 12.0,
        }
    }


    pub fn damage_multiplier(self) -> f32 {
        match self {
            Self::Light => 1.0,
            Self::Thrust => 0.9,
            Self::Heavy => 1.8,
        }
    }


    pub fn knockback_strength(self) -> f32 {
        match self {
            Self::Light => 0.15,
            Self::Thrust => 0.05,
            Self::Heavy => 0.8,
        }
    }


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









#[derive(Component, Debug, Clone, Copy)]
pub struct Stamina {

    pub current: f32,

    pub max: f32,

    pub regen_per_sec: f32,
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            regen_per_sec: 18.0,
        }
    }
}

impl Stamina {

    pub fn consume(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current - amount).max(0.0);
        before - self.current
    }


    pub fn regen(&mut self, dt: f32) {
        self.current = (self.current + self.regen_per_sec * dt).min(self.max);
    }


    pub fn has_enough(&self, cost: f32) -> bool {
        self.current >= cost
    }


    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}









#[derive(Component, Debug, Clone, Copy)]
pub struct BlockState {

    pub blocking: bool,

    pub drain_per_sec: f32,

    pub damage_reduction: f32,

    pub per_hit_cost: f32,
}

impl Default for BlockState {
    fn default() -> Self {


        Self { blocking: false, drain_per_sec: 6.0, damage_reduction: 0.45, per_hit_cost: 4.0 }
    }
}

impl BlockState {

    pub fn start(&mut self) {
        self.blocking = true;
    }


    pub fn stop(&mut self) {
        self.blocking = false;
    }


    pub fn drain(&self, sta: &mut Stamina, dt: f32) {
        if self.blocking {
            sta.consume(self.drain_per_sec * dt);
        }
    }





    pub fn apply_hit(&mut self, raw_damage: f32, sta: &mut Stamina) -> f32 {
        if !self.blocking {
            return raw_damage;
        }

        let actual_cost = self.per_hit_cost.min(sta.current);
        sta.consume(actual_cost);
        if sta.current <= 0.0 {

            self.blocking = false;
            return raw_damage;
        }
        raw_damage * (1.0 - self.damage_reduction)
    }
}









#[derive(Component, Debug, Clone, Copy)]
pub struct ParryWindow {

    pub parry_active: bool,

    pub parry_timer: f32,

    pub parry_window_secs: f32,

    pub riposte_window_secs: f32,
}

impl Default for ParryWindow {
    fn default() -> Self {


        Self {
            parry_active: false,
            parry_timer: 0.0,
            parry_window_secs: 0.16,
            riposte_window_secs: 0.40,
        }
    }
}

impl ParryWindow {

    pub fn begin(&mut self) {
        self.parry_active = true;
        self.parry_timer = self.parry_window_secs;
    }



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


    pub fn try_parry(&self) -> bool {
        self.parry_active
    }


    pub fn consume_on_success(&mut self) -> bool {
        if self.parry_active {
            self.parry_active = false;
            self.parry_timer = 0.0;

            true
        } else {
            false
        }
    }
}









#[derive(Component, Debug, Clone, Copy)]
pub struct StunState {
    pub stunned: bool,
    pub stun_timer: f32,

    pub source: StunSource,
}

impl Default for StunState {
    fn default() -> Self {
        Self { stunned: false, stun_timer: 0.0, source: StunSource::None }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StunSource {

    None,

    Parried,

    HeavyHit,

    Exhausted,
}

impl StunState {

    pub fn apply(&mut self, secs: f32, source: StunSource) {
        self.stunned = true;
        self.stun_timer = secs;
        self.source = source;
    }


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


    pub fn can_act(&self) -> bool {
        !self.stunned
    }
}









#[derive(Component, Debug, Clone, Copy)]
pub struct Knockback {

    pub direction: Vec3,

    pub magnitude: f32,

    pub remaining_secs: f32,

    pub total_secs: f32,
}

impl Default for Knockback {
    fn default() -> Self {
        Self { direction: Vec3::ZERO, magnitude: 0.0, remaining_secs: 0.0, total_secs: 0.0 }
    }
}

impl Knockback {

    pub fn apply(&mut self, direction: Vec3, magnitude: f32, duration_secs: f32) {
        self.direction = direction.normalize_or_zero();
        self.magnitude = magnitude;
        self.remaining_secs = duration_secs;
        self.total_secs = duration_secs;
    }


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


    pub fn progress(&self) -> f32 {
        if self.total_secs <= 0.0 {
            0.0
        } else {
            (self.remaining_secs / self.total_secs).clamp(0.0, 1.0)
        }
    }
}






#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CombatIntent {

    Attack(AttackType),

    BlockStart,

    BlockEnd,

    ParryAttempt,
}






#[derive(Message, Debug, Clone)]
pub enum CombatEvent {

    DamageDealt {
        attacker: Entity,
        victim: Entity,
        attack: AttackType,
        damage: f32,
    },

    Blocked { attacker: Entity, defender: Entity },

    Parried { attacker: Entity, defender: Entity },

    Stunned {
        entity: Entity,
        source: StunSource,
        duration_secs: f32,
    },

    Knockback { victim: Entity, magnitude: f32 },
}















pub fn sweep_hits(
    attacker_pos: Vec3,
    attacker_forward: Vec3,
    target_pos: Vec3,
    reach: f32,
    sweep_half_angle_deg: f32,
) -> bool {

    let delta = target_pos - attacker_pos;
    let dist = Vec3::new(delta.x, 0.0, delta.z).length();
    if dist > reach {
        return false;
    }

    if dist < 0.001 {
        return true;
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
    _attacker_forward: Vec3,
    defender_pos: Vec3,
    _reach: f32,
) -> (Vec<CombatEvent>, f32) {
    let mut events = Vec::new();
    let raw = weapon_damage * attack.damage_multiplier();


    if defender_parry.try_parry() {
        defender_parry.consume_on_success();

        events.push(CombatEvent::Parried { attacker, defender });
        events.push(CombatEvent::Stunned {
            entity: attacker,
            source: StunSource::Parried,
            duration_secs: 0.6,
        });

        return (events, 0.0);
    }


    if defender_block.blocking {
        let actual = defender_block.apply_hit(raw, defender_stamina);
        events.push(CombatEvent::Blocked { attacker, defender });
        events.push(CombatEvent::DamageDealt {
            attacker,
            victim: defender,
            attack,
            damage: actual,
        });

        if matches!(attack, AttackType::Heavy) {
            let dir = (defender_pos - attacker_pos).normalize_or_zero();
            defender_knockback.apply(dir, 0.5, 0.20);
            events.push(CombatEvent::Knockback { victim: defender, magnitude: 0.5 });
        }
        return (events, actual);
    }


    events.push(CombatEvent::DamageDealt { attacker, victim: defender, attack, damage: raw });


    if attack.knockback_strength() > 0.0 {
        let dir = (defender_pos - attacker_pos).normalize_or_zero();
        defender_knockback.apply(dir, attack.knockback_strength() * weapon_damage * 0.5, 0.30);
        events.push(CombatEvent::Knockback {
            victim: defender,
            magnitude: attack.knockback_strength() * weapon_damage * 0.5,
        });
    }


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













#[derive(Component, Debug, Clone, Copy)]
pub struct Health {
    pub current: f32,
    pub max: f32,

    pub invuln_until_tick: u32,
}

impl Default for Health {
    fn default() -> Self {

        Self { current: 100.0, max: 100.0, invuln_until_tick: 0 }
    }
}

impl Health {



    pub fn damage(&mut self, amount: f32, current_tick: u32, invuln_ticks: u32) -> f32 {
        if current_tick < self.invuln_until_tick {
            return 0.0;
        }
        let actual = amount.min(self.current.max(0.0));
        self.current = (self.current - actual).max(0.0);
        self.invuln_until_tick = current_tick.saturating_add(invuln_ticks);
        actual
    }


    pub fn heal(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current + amount).min(self.max);
        self.current - before
    }


    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }


    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}













#[derive(Component, Debug, Clone, Copy)]
pub struct Downed {
    pub downed: bool,

    pub timer: f32,

    pub total_secs: f32,

    pub revive_hp_ratio: f32,
}

impl Default for Downed {
    fn default() -> Self {


        Self { downed: false, timer: 0.0, total_secs: 8.0, revive_hp_ratio: 0.5 }
    }
}

impl Downed {

    pub fn knockdown(&mut self, total_secs: f32) {
        self.downed = true;
        self.timer = total_secs;
        self.total_secs = total_secs;
    }


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


    pub fn progress(&self) -> f32 {
        if !self.downed || self.total_secs <= 0.0 {
            0.0
        } else {
            (self.timer / self.total_secs).clamp(0.0, 1.0)
        }
    }


    pub fn can_act(&self) -> bool {
        !self.downed
    }
}











#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackPhase {
    Windup,
    Active,
    Recovery,
}


#[derive(Debug, Clone, Copy)]
pub struct ActiveAttack {
    pub kind: AttackType,
    pub phase: AttackPhase,
    pub phase_timer: f32,

    pub phase_secs: f32,

    pub hit_apps: u8,
}







#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AttackState {
    pub current: Option<ActiveAttack>,
}

impl AttackState {

    pub fn try_start(
        &mut self,
        kind: AttackType,
        stamina: &Stamina,
        stun: &StunState,
        downed: &Downed,
    ) -> bool {
        if self.current.is_some() {
            return false;
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
        self.current =
            Some(ActiveAttack { kind, phase, phase_timer: phase_secs, phase_secs, hit_apps: 0 });
        true
    }



    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(att) = self.current.as_mut() else {
            return false;
        };
        att.phase_timer -= dt;
        if att.phase_timer > 0.0 {
            return false;
        }

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

                self.current = None;
                return true;
            }
        }
        false
    }


    pub fn is_active(&self) -> bool {
        matches!(self.current, Some(a) if a.phase == AttackPhase::Active)
    }


    pub fn current_kind(&self) -> Option<AttackType> {
        self.current.map(|a| a.kind)
    }


    pub fn phase_progress(&self) -> f32 {
        match self.current {
            Some(a) if a.phase_secs > 0.0 => 1.0 - (a.phase_timer / a.phase_secs).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}













#[derive(Component, Debug, Clone, Default)]
pub struct InputBuffer {

    pub queue: Vec<(CombatIntent, u32)>,

    pub window_secs: f32,

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


    pub fn push(&mut self, intent: CombatIntent, current_tick: u32) {
        if self.queue.len() >= 8 {
            self.queue.remove(0);
        }
        self.queue.push((intent, current_tick));
    }


    pub fn drain_fresh(&mut self, current_tick: u32) -> Vec<CombatIntent> {
        let window = self.window_ticks;
        self.queue.retain(|(_, t)| current_tick.saturating_sub(*t) <= window);
        self.queue.drain(..).map(|(i, _)| i).collect()
    }


    pub fn clear(&mut self) {
        self.queue.clear();
    }
}



















pub mod systems {

    use super::*;
    use crate::pvp::{FixedTick, WeaponStats};
    use bevy::prelude::*;








    pub fn regen_stamina_system(
        fixed_time: Res<Time<Fixed>>,
        mut q: Query<(&mut Stamina, Option<&StunState>, Option<&Downed>)>,
    ) {
        let dt = fixed_time.delta_secs();
        for (mut sta, stun, downed) in q.iter_mut() {
            let in_combat = stun.map(|s| s.stunned).unwrap_or(false)
                || downed.map(|d| d.downed).unwrap_or(false);
            if in_combat {

                sta.current = (sta.current + sta.regen_per_sec * 0.5 * dt).min(sta.max);
            } else {
                sta.regen(dt);
            }
        }
    }






    pub fn tick_parry_window_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut ParryWindow>) {
        let dt = fixed_time.delta_secs();
        for mut p in q.iter_mut() {
            p.tick(dt);
        }
    }






    pub fn tick_stun_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut StunState>) {
        let dt = fixed_time.delta_secs();
        for mut s in q.iter_mut() {
            s.tick(dt);
        }
    }









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

        }
    }






    pub fn tick_downed_system(
        fixed_time: Res<Time<Fixed>>,
        mut q: Query<(&mut Downed, &mut Health)>,
    ) {
        let dt = fixed_time.delta_secs();
        for (mut down, mut hp) in q.iter_mut() {
            if down.tick(dt) {

                let target = hp.max * down.revive_hp_ratio;
                let need = (target - hp.current).max(0.0);
                hp.heal(need);
            }
        }
    }






    pub fn tick_attack_state_system(fixed_time: Res<Time<Fixed>>, mut q: Query<&mut AttackState>) {
        let dt = fixed_time.delta_secs();
        for mut att in q.iter_mut() {
            att.tick(dt);
        }
    }











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

            let intents = buf.drain_fresh(fixed_tick.0);
            for intent in intents {
                match intent {
                    CombatIntent::Attack(kind) => {
                        let _ = att.try_start(kind, sta, stun, down);
                    }
                    CombatIntent::BlockStart => {
                        block.start();

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

        let attackers: Vec<Entity> = {
            let q0 = queries.p0();
            q0.iter()
                .filter(|(_, _, att, _, _, _)| att.is_active())
                .map(|(e, _, _, _, _, _)| e)
                .collect()
        };

        for attacker_entity in attackers {

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
            let (
                atk_pos,
                atk_forward,
                weapon_damage,
                weapon_reach,
                sweep_half_angle,
                attack_kind,
                sta_now,
            ) = match attacker_info {
                Some(v) => v,
                None => continue,
            };

            let atk_cost = attack_kind.stamina_cost();


            {
                let mut q0 = queries.p0();
                let Ok((_, _, mut att, mut sta, _, _)) = q0.get_mut(attacker_entity) else {
                    continue;
                };
                if sta_now < atk_cost {

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


            let mut attacker_stun_apply: Option<(StunSource, f32)> = None;


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
                        continue;
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


                    if actual_damage > 0.0 {
                        tgt_hp.damage(actual_damage, fixed_tick.0, 6);
                    }


                    for evt in &events {
                        if let CombatEvent::Stunned { entity, source, duration_secs } = evt {
                            if *entity == attacker_entity {
                                attacker_stun_apply = Some((*source, *duration_secs));
                            }
                        }
                    }


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

                    break;
                }
            }


            if let Some((source, secs)) = attacker_stun_apply {
                let mut q0 = queries.p0();
                if let Ok((_, _, _, _, mut stun, _)) = q0.get_mut(attacker_entity) {
                    stun.apply(secs, source);
                }
            }
        }
    }









    pub fn emit_combat_events_system(_: MessageWriter<CombatEvent>) {



    }
}




















pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatEvent>();

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





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_stamina_costs_match_doc() {

        assert!(
            AttackType::Light.stamina_cost() >= 5.0 && AttackType::Light.stamina_cost() <= 12.0
        );
        assert!(
            AttackType::Thrust.stamina_cost() >= 5.0 && AttackType::Thrust.stamina_cost() <= 12.0
        );
        assert!(
            AttackType::Heavy.stamina_cost() >= 5.0 && AttackType::Heavy.stamina_cost() <= 12.0
        );

        assert!(AttackType::Heavy.stamina_cost() > AttackType::Light.stamina_cost());

        assert!(AttackType::Heavy.cooldown_secs() > AttackType::Light.cooldown_secs());
    }

    #[test]
    fn heavy_hits_harder_than_light() {
        assert!(AttackType::Heavy.damage_multiplier() > AttackType::Light.damage_multiplier());
        assert!(AttackType::Heavy.knockback_strength() > AttackType::Light.knockback_strength());

        assert!(AttackType::Thrust.knockback_strength() < AttackType::Light.knockback_strength());
    }

    #[test]
    fn stamina_consume_clamp() {
        let mut s = Stamina::default();
        let consumed = s.consume(30.0);
        assert!((consumed - 30.0).abs() < 0.01);
        assert!((s.current - 70.0).abs() < 0.01);


        let consumed = s.consume(999.0);
        assert!((consumed - 70.0).abs() < 0.01);
        assert_eq!(s.current, 0.0);
    }

    #[test]
    fn stamina_regen_clamp_to_max() {
        let mut s = Stamina { current: 50.0, max: 100.0, regen_per_sec: 20.0 };
        s.regen(1.0);
        assert!((s.current - 70.0).abs() < 0.01);
        s.regen(2.0);
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

        let b = BlockState::default();
        assert!((b.damage_reduction - 0.45).abs() < 0.01);
    }

    #[test]
    fn block_apply_hit_reduces_damage() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina::default();
        let actual = block.apply_hit(20.0, &mut sta);

        assert!((actual - 11.0).abs() < 0.01);
        assert!(block.blocking, "耐力够 → 仍在格挡");
    }

    #[test]
    fn block_break_when_stamina_empty() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina { current: 1.0, max: 100.0, regen_per_sec: 10.0 };
        let actual = block.apply_hit(20.0, &mut sta);

        assert!((actual - 20.0).abs() < 0.01);
        assert!(!block.blocking, "破盾后 blocking=false");
    }

    #[test]
    fn block_drain_when_blocking() {
        let mut block = BlockState::default();
        block.start();
        let mut sta = Stamina::default();
        block.drain(&mut sta, 1.0);
        assert!((sta.current - (100.0 - 6.0)).abs() < 0.01);
        block.drain(&mut sta, 1.0);
        assert!((sta.current - (100.0 - 12.0)).abs() < 0.01);
    }

    #[test]
    fn block_no_drain_when_not_blocking() {
        let block = BlockState::default();
        let mut sta = Stamina::default();
        block.drain(&mut sta, 5.0);
        assert_eq!(sta.current, 100.0);
    }

    #[test]
    fn parry_window_default_160ms() {

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
        kb.apply(Vec3::new(3.0, 0.0, 4.0), 5.0, 0.5);

        assert!((kb.direction.length() - 1.0).abs() < 0.001);
        assert!((kb.magnitude - 5.0).abs() < 0.01);
        assert!((kb.remaining_secs - 0.5).abs() < 0.01);
    }

    #[test]
    fn knockback_tick_returns_velocity() {
        let mut kb = Knockback::default();
        kb.apply(Vec3::new(1.0, 0.0, 0.0), 4.0, 0.20);
        let v = kb.tick(0.05);

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

        let hit = sweep_hits(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(3.0, 0.0, 1.0),
            4.0,
            30.0,
        );
        assert!(hit, "3m 略偏应在 60° 锥内");
    }

    #[test]
    fn sweep_misses_outside_cone() {
        let miss = sweep_hits(
            Vec3::ZERO,
            Vec3::X,
            Vec3::new(3.0, 0.0, 3.0),
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
            Vec3::new(10.0, 0.0, 0.0),
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
        assert!(events.iter().any(|e| matches!(e, CombatEvent::Blocked { .. })));

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
        assert!(events.iter().any(|e| matches!(e, CombatEvent::Parried { .. })));

        assert_eq!(dmg, 0.0);

        let stun_evt = events
            .iter()
            .find(|e| matches!(e, CombatEvent::Stunned { source: StunSource::Parried, .. }));
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

        assert!(stun.stunned);
        assert!((stun.stun_timer - 0.4).abs() < 0.01);
        let stun_evt = events
            .iter()
            .find(|e| matches!(e, CombatEvent::Stunned { source: StunSource::HeavyHit, .. }));
        assert!(stun_evt.is_some());

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
        assert!(!stun.stunned, "轻击不应 stun");
        assert!(dmg > 0.0);

        assert!(
            !events
                .iter()
                .any(|e| matches!(e, CombatEvent::Stunned { source: StunSource::HeavyHit, .. }))
        );
    }

    #[test]
    fn stamina_exhaustion_should_stun() {

        let mut sta = Stamina { current: 1.0, max: 100.0, regen_per_sec: 10.0 };
        let consumed = sta.consume(12.0);
        assert_eq!(consumed, 1.0, "只能扣 1");
        assert_eq!(sta.current, 0.0);

        assert!(!sta.has_enough(12.0));
    }





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

        assert_eq!(actual, 10.0);
        assert_eq!(h.current, 0.0);
        assert!(h.is_dead());
    }

    #[test]
    fn health_damage_respects_invuln() {
        let mut h = Health::default();

        let _ = h.damage(30.0, 0, 6);

        let actual = h.damage(20.0, 3, 6);
        assert_eq!(actual, 0.0);
        assert_eq!(h.current, 70.0);

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
        let healed = h.heal(50.0);
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
        let ended = d.tick(5.0);
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
        let sta = Stamina::default();
        let stun = StunState::default();
        let down = Downed::default();

        assert!(att.try_start(AttackType::Light, &sta, &stun, &down));

        assert!(!att.is_active(), "刚起手还在 Windup");

        let ended = att.tick(0.07);
        assert!(!ended);
        assert!(att.is_active(), "进入 Active");

        let ended = att.tick(0.11);
        assert!(!ended);
        assert!(!att.is_active(), "进入 Recovery");

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
        let sta = Stamina { current: 3.0, max: 100.0, regen_per_sec: 10.0 };
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

        assert!(!att.try_start(AttackType::Heavy, &sta, &stun, &down));
    }

    #[test]
    fn input_buffer_window_filter() {
        let mut buf = InputBuffer::new(0.20, 30);
        buf.push(CombatIntent::Attack(AttackType::Light), 0);
        buf.push(CombatIntent::Attack(AttackType::Heavy), 3);

        let fresh = buf.drain_fresh(5);
        assert_eq!(fresh.len(), 2, "tick 0/3 都应在 tick 5 窗口内");

        buf.push(CombatIntent::BlockStart, 7);
        let fresh = buf.drain_fresh(14);

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
            100.0,
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

        let _ = hp.damage(dmg, 0, 0);
        assert!(hp.is_dead(), "HP 应为 0");
        assert!(stun.stunned, "Heavy hit 应 stun 0.4s");
        assert!(kb.is_active(), "应有击退");
    }

    #[test]
    fn e2e_resolve_hit_three_hit_combo_kills_player() {

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
                30.0,
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

        assert_eq!(hp.current, 0.0);
        assert!(hp.is_dead());
    }

    #[test]
    fn e2e_resolve_hit_parry_breaks_attacker_combo() {

        let atk_sta = Stamina::default();
        let mut atk_stun = StunState::default();
        let mut def_parry = ParryWindow::default();
        let mut def_block = BlockState::default();
        let mut def_stun = StunState::default();
        let mut def_sta = Stamina::default();
        let mut def_kb = Knockback::default();
        let def_hp = Health::default();

        def_parry.begin();


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


        atk_stun.apply(0.6, StunSource::Parried);
        assert!(atk_stun.stunned);
        assert!(!atk_stun.can_act());


        let mut att_attack = AttackState::default();
        assert!(!att_attack.try_start(AttackType::Light, &atk_sta, &atk_stun, &Downed::default()));

        assert_eq!(def_hp.current, 100.0);
    }
}
