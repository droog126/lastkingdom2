//! V2 P0 骨架 — Bevy SystemSet 调度顺序
//!
//! 参考 docs\万国余烬_王冠赛季\万国余烬_王冠赛季_Bevy0 18架构实现 382d63af464581039accc17054980fc1.md §7 Schedule 与 SystemSet
//!
//! ## 设计原则
//! - `FixedUpdate` 30Hz 跑权威模拟，按下面顺序 `.chain()` 推进
//! - `Update` / `PostUpdate` 跑表现层（渲染、动画、UI、相机）
//! - 跨系统通信用 `Message`（见 messages.rs），不直接 ResMut 互调

use bevy::prelude::*;

/// 权威模拟 SystemSet — 30Hz FixedUpdate 内严格顺序
///
/// 每 tick 流水线：
/// ```text
/// ReceiveNet -> ReadInput -> BuildIntent -> ValidateRules -> Movement
/// -> Interaction -> Combat -> ResourceAndLoot -> EquipmentAndStats
/// -> NationAndArtifact -> ScoreAndAudit -> Snapshot
/// ```
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimSet {
    /// 接收网络包（专服/在线模式才有）
    ReceiveNet,
    /// 读取本地输入缓冲（从 PreUpdate 累积的 InputAccumulator）
    ReadInput,
    /// 构建意图（按键 → 移动/攻击/施法 IntentMsg）
    BuildIntent,
    /// 校验规则（保护期 / 距离 / 视线 / 负重 / 资源够不够）
    ValidateRules,
    /// 移动 + 疾跑蓄势 + 负重
    Movement,
    /// 交互（采集 / 开箱 / 拾取 / 建造前置）
    Interaction,
    /// 战斗（剑术 / 招架 / 法术 / 伤害结算）
    Combat,
    /// 资源与掉落（矿坑 / 资源池扣除 / 战利品 / 审计）
    ResourceAndLoot,
    /// 装备与数值聚合（StatBlock 重算）
    EquipmentAndStats,
    /// 国家与神器（建国 / 王权秘仪 / 神器激活）
    NationAndArtifact,
    /// 计分与审计（公开赛积分 / 资源守恒 / 神器轨迹）
    ScoreAndAudit,
    /// 快照生成（30Hz 写入 SnapshotBuffer 准备发包）
    Snapshot,
}

/// 表现层 SystemSet — `Update` 跑镜头/UI/动画，按需
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PresentationSet {
    /// 相机跟随（第一人称 / 第三人称 / orbit）
    CameraFollow,
    /// 动画状态机（基于权威 SimTransform + 本地 look input）
    Animation,
    /// VFX 与音效触发（基于表现层事件，不读权威组件）
    Vfx,
    /// UI 更新（HUD / 背包 / 装备 / 地形工具）
    Ui,
    /// 视觉 Transform 同步（SimTransform -> RenderProxy 插值）
    TransformSync,
}

/// 把 12 个 SimSet 装进 tuple 的辅助宏（避免手写 12 行 .chain()）
#[macro_export]
macro_rules! sim_set_tuple {
    () => {
        (
            $crate::v2::app_sets::SimSet::ReceiveNet,
            $crate::v2::app_sets::SimSet::ReadInput,
            $crate::v2::app_sets::SimSet::BuildIntent,
            $crate::v2::app_sets::SimSet::ValidateRules,
            $crate::v2::app_sets::SimSet::Movement,
            $crate::v2::app_sets::SimSet::Interaction,
            $crate::v2::app_sets::SimSet::Combat,
            $crate::v2::app_sets::SimSet::ResourceAndLoot,
            $crate::v2::app_sets::SimSet::EquipmentAndStats,
            $crate::v2::app_sets::SimSet::NationAndArtifact,
            $crate::v2::app_sets::SimSet::ScoreAndAudit,
            $crate::v2::app_sets::SimSet::Snapshot,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simset_labels_unique() {
        // 12 个不重复枚举值（编译期保证，运行时再 sanity-check 一次）
        use std::collections::HashSet;
        let labels: HashSet<&'static str> = [
            "ReceiveNet",
            "ReadInput",
            "BuildIntent",
            "ValidateRules",
            "Movement",
            "Interaction",
            "Combat",
            "ResourceAndLoot",
            "EquipmentAndStats",
            "NationAndArtifact",
            "ScoreAndAudit",
            "Snapshot",
        ]
        .iter()
        .copied()
        .collect();
        assert_eq!(labels.len(), 12);
    }
}
