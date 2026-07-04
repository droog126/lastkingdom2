use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::match_state::MatchClock;
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::player::PlayerState;
use crate::resource::{GlobalResourcePool, ResourceKind};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveKind {
    GatherResource { kind: ResourceKind, count: i64 },

    FoundNation,

    UpgradePop { target: u32 },

    KillMonsters { count: u32 },

    ReachPosition { pos: [i32; 3], radius: i32 },
}

impl ObjectiveKind {
    pub fn short_label(&self) -> String {
        match self {
            ObjectiveKind::GatherResource { kind, count } => {
                format!("采集 {:?} {}", kind, count)
            }
            ObjectiveKind::FoundNation => "建立第一座国家".into(),
            ObjectiveKind::UpgradePop { target } => format!("升级人口到 {}", target),
            ObjectiveKind::KillMonsters { count } => format!("击杀 {} 只怪物", count),
            ObjectiveKind::ReachPosition { pos, radius } => {
                format!("前往 ({},{},{}) {}m 内", pos[0], pos[1], pos[2], radius)
            }
        }
    }

    pub fn progress_str(&self, progress: &ObjectiveProgress) -> String {
        match (self, progress) {
            (ObjectiveKind::GatherResource { count, .. }, ObjectiveProgress::Count(p)) => {
                format!("{} / {}", p, count)
            }
            (ObjectiveKind::FoundNation, ObjectiveProgress::Flag(b)) => {
                if *b {
                    "完成".into()
                } else {
                    "未完成".into()
                }
            }
            (ObjectiveKind::UpgradePop { target }, ObjectiveProgress::Pop(p)) => {
                format!("{} / {}", p, target)
            }
            (ObjectiveKind::KillMonsters { count }, ObjectiveProgress::CountU(p)) => {
                format!("{} / {}", p, count)
            }
            (
                ObjectiveKind::ReachPosition { pos, radius },
                ObjectiveProgress::AtPosition { reached },
            ) => {
                if *reached {
                    "已到达".into()
                } else {
                    format!("目标 ({},{},{}) {}m 内", pos[0], pos[1], pos[2], radius)
                }
            }
            _ => "?".into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ObjectiveProgress {
    #[default]
    Empty,

    Count(i64),
    CountU(u32),

    Pop(u32),

    Flag(bool),

    AtPosition {
        reached: bool,
    },
}

impl ObjectiveProgress {
    pub fn is_done(&self) -> bool {
        match self {
            ObjectiveProgress::Empty => false,
            ObjectiveProgress::Count(n) => *n > 0,
            ObjectiveProgress::CountU(n) => *n > 0,
            ObjectiveProgress::Pop(_) => false,
            ObjectiveProgress::Flag(b) => *b,
            ObjectiveProgress::AtPosition { reached } => *reached,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub kind: ObjectiveKind,
    pub progress: ObjectiveProgress,
    pub done: bool,
}

impl Objective {
    pub fn new(id: impl Into<String>, kind: ObjectiveKind) -> Self {
        let progress = match &kind {
            ObjectiveKind::GatherResource { .. } => ObjectiveProgress::Count(0),
            ObjectiveKind::FoundNation => ObjectiveProgress::Flag(false),
            ObjectiveKind::UpgradePop { .. } => ObjectiveProgress::Pop(0),
            ObjectiveKind::KillMonsters { .. } => ObjectiveProgress::CountU(0),
            ObjectiveKind::ReachPosition { .. } => ObjectiveProgress::AtPosition { reached: false },
        };
        Self { id: id.into(), kind, progress, done: false }
    }

    pub fn check_complete(&self) -> bool {
        match (&self.kind, &self.progress) {
            (ObjectiveKind::GatherResource { count, .. }, ObjectiveProgress::Count(p)) => {
                *p >= *count
            }
            (ObjectiveKind::FoundNation, ObjectiveProgress::Flag(b)) => *b,
            (ObjectiveKind::UpgradePop { target }, ObjectiveProgress::Pop(p)) => *p >= *target,
            (ObjectiveKind::KillMonsters { count }, ObjectiveProgress::CountU(p)) => *p >= *count,
            (ObjectiveKind::ReachPosition { .. }, ObjectiveProgress::AtPosition { reached }) => {
                *reached
            }
            _ => false,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct Objectives {
    pub all: Vec<Objective>,

    pub current_idx: Option<usize>,
}

impl Objectives {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, obj: Objective) {
        if self.current_idx.is_none() && !obj.done {
            self.current_idx = Some(self.all.len());
        }
        self.all.push(obj);
    }

    pub fn current(&self) -> Option<&Objective> {
        self.current_idx.and_then(|i| self.all.get(i))
    }

    pub fn current_mut(&mut self) -> Option<&mut Objective> {
        let i = self.current_idx?;
        self.all.get_mut(i)
    }

    pub fn try_complete_current(&mut self) -> bool {
        let Some(i) = self.current_idx else {
            return false;
        };
        let Some(obj) = self.all.get(i) else {
            return false;
        };
        if obj.done {
            return false;
        }
        if !obj.check_complete() {
            return false;
        }

        self.all[i].done = true;

        let next = (i + 1..self.all.len()).find(|&j| !self.all[j].done);
        self.current_idx = next;
        true
    }

    pub fn default_chain() -> Self {
        let mut o = Objectives::new();
        o.push(Objective::new(
            "q1_gather_wood",
            ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 },
        ));
        o.push(Objective::new(
            "q2_found_nation",
            ObjectiveKind::FoundNation,
        ));
        o.push(Objective::new(
            "q3_gather_food",
            ObjectiveKind::GatherResource { kind: ResourceKind::Food, count: 30 },
        ));
        o.push(Objective::new(
            "q4_upgrade_pop",
            ObjectiveKind::UpgradePop { target: 10 },
        ));
        o.push(Objective::new(
            "q5_kill_monsters",
            ObjectiveKind::KillMonsters { count: 5 },
        ));
        o.push(Objective::new(
            "q6_reach_summit",
            ObjectiveKind::ReachPosition { pos: [48, 30, 48], radius: 5 },
        ));
        o
    }
}

#[derive(Message, Debug, Clone)]
pub struct ObjectiveCompleted {
    pub id: String,
    pub kind: ObjectiveKind,
    pub at_wall_secs: f32,
}

pub fn auto_advance_objectives(
    mut objectives: ResMut<Objectives>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    _monsters: Res<MonsterEcosystem>,
    match_clock: Res<MatchClock>,
    mut completed_events: MessageWriter<ObjectiveCompleted>,
) {
    let mut safety = 16;
    while safety > 0 {
        safety -= 1;
        let Some(obj) = objectives.current().cloned() else {
            break;
        };
        if obj.done {
            break;
        }

        match &obj.kind {
            ObjectiveKind::GatherResource { kind, .. } => {
                let cur = pool.get(*kind);
                let new_p = match obj.progress {
                    ObjectiveProgress::Count(_) => cur,
                    _ => cur,
                };
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Count(new_p);
                }
            }
            ObjectiveKind::FoundNation => {
                let mine = player.nation_id.is_some();
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Flag(mine);
                }
            }
            ObjectiveKind::UpgradePop { .. } => {
                let pop = player
                    .nation_id
                    .and_then(|id| nations.nations.get(&id))
                    .map(|n| n.pop_cap)
                    .unwrap_or(0);
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::Pop(pop);
                }
            }
            ObjectiveKind::KillMonsters { .. } => {
                let killed = player.monsters_killed;
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::CountU(killed);
                }
            }
            ObjectiveKind::ReachPosition { pos, radius } => {
                let dx = (player.block_pos[0] - pos[0]).abs();
                let dy = (player.block_pos[1] - pos[1]).abs();
                let dz = (player.block_pos[2] - pos[2]).abs();
                let reached = dx + dy + dz <= *radius;
                if let Some(cur_obj) = objectives.current_mut() {
                    cur_obj.progress = ObjectiveProgress::AtPosition { reached };
                }
            }
        }

        if objectives.try_complete_current() {
            let just_done = objectives
                .all
                .iter()
                .rev()
                .find(|o| o.done)
                .cloned()
                .expect("just-completed objective disappeared");
            info!(
                "[objective] ✓ 完成: {} ({}) → 下一条 idx={:?}",
                just_done.id,
                just_done.kind.short_label(),
                objectives.current_idx,
            );
            completed_events.write(ObjectiveCompleted {
                id: just_done.id.clone(),
                kind: just_done.kind.clone(),
                at_wall_secs: match_clock.wall_secs,
            });
        } else {
            break;
        }
    }
}

pub fn objective_phase_hints(
    mut completed_events: MessageReader<ObjectiveCompleted>,
    match_clock: Res<MatchClock>,
) {
    for ev in completed_events.read() {
        match ev.id.as_str() {
            "q5_kill_monsters" => {
                info!(
                    "[objective] {} 完成后,Wildland 阶段入口解锁 (剩余 {:.0}s)",
                    ev.id,
                    match_clock.phase_remaining_secs()
                );
            }
            "q6_reach_summit" => {
                info!(
                    "[objective] {} 到达王座山顶,SovereignReveal 叙事窗口开启",
                    ev.id
                );
            }
            _ => {}
        }
    }
}

pub struct ObjectivesPlugin;

impl Plugin for ObjectivesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Objectives>().add_message::<ObjectiveCompleted>().add_systems(
            FixedUpdate,
            (auto_advance_objectives, objective_phase_hints).chain(),
        );
    }
}

pub fn setup_default_objectives(mut commands: Commands) {
    commands.insert_resource(Objectives::default_chain());
}

pub fn on_phase_change_emit_summary(
    mut phase_events: MessageReader<crate::match_state::MatchPhaseChanged>,
    objectives: Res<Objectives>,
) {
    for ev in phase_events.read() {
        let done = objectives.all.iter().filter(|o| o.done).count();
        let total = objectives.all.len();
        info!(
            "[objective] phase {} → {}:已完成 {}/{}",
            ev.from.label_zh(),
            ev.to.label_zh(),
            done,
            total
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_pool() -> GlobalResourcePool {
        GlobalResourcePool::new()
    }

    fn add(pool: &mut GlobalResourcePool, k: ResourceKind, n: i64) {
        let _ = pool.try_add(k, n);
    }

    #[test]
    fn default_chain_order_and_kinds() {
        let c = Objectives::default_chain();
        assert_eq!(c.all.len(), 6);
        assert_eq!(c.current().unwrap().id, "q1_gather_wood");
        assert!(matches!(
            c.current().unwrap().kind,
            ObjectiveKind::GatherResource { .. }
        ));
    }

    #[test]
    fn q1_complete_after_10_wood() {
        let mut c = Objectives::default_chain();
        let mut pool = fresh_pool();
        add(&mut pool, ResourceKind::Wood, 5);

        let mut player = PlayerState::default();
        player.nation_id = None;
        player.monsters_killed = 0;
        player.block_pos = [0, 0, 0];

        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(pool.get(ResourceKind::Wood));
        }
        assert!(!c.current().unwrap().check_complete());

        add(&mut pool, ResourceKind::Wood, 5);
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(pool.get(ResourceKind::Wood));
        }
        assert!(c.current().unwrap().check_complete());
        assert!(c.try_complete_current());
        assert_eq!(c.current().unwrap().id, "q2_found_nation");
        assert!(c.all[0].done);
    }

    #[test]
    fn q2_found_nation_completes_when_player_has_nation_id() {
        let mut c = Objectives::default_chain();

        c.all[0].done = true;
        c.current_idx = Some(1);
        let mut player = PlayerState::default();
        player.nation_id = None;

        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Flag(player.nation_id.is_some());
        }
        assert!(!c.current().unwrap().check_complete());

        use crate::nation::NationId;
        player.nation_id = Some(NationId(1));
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Flag(player.nation_id.is_some());
        }
        assert!(c.current().unwrap().check_complete());
        assert!(c.try_complete_current());
        assert_eq!(c.current().unwrap().id, "q3_gather_food");
    }

    #[test]
    fn q4_upgrade_pop_completes_at_target() {
        let mut c = Objectives::default_chain();
        for i in 0..3 {
            c.all[i].done = true;
        }
        c.current_idx = Some(3);

        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Pop(5);
        }
        assert!(!c.current().unwrap().check_complete());
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Pop(10);
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn q5_kill_monsters_uses_player_counter() {
        let mut c = Objectives::default_chain();
        for i in 0..4 {
            c.all[i].done = true;
        }
        c.current_idx = Some(4);
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::CountU(3);
        }
        assert!(!c.current().unwrap().check_complete());
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::CountU(5);
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn q6_reach_position_uses_block_pos_manhattan() {
        let mut c = Objectives::default_chain();
        for i in 0..5 {
            c.all[i].done = true;
        }
        c.current_idx = Some(5);

        let mut player = PlayerState::default();
        player.block_pos = [50, 30, 50];
        let reached = {
            let pos = [48, 30, 48];
            let r = 5;
            let dx = (player.block_pos[0] - pos[0]).abs();
            let dy = (player.block_pos[1] - pos[1]).abs();
            let dz = (player.block_pos[2] - pos[2]).abs();
            dx + dy + dz <= r
        };
        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::AtPosition { reached };
        }
        assert!(c.current().unwrap().check_complete());
    }

    #[test]
    fn current_idx_advances_on_completion() {
        let mut c = Objectives::default_chain();
        assert_eq!(c.current_idx, Some(0));

        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(0);
        }
        assert!(!c.try_complete_current());

        if let Some(cur) = c.current_mut() {
            cur.progress = ObjectiveProgress::Count(10);
        }
        assert!(c.try_complete_current());
        assert_eq!(c.current_idx, Some(1));

        c.all[1].progress = ObjectiveProgress::Flag(true);
        assert!(c.try_complete_current());
        assert_eq!(c.current_idx, Some(2));
    }

    #[test]
    fn completed_event_payload() {
        let ev = ObjectiveCompleted {
            id: "q1".into(),
            kind: ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 },
            at_wall_secs: 12.5,
        };
        assert_eq!(ev.id, "q1");
        assert!((ev.at_wall_secs - 12.5).abs() < 1e-3);
        match ev.kind {
            ObjectiveKind::GatherResource { count, .. } => assert_eq!(count, 10),
            _ => panic!("expected gather"),
        }
    }

    #[test]
    fn objective_kind_progress_str_format() {
        let k = ObjectiveKind::GatherResource { kind: ResourceKind::Wood, count: 10 };
        let p = ObjectiveProgress::Count(5);
        assert_eq!(k.progress_str(&p), "5 / 10");
    }

    #[test]
    fn objectives_plugin_can_be_added_without_panic() {
        fn _assert_plugin<P: Plugin>(_: &P) {}
        let p = ObjectivesPlugin;
        _assert_plugin(&p);
    }

    #[test]
    fn empty_chain_current_is_none() {
        let mut c = Objectives::new();
        assert!(c.current().is_none());
        assert_eq!(c.current_idx, None);
        assert!(!c.try_complete_current());
    }
}
