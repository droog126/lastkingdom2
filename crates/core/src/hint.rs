

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetHint {

    pub label_zh: &'static str,

    pub key_hint: &'static str,

    pub block_pos: [i32; 3],
}

pub fn find_nearest_target(
    player: [i32; 3],
    candidates: &[TargetHint],
) -> Option<(usize, f32, &TargetHint)> {
    let mut best: Option<(usize, f32)> = None;
    for (i, hint) in candidates.iter().enumerate() {
        let dx = (hint.block_pos[0] - player[0]) as f32;
        let dy = (hint.block_pos[1] - player[1]) as f32;
        let dz = (hint.block_pos[2] - player[2]) as f32;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        match best {
            None => best = Some((i, dist)),
            Some((_, bd)) if dist < bd => best = Some((i, dist)),
            _ => {}
        }
    }
    best.map(|(i, d)| (i, d, &candidates[i]))
}

pub fn format_distance(dist_blocks: f32) -> String {
    if dist_blocks < 1.0 {
        return "脚下".to_string();
    }
    if dist_blocks >= 100.0 {
        let tens = (dist_blocks / 10.0).round() as i32 * 10;
        return format!("{}+m", tens);
    }
    let meters = dist_blocks.round() as i32;
    if meters < 1 {
        "1m".to_string()
    } else {
        format!("{}m", meters)
    }
}

pub fn format_nearest_target_hint(hint: &TargetHint, dist_blocks: f32) -> String {
    let dist = format_distance(dist_blocks);
    if hint.key_hint.is_empty() {
        format!("最近: {} {}  → 走过去", hint.label_zh, dist)
    } else {
        format!("最近: {} {}  {}", hint.label_zh, dist, hint.key_hint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hint(label: &'static str, key: &'static str, pos: [i32; 3]) -> TargetHint {
        TargetHint { label_zh: label, key_hint: key, block_pos: pos }
    }

    #[test]
    fn empty_candidates_returns_none() {
        let out = find_nearest_target([0, 0, 0], &[]);
        assert!(out.is_none());
    }

    #[test]
    fn single_candidate_returns_itself() {
        let candidates = [hint("动物(牛)", "K 杀", [3, 0, 4])];
        let (i, d, h) = find_nearest_target([0, 0, 0], &candidates).unwrap();
        assert_eq!(i, 0);
        assert!((d - 5.0).abs() < 1e-4);
        assert_eq!(h.label_zh, "动物(牛)");
    }

    #[test]
    fn picks_nearest_among_multiple() {

        let candidates = [
            hint("A", "K", [3, 0, 4]),
            hint("B", "K", [6, 0, 0]),
            hint("C", "K", [0, 0, 2]),
        ];
        let (i, d, h) = find_nearest_target([0, 0, 0], &candidates).unwrap();
        assert_eq!(i, 2, "should pick the 0,0,2 target");
        assert!((d - 2.0).abs() < 1e-4);
        assert_eq!(h.label_zh, "C");
    }

    #[test]
    fn ties_resolve_to_first_appearance() {
        let candidates = [
            hint("first", "K", [1, 0, 0]),
            hint("second", "K", [-1, 0, 0]),
        ];
        let (i, _d, h) = find_nearest_target([0, 0, 0], &candidates).unwrap();
        assert_eq!(i, 0, "ties keep the first one in the slice");
        assert_eq!(h.label_zh, "first");
    }

    #[test]
    fn distance_uses_3d_euclidean() {
        let candidates = [hint("diag", "K", [2, 2, 1])];
        let (_i, d, _h) = find_nearest_target([0, 0, 0], &candidates).unwrap();

        assert!((d - 3.0).abs() < 1e-4);
    }

    #[test]
    fn format_distance_handles_thresholds() {
        assert_eq!(format_distance(0.0), "脚下");
        assert_eq!(format_distance(0.5), "脚下");
        assert_eq!(format_distance(0.99), "脚下");
        assert_eq!(format_distance(1.0), "1m");
        assert_eq!(format_distance(1.4), "1m");
        assert_eq!(format_distance(1.5), "2m");
        assert_eq!(format_distance(12.0), "12m");
        assert_eq!(format_distance(99.5), "100m");
        assert_eq!(format_distance(100.0), "100+m");
        assert_eq!(format_distance(125.0), "130+m");
    }

    #[test]
    fn format_nearest_target_hint_with_key() {
        let h = hint("动物(牛)", "K 杀", [3, 0, 4]);
        let s = format_nearest_target_hint(&h, 5.0);
        assert_eq!(s, "最近: 动物(牛) 5m  K 杀");
    }

    #[test]
    fn format_nearest_target_hint_without_key_shows_direction() {
        let h = hint("旗杆", "", [10, 0, 0]);
        let s = format_nearest_target_hint(&h, 10.0);
        assert_eq!(s, "最近: 旗杆 10m  → 走过去");
    }

    #[test]
    fn format_nearest_target_hint_close_distance_says_under_feet() {
        let h = hint("动物(鸡)", "K 杀", [0, 0, 0]);
        let s = format_nearest_target_hint(&h, 0.0);
        assert_eq!(s, "最近: 动物(鸡) 脚下  K 杀");
    }
}
