pub fn split_flag(arg: &str) -> (Option<String>, Option<String>) {
    if !arg.starts_with('-') {
        return (None, None);
    }
    let trimmed = arg.trim_start_matches('-');
    let mut pieces = trimmed.splitn(2, ['=', ':']);
    let name = pieces.next().map(normalize_flag);
    let inline = pieces.next().map(|s| s.trim_matches('"').to_string());
    (name, inline)
}

fn normalize_flag(s: &str) -> String {
    s.chars()
        .filter(|ch| *ch != '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

pub fn take_next(raw: &[String], i: &mut usize) -> Option<String> {
    let next = raw.get(*i + 1)?;
    if next.starts_with('-') {
        None
    } else {
        *i += 1;
        Some(next.clone())
    }
}

pub fn matches_false(value: Option<&str>) -> bool {
    value
        .map(|v| {
            matches!(
                v.trim_start_matches('$').to_ascii_lowercase().as_str(),
                "false" | "0"
            )
        })
        .unwrap_or(false)
}
