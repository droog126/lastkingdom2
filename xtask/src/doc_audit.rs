use std::{ffi::OsStr, fs, path::Path};

use crate::{Result, audit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocStatus {
    Current,
    Reference,
    Proposal,
}

fn parse_doc_status(text: &str) -> Option<DocStatus> {
    let marker = text.lines().next()?.trim();
    match marker {
        "<!-- doc-status: current -->" => Some(DocStatus::Current),
        "<!-- doc-status: reference -->" => Some(DocStatus::Reference),
        "<!-- doc-status: proposal -->" => Some(DocStatus::Proposal),
        _ => None,
    }
}

fn inspect_current_doc(path: &Path, text: &str) -> Vec<String> {
    if parse_doc_status(text) != Some(DocStatus::Current) {
        return Vec::new();
    }

    const STALE_TERMS: &[&str] = &[
        "loop.ps1",
        "state_t",
        ".harness/reins",
        "scripts/diff_state.py",
        "minecraft_bevy",
        "Bevy 0.18",
        "Lightyear 0.26",
        "compt",
        "broccoli",
        "currently healthy",
        "最新迭代",
        "测试总数已",
    ];
    let mut issues = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        for term in STALE_TERMS {
            if line.contains(term) {
                issues.push(format!(
                    "{}:{} current document contains stale term {term}",
                    path.display(),
                    line_idx + 1
                ));
            }
        }
        if has_windows_absolute_path(line) {
            issues.push(format!(
                "{}:{} current document contains a local Windows absolute path",
                path.display(),
                line_idx + 1
            ));
        }
    }
    issues
}

fn broken_local_links(root: &Path, path: &Path, text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let mut remaining = line;
        while let Some(open) = remaining.find("](") {
            let target_start = open + 2;
            let Some(close_offset) = remaining[target_start..].find(')') else {
                break;
            };
            let raw_target = remaining[target_start..target_start + close_offset].trim();
            remaining = &remaining[target_start + close_offset + 1..];
            let target = raw_target
                .trim_matches(|ch| ch == '<' || ch == '>')
                .split('#')
                .next()
                .unwrap_or("")
                .trim();
            if target.is_empty()
                || raw_target.starts_with('#')
                || target.contains("://")
                || target.starts_with("mailto:")
            {
                continue;
            }
            let resolved = path.parent().unwrap_or(root).join(target);
            if !resolved.exists() {
                issues.push(format!(
                    "{}:{} local Markdown link does not resolve: {raw_target}",
                    path.display(),
                    line_idx + 1
                ));
            }
        }
    }
    issues
}

pub fn run(root: &Path) -> Result<()> {
    let docs_root = root.join("docs");
    if !docs_root.exists() {
        return Err("Missing docs root: docs".to_string());
    }
    let docs_index_path = docs_root.join("README.md");
    let docs_index = fs::read_to_string(&docs_index_path).map_err(|e| e.to_string())?;
    let mut files = audit::find_files(&docs_root, |path| {
        path.extension().and_then(OsStr::to_str) == Some("md")
            && !path.components().any(|part| part.as_os_str() == "archive")
    })?;
    files.push(root.join("README.md"));
    files.sort();

    let mut issues = Vec::new();
    let mut current = 0_usize;
    let mut reference = 0_usize;
    let mut proposal = 0_usize;
    for path in &files {
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        match parse_doc_status(&text) {
            Some(DocStatus::Current) => {
                current += 1;
                issues.extend(inspect_current_doc(path, &text));
            }
            Some(DocStatus::Reference) => reference += 1,
            Some(DocStatus::Proposal) => proposal += 1,
            None => issues.push(format!(
                "{}: first line must declare doc-status current, reference, or proposal",
                audit::rel(root, path)
            )),
        }
        issues.extend(broken_local_links(root, path, &text));

        if path.starts_with(&docs_root) && path != &docs_index_path {
            let relative = path
                .strip_prefix(&docs_root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if !docs_index.contains(&format!("]({relative})")) {
                issues.push(format!("docs/README.md does not index {relative}"));
            }
        }
    }

    for required in [
        root.join("README.md"),
        docs_root.join("README.md"),
        docs_root.join("STARTING.md"),
        docs_root.join("architecture/engineering-baseline.md"),
        docs_root.join("notes/tdd.md"),
        docs_root.join("plans/closed-loop-iteration.md"),
    ] {
        let text = fs::read_to_string(&required).map_err(|e| e.to_string())?;
        if parse_doc_status(&text) != Some(DocStatus::Current) {
            issues.push(format!(
                "{} must remain a current document",
                audit::rel(root, &required)
            ));
        }
    }

    println!("# Documentation audit\n");
    println!("files: {}", files.len());
    println!("current: {current}");
    println!("reference: {reference}");
    println!("proposal: {proposal}");
    if issues.is_empty() {
        println!("OK: document status, current facts, index coverage, and links are valid");
        Ok(())
    } else {
        for issue in &issues {
            println!("FAIL: {issue}");
        }
        Err(format!(
            "Documentation audit failed with {} issue(s)",
            issues.len()
        ))
    }
}

fn has_windows_absolute_path(line: &str) -> bool {
    let bytes = line.as_bytes();
    bytes.windows(3).enumerate().any(|(idx, window)| {
        let drive = window[0].is_ascii_alphabetic()
            && window[1] == b':'
            && (window[2] == b'\\' || window[2] == b'/');
        let boundary = idx == 0
            || bytes[idx - 1].is_ascii_whitespace()
            || matches!(bytes[idx - 1], b'`' | b'(' | b'"' | b'\'');
        drive && boundary
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parses_document_status_marker() {
        assert_eq!(
            parse_doc_status("<!-- doc-status: current -->\n# Guide\n"),
            Some(DocStatus::Current)
        );
        assert_eq!(
            parse_doc_status("<!-- doc-status: reference -->\n# Notes\n"),
            Some(DocStatus::Reference)
        );
        assert_eq!(
            parse_doc_status("<!-- doc-status: proposal -->\n# Plan\n"),
            Some(DocStatus::Proposal)
        );
        assert_eq!(parse_doc_status("# Missing marker\n"), None);
    }

    #[test]
    fn current_docs_reject_legacy_facts_and_local_absolute_paths() {
        let issues = inspect_current_doc(
            Path::new("docs/STARTING.md"),
            "<!-- doc-status: current -->\nUse loop.ps1 from F:\\repo\\game.\nBevy 0.18.\n",
        );
        assert!(issues.iter().any(|issue| issue.contains("loop.ps1")));
        assert!(issues.iter().any(|issue| issue.contains("Bevy 0.18")));
        assert!(issues.iter().any(|issue| issue.contains("absolute path")));
    }

    #[test]
    fn reference_docs_may_preserve_historical_terms() {
        let text = "<!-- doc-status: reference -->\nHistorical loop.ps1 and Bevy 0.18 notes.\n";
        assert_eq!(parse_doc_status(text), Some(DocStatus::Reference));
        assert!(inspect_current_doc(Path::new("docs/old.md"), text).is_empty());
    }

    #[test]
    fn current_docs_allow_web_urls() {
        let text = "<!-- doc-status: current -->\nSee https://example.com/docs.\n";
        assert!(inspect_current_doc(Path::new("README.md"), text).is_empty());
    }

    #[test]
    fn local_markdown_links_must_resolve() {
        let root = temp_root("links");
        let docs = root.join("docs");
        fs::create_dir_all(&docs).unwrap();
        let guide = docs.join("guide.md");
        fs::write(
            &guide,
            "[missing](missing.md)\n[web](https://example.com)\n",
        )
        .unwrap();
        let issues = broken_local_links(&root, &guide, &fs::read_to_string(&guide).unwrap());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("missing.md"));
        fs::remove_dir_all(root).unwrap();
    }

    fn temp_root(name: &str) -> PathBuf {
        let mut root = std::env::temp_dir();
        root.push(format!("lk2_doc_audit_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
    }
}
