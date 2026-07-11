//! Unit tests for the combat data, rules, and state machine.

use bevy::prelude::*;

use super::attack::{AttackState, AttackType, InputBuffer};
use super::events::{CombatEvent, CombatIntent};
use super::resolve::{resolve_hit, sweep_hits};
use super::state::{BlockState, Downed, Knockback, ParryWindow, Stamina, StunSource, StunState};
use crate::pvp::Health;

#[test]
fn attack_stamina_costs_match_doc() {
    assert!(AttackType::Light.stamina_cost() >= 5.0 && AttackType::Light.stamina_cost() <= 12.0);
    assert!(AttackType::Thrust.stamina_cost() >= 5.0 && AttackType::Thrust.stamina_cost() <= 12.0);
    assert!(AttackType::Heavy.stamina_cost() >= 5.0 && AttackType::Heavy.stamina_cost() <= 12.0);

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
    let mut s = Stamina {
        current: 50.0,
        max: 100.0,
        regen_per_sec: 20.0,
    };
    s.regen(1.0);
    assert!((s.current - 70.0).abs() < 0.01);
    s.regen(2.0);
    assert_eq!(s.current, 100.0);
}

#[test]
fn stamina_has_enough() {
    let s = Stamina {
        current: 5.0,
        max: 100.0,
        regen_per_sec: 10.0,
    };
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
    let mut sta = Stamina {
        current: 1.0,
        max: 100.0,
        regen_per_sec: 10.0,
    };
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
    let hit = sweep_hits(Vec3::ZERO, Vec3::X, Vec3::new(3.0, 0.0, 1.0), 4.0, 30.0);
    assert!(hit, "3m 略偏应在 60° 锥内");
}

#[test]
fn sweep_misses_outside_cone() {
    let miss = sweep_hits(Vec3::ZERO, Vec3::X, Vec3::new(3.0, 0.0, 3.0), 4.0, 30.0);
    assert!(!miss, "45° 偏应不在 30° 半角内");
}

#[test]
fn sweep_misses_out_of_reach() {
    let miss = sweep_hits(Vec3::ZERO, Vec3::X, Vec3::new(10.0, 0.0, 0.0), 4.0, 60.0);
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
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CombatEvent::Blocked { .. }))
    );

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
    assert!(
        events
            .iter()
            .any(|e| matches!(e, CombatEvent::Parried { .. }))
    );

    assert_eq!(dmg, 0.0);

    let stun_evt = events.iter().find(|e| {
        matches!(
            e,
            CombatEvent::Stunned {
                source: StunSource::Parried,
                ..
            }
        )
    });
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
    let stun_evt = events.iter().find(|e| {
        matches!(
            e,
            CombatEvent::Stunned {
                source: StunSource::HeavyHit,
                ..
            }
        )
    });
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

    assert!(!events.iter().any(|e| matches!(
        e,
        CombatEvent::Stunned {
            source: StunSource::HeavyHit,
            ..
        }
    )));
}

#[test]
fn stamina_exhaustion_should_stun() {
    let mut sta = Stamina {
        current: 1.0,
        max: 100.0,
        regen_per_sec: 10.0,
    };
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
    let mut h = Health {
        current: 10.0,
        max: 100.0,
        invuln_until_tick: 0,
    };
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
    let mut h = Health {
        current: 30.0,
        max: 100.0,
        invuln_until_tick: 0,
    };
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
    let sta = Stamina {
        current: 3.0,
        max: 100.0,
        regen_per_sec: 10.0,
    };
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
    let mut hp = Health {
        current: 30.0,
        max: 100.0,
        invuln_until_tick: 0,
    };
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
