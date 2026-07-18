//! Prepare a reproducible three-view model optimization task for an AI agent.
//!
//! The command renders the selected GLB from front, side, and top views, then
//! writes a portable task directory containing those images, the preview
//! manifest, and instructions that point the agent back to the canonical
//! generator in tools/.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;

use crate::{Result, args, workspace_command};

const OUTPUT_ROOT: &str = "screenshots/model_optimize";
const VIEWS: [&str; 3] = ["front", "side", "top"];
const MIN_PNG_BYTES: u64 = 8 * 1024;

#[derive(Debug, Serialize)]
struct OptimizationTask {
    schema_version: u32,
    asset_path: String,
    output_dir: String,
    source_glb_bytes: u64,
    views: Vec<ViewArtifact>,
    preview_manifest: String,
    instructions: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ViewArtifact {
    view: String,
    image: String,
    bytes: u64,
}

pub fn run(root: &Path, raw: &[String]) -> Result<()> {
    let (filter, skip_build, skip_render) = parse_args(raw)?;
    let (asset_path, asset_filter) = resolve_asset(root, &filter)?;
    let stem = asset_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("invalid GLB filename: {}", asset_path.display()))?;
    let asset_rel = asset_path
        .strip_prefix(root.join("assets"))
        .map_err(|_| format!("asset is outside assets/: {}", asset_path.display()))?
        .to_string_lossy()
        .replace('\\', "/");
    let task_key = safe_task_key(&asset_rel);
    let task_dir = root.join(OUTPUT_ROOT).join(&task_key);
    fs::create_dir_all(&task_dir).map_err(|error| error.to_string())?;

    let client_exe = root.join("target/debug").join(if cfg!(windows) {
        "lk2-client.exe"
    } else {
        "lk2-client"
    });
    if !skip_render {
        if !skip_build || !client_exe.exists() {
            build_client(root)?;
        }
        if !client_exe.exists() {
            return Err(format!("client binary not found: {}", client_exe.display()));
        }
    }

    let preview_root = root.join("screenshots/model_preview");
    let mut views = Vec::with_capacity(VIEWS.len());
    for view in VIEWS {
        let source_png = preview_root.join(format!("{stem}_{view}.png"));
        let output_png = task_dir.join(format!("{view}.png"));
        if !skip_render {
            let _ = fs::remove_file(&source_png);
            run_client_view(root, &client_exe, &asset_filter, view)?;
        }
        if !source_png.is_file() {
            return Err(format!(
                "missing {view} view: {}; run without --skip-render",
                source_png.display()
            ));
        }
        let bytes = fs::metadata(&source_png)
            .map_err(|error| error.to_string())?
            .len();
        if bytes < MIN_PNG_BYTES {
            return Err(format!(
                "{view} view is too small ({bytes} bytes): {}",
                source_png.display()
            ));
        }
        fs::copy(&source_png, &output_png).map_err(|error| error.to_string())?;
        views.push(ViewArtifact {
            view: view.to_string(),
            image: format!("{view}.png"),
            bytes,
        });
    }

    let preview_manifest = task_dir.join("preview_manifest.json");
    let global_manifest = preview_root.join("manifest.json");
    if global_manifest.is_file() {
        fs::copy(&global_manifest, &preview_manifest).map_err(|error| error.to_string())?;
    }

    let task = OptimizationTask {
        schema_version: 1,
        asset_path: format!("assets/{asset_rel}"),
        output_dir: task_dir
            .strip_prefix(root)
            .unwrap_or(&task_dir)
            .to_string_lossy()
            .replace('\\', "/"),
        source_glb_bytes: fs::metadata(&asset_path)
            .map_err(|error| error.to_string())?
            .len(),
        views,
        preview_manifest: if preview_manifest.is_file() {
            "preview_manifest.json".to_string()
        } else {
            String::new()
        },
        instructions: vec![
            "Inspect all three images before changing geometry.".to_string(),
            "Find the asset's sole owner in tools/model_catalog.py.".to_string(),
            "Modify the deterministic generator or tools/models_lib.py; do not hand-edit only the GLB.".to_string(),
            "Run python tools/audit_model_generators.py and python tools/model_pipeline.py validate after rebuilding.".to_string(),
        ],
    };
    let task_json = serde_json::to_string_pretty(&task).map_err(|error| error.to_string())?;
    fs::write(task_dir.join("task.json"), task_json).map_err(|error| error.to_string())?;
    fs::write(
        task_dir.join("task.md"),
        task_markdown(&task, &asset_rel, stem),
    )
    .map_err(|error| error.to_string())?;

    println!(
        ">>> model optimization task ready: {}",
        task_dir.join("task.md").display()
    );
    println!(">>> views: {}", task_dir.display());
    Ok(())
}

fn parse_args(raw: &[String]) -> Result<(String, bool, bool)> {
    let mut filter = None;
    let mut skip_build = false;
    let mut skip_render = false;
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("asset") | Some("model") => {
                filter = inline.or_else(|| args::take_next(raw, &mut i));
            }
            Some("skipbuild") | Some("skip-build") => skip_build = true,
            Some("skiprender") | Some("skip-render") => skip_render = true,
            Some("help") | Some("h") | Some("?") => {
                println!("xtask model-optimize <asset> [--skip-build] [--skip-render]");
                return Err("help requested".to_string());
            }
            None if filter.is_none() => filter = Some(raw[i].clone()),
            Some(other) => return Err(format!("unknown model-optimize flag: --{other}")),
            None => return Err("model-optimize accepts only one asset filter".to_string()),
        }
        i += 1;
    }
    let filter =
        filter.ok_or_else(|| "model-optimize requires an asset stem or path".to_string())?;
    Ok((filter, skip_build, skip_render))
}

fn resolve_asset(root: &Path, filter: &str) -> Result<(PathBuf, String)> {
    let needle = normalize_filter(filter);
    let mut matches = Vec::new();
    collect_glbs(&root.join("assets"), &root.join("assets"), &mut matches);
    let matches: Vec<(PathBuf, String)> = matches
        .into_iter()
        .filter(|(_, relative)| path_matches(relative, &needle))
        .collect();
    match matches.as_slice() {
        [(path, relative)] => Ok((path.clone(), relative.clone())),
        [] => Err(format!("no GLB matched '{filter}' under assets/")),
        _ => Err(format!(
            "asset filter '{filter}' is ambiguous; use one of: {}",
            matches
                .iter()
                .map(|(_, relative)| relative.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn collect_glbs(root: &Path, dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_glbs(root, &path, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some("glb") {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((path, relative));
        }
    }
    out.sort_by(|left, right| left.1.cmp(&right.1));
}

fn normalize_filter(filter: &str) -> String {
    let normalized = filter.trim().trim_matches('"').replace('\\', "/");
    let normalized = normalized.strip_prefix("assets/").unwrap_or(&normalized);
    normalized
        .strip_suffix(".glb")
        .unwrap_or(normalized)
        .to_ascii_lowercase()
}

fn path_matches(relative: &str, needle: &str) -> bool {
    let normalized = relative
        .strip_suffix(".glb")
        .unwrap_or(relative)
        .to_ascii_lowercase();
    let stem = normalized.rsplit('/').next().unwrap_or(&normalized);
    normalized == needle || stem == needle || normalized.ends_with(&format!("/{needle}"))
}

fn safe_task_key(relative: &str) -> String {
    relative
        .strip_suffix(".glb")
        .unwrap_or(relative)
        .chars()
        .map(|ch| match ch {
            '/' | '\\' => '_',
            ch if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' => ch,
            _ => '_',
        })
        .collect()
}

fn build_client(root: &Path) -> Result<()> {
    println!(">>> building lk2-client for three-view capture");
    let status = workspace_command(root, "cargo")
        .args(["build", "-p", "lk2-client"])
        .status()
        .map_err(|error| format!("cargo build failed to start: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("cargo build -p lk2-client failed".to_string())
    }
}

fn run_client_view(root: &Path, client_exe: &Path, asset: &str, view: &str) -> Result<()> {
    println!(">>> rendering {view} view for {asset}");
    let status = Command::new(client_exe)
        .current_dir(root)
        .args([
            "--model-preview".to_string(),
            format!("--model-preview-one={asset}"),
            format!("--model-preview-view={view}"),
            "--model-preview-no-base".to_string(),
            "--model-preview-shot".to_string(),
        ])
        .env("BEVY_DISABLE_ACCESSIBILITY", "1")
        .env("WGPU_BACKEND", "vulkan")
        .status()
        .map_err(|error| format!("failed to start model preview: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("model preview failed for {view} view"))
    }
}

fn task_markdown(task: &OptimizationTask, asset_rel: &str, stem: &str) -> String {
    let mut markdown = String::new();
    markdown.push_str("# Three-view model optimization task\n\n");
    markdown.push_str(&format!("- Source asset: assets/{asset_rel}\n"));
    markdown.push_str(&format!("- Source size: {} bytes\n", task.source_glb_bytes));
    markdown.push_str("- Input views:\n");
    for view in &task.views {
        markdown.push_str(&format!("  - {}: {}\n", view.view, view.image));
    }
    if !task.preview_manifest.is_empty() {
        markdown.push_str("- GLB inspection: preview_manifest.json\n");
    }
    markdown.push_str(
        "\n## Instructions for the AI\n\n\
Inspect front.png, side.png, and top.png together. Identify silhouette, proportions, contact points, gaps, intersections, orientation, and material-role problems. The images are captured without the showroom display base.\n\n\
Find the asset's single owner in tools/model_catalog.py, then modify that deterministic generator or a shared helper in tools/models_lib.py. Do not hand-edit only the exported GLB.\n\n\
After the change, rebuild the asset with the canonical pipeline, run python tools/audit_model_generators.py, run python tools/model_pipeline.py validate, and regenerate this task's three views.\n\n\
The requested model stem is ",
    );
    markdown.push_str(stem);
    markdown.push_str(
        ". Preserve the project's 5000-triangle hard limit and target-camera readability.\n",
    );
    markdown
}

#[cfg(test)]
mod tests {
    use super::{normalize_filter, path_matches, safe_task_key};

    #[test]
    fn normalizes_asset_paths_for_matching() {
        assert_eq!(
            normalize_filter("assets\\animals\\rabbit.glb"),
            "animals/rabbit"
        );
        assert!(path_matches("animals/rabbit.glb", "animals/rabbit"));
        assert!(path_matches("animals/rabbit.glb", "rabbit"));
        assert!(!path_matches("animals/fox.glb", "rabbit"));
    }

    #[test]
    fn task_key_keeps_collection_identity() {
        assert_eq!(safe_task_key("animals/rabbit.glb"), "animals_rabbit");
    }
}
