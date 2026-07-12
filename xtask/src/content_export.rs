use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use lk2_core::content::{
    ContentCategory, ContentExportDefinition, ContentExportRecipe, ContentRegistry,
    ContentRegistryExport, ContentStatus, game_content_registry,
};
use serde_json::json;

use crate::Result;

const EXPORT_SCHEMA_VERSION: u32 = ContentRegistry::EXPORT_SCHEMA_VERSION;

pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let output_dir = output_dir(root, args)?;
    fs::create_dir_all(&output_dir)
        .map_err(|error| format!("create content export directory failed: {error}"))?;

    let registry = game_content_registry();
    let export = registry.export();
    write_file(
        &output_dir.join("content_registry.json"),
        &(registry
            .export_json()
            .map_err(|error| format!("serialize content registry failed: {error}"))?
            + "\n"),
    )?;
    write_file(
        &output_dir.join("content.csv"),
        &content_csv(&export.content)?,
    )?;
    let active = registry.export_status(ContentStatus::Active);
    let planned = registry.export_status(ContentStatus::Planned);
    write_export_json(&output_dir.join("content_active.json"), &active)?;
    write_export_json(&output_dir.join("content_planned.json"), &planned)?;
    write_file(
        &output_dir.join("content_active.csv"),
        &content_csv(&active.content)?,
    )?;
    write_file(
        &output_dir.join("content_planned.csv"),
        &content_csv(&planned.content)?,
    )?;
    write_file(
        &output_dir.join("recipes.csv"),
        &recipes_csv(&export.recipes),
    )?;
    write_file(&output_dir.join("content.md"), &content_markdown(&export))?;

    let mut category_counts = BTreeMap::new();
    for definition in &export.content {
        *category_counts
            .entry(category_name(definition.category).to_string())
            .or_insert(0usize) += 1;
    }
    let manifest = json!({
        "schema_version": EXPORT_SCHEMA_VERSION,
        "content_count": export.content.len(),
        "recipe_count": export.recipes.len(),
        "active_count": active.content.len(),
        "planned_count": planned.content.len(),
        "categories": category_counts,
        "files": [
            "content_registry.json",
            "content_active.json",
            "content_planned.json",
            "content.csv",
            "content_active.csv",
            "content_planned.csv",
            "recipes.csv",
            "content.md",
            "manifest.json"
        ],
    });
    write_file(
        &output_dir.join("manifest.json"),
        &(serde_json::to_string_pretty(&manifest)
            .map_err(|error| format!("serialize content manifest failed: {error}"))?
            + "\n"),
    )?;

    println!("content export complete");
    println!("  output: {}", output_dir.display());
    println!(
        "  content: {} ({} active, {} planned)",
        export.content.len(),
        active.content.len(),
        planned.content.len()
    );
    println!("  recipes: {}", export.recipes.len());
    println!("  report:  content.md");
    Ok(())
}

fn output_dir(root: &Path, args: &[String]) -> Result<PathBuf> {
    let mut output = None;
    for argument in args {
        if let Some(path) = argument.strip_prefix("--out=") {
            output = Some(path.to_string());
        } else if argument == "--out" {
            return Err("use --out=PATH for export-content".to_string());
        } else {
            return Err(format!("unknown export-content argument: {argument}"));
        }
    }
    Ok(output
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| root.to_path_buf())))
}

fn write_file(path: &Path, body: &str) -> Result<()> {
    fs::write(path, body).map_err(|error| format!("write {} failed: {error}", path.display()))
}

fn write_export_json(path: &Path, export: &ContentRegistryExport) -> Result<()> {
    let body = serde_json::to_string_pretty(export)
        .map_err(|error| format!("serialize {} failed: {error}", path.display()))?;
    write_file(path, &(body + "\n"))
}

fn content_csv(definitions: &[ContentExportDefinition]) -> Result<String> {
    let mut output = String::from(
        "id,key,display_name,category,status,description,source,model_path,scale,preferred_biome,source_block,produced_resources,stack_limit\n",
    );
    for definition in definitions {
        let source = serde_json::to_string(&definition.source)
            .map_err(|error| format!("serialize content source failed: {error}"))?;
        let produced = definition
            .produced_resources
            .iter()
            .map(|yield_| {
                format!(
                    "{}:{}:{}",
                    yield_.content.id.0, yield_.content.key, yield_.amount
                )
            })
            .collect::<Vec<_>>()
            .join(";");
        let scale = definition.visual.as_ref().map_or_else(
            || String::new(),
            |visual| {
                format!(
                    "{},{},{}",
                    visual.scale[0], visual.scale[1], visual.scale[2]
                )
            },
        );
        let model_path = definition
            .visual
            .as_ref()
            .map(|visual| visual.model_path.as_str())
            .unwrap_or_default();
        let preferred_biome = definition
            .visual
            .as_ref()
            .and_then(|visual| visual.preferred_biome)
            .map(|biome| format!("{biome:?}"))
            .unwrap_or_default();
        let source_block = definition
            .visual
            .as_ref()
            .and_then(|visual| visual.source_block)
            .map(|block| format!("{block:?}"))
            .unwrap_or_default();
        let row = [
            definition.id.0.to_string(),
            definition.key.clone(),
            definition.display_name.clone(),
            category_name(definition.category).to_string(),
            status_name(definition.status).to_string(),
            definition.description.clone(),
            source,
            model_path.to_string(),
            scale,
            preferred_biome,
            source_block,
            produced,
            definition
                .stack_limit
                .map(|limit| limit.to_string())
                .unwrap_or_default(),
        ];
        output.push_str(
            &row.iter()
                .map(|value| csv_cell(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    Ok(output)
}

fn recipes_csv(recipes: &[ContentExportRecipe]) -> String {
    let mut output = String::from(
        "recipe_id,recipe_key,display_name,output_id,output_key,output_amount,ingredient_id,ingredient_key,ingredient_amount\n",
    );
    for recipe in recipes {
        for ingredient in &recipe.ingredients {
            let row = [
                recipe.id.0.to_string(),
                recipe.key.clone(),
                recipe.display_name.clone(),
                recipe.output.id.0.to_string(),
                recipe.output.key.clone(),
                recipe.output_amount.to_string(),
                ingredient.content.id.0.to_string(),
                ingredient.content.key.clone(),
                ingredient.amount.to_string(),
            ];
            output.push_str(
                &row.iter()
                    .map(|value| csv_cell(value))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            output.push('\n');
        }
    }
    output
}

fn content_markdown(export: &ContentRegistryExport) -> String {
    let active_count = export
        .content
        .iter()
        .filter(|definition| definition.status == ContentStatus::Active)
        .count();
    let planned_count = export
        .content
        .iter()
        .filter(|definition| definition.status == ContentStatus::Planned)
        .count();
    let mut categories = BTreeMap::<&'static str, Vec<&ContentExportDefinition>>::new();
    for definition in &export.content {
        categories
            .entry(category_name(definition.category))
            .or_default()
            .push(definition);
    }

    let mut output = String::from("# Content Export\n\n");
    output.push_str("## Overview\n\n");
    output.push_str("| Metric | Value |\n| --- | ---: |\n");
    output.push_str(&format!(
        "| Schema version | {} |\n| Content definitions | {} |\n| Active | {} |\n| Planned | {} |\n| Recipes | {} |\n\n",
        export.schema_version,
        export.content.len(),
        active_count,
        planned_count,
        export.recipes.len()
    ));

    output.push_str("## Content Catalog\n\n");
    for (category, definitions) in categories {
        output.push_str(&format!(
            "### {} ({})\n\n",
            category_title(category),
            definitions.len()
        ));
        output.push_str("| Key | Name | Status | Visual | Produces | Stack |\n");
        output.push_str("| --- | --- | --- | --- | --- | ---: |\n");
        for definition in definitions {
            let visual = definition
                .visual
                .as_ref()
                .map(format_visual)
                .unwrap_or_else(|| "-".to_string());
            let produced = if definition.produced_resources.is_empty() {
                "-".to_string()
            } else {
                definition
                    .produced_resources
                    .iter()
                    .map(|yield_| format!("{} x {}", yield_.content.key, yield_.amount))
                    .collect::<Vec<_>>()
                    .join("<br>")
            };
            let stack = definition
                .stack_limit
                .map(|limit| limit.to_string())
                .unwrap_or_else(|| "-".to_string());
            output.push_str(&format!(
                "| `{}` | {} | {} | {} | {} | {} |\n",
                markdown_cell(&definition.key),
                markdown_cell(&definition.display_name),
                status_name(definition.status),
                markdown_cell(&visual),
                markdown_cell(&produced),
                stack
            ));
        }
        output.push('\n');
    }

    output.push_str("## Recipes\n\n");
    if export.recipes.is_empty() {
        output.push_str("No recipes registered.\n");
    } else {
        output.push_str("| Recipe | Output | Ingredients |\n| --- | --- | --- |\n");
        for recipe in &export.recipes {
            let ingredients = recipe
                .ingredients
                .iter()
                .map(|ingredient| format!("{} x {}", ingredient.content.key, ingredient.amount))
                .collect::<Vec<_>>()
                .join("<br>");
            output.push_str(&format!(
                "| `{}` | {} x {} | {} |\n",
                markdown_cell(&recipe.key),
                markdown_cell(&recipe.output.key),
                recipe.output_amount,
                markdown_cell(&ingredients)
            ));
        }
    }
    output.push('\n');
    output.push_str(
        "IDs, descriptions, tags, sources and full structured data remain available in `content_registry.json`.\n",
    );
    output
}

fn format_visual(visual: &lk2_core::content::ContentVisual) -> String {
    let scale = visual
        .scale
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let biome = visual
        .preferred_biome
        .map(|biome| format!("biome: {biome:?}"))
        .unwrap_or_else(|| "biome: -".to_string());
    let source_block = visual
        .source_block
        .map(|block| format!("block: {block:?}"))
        .unwrap_or_else(|| "block: -".to_string());
    format!(
        "`{}`<br>scale: {}<br>{}; {}",
        markdown_cell(&visual.model_path),
        scale,
        biome,
        source_block
    )
}

fn category_title(category: &str) -> String {
    category
        .split('_')
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + characters.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn markdown_cell(value: &str) -> String {
    value
        .replace('|', "\\|")
        .replace('\n', " ")
        .replace('\r', " ")
}

fn category_name(category: ContentCategory) -> &'static str {
    match category {
        ContentCategory::Resource => "resource",
        ContentCategory::Creature => "creature",
        ContentCategory::Wildlife => "wildlife",
        ContentCategory::Plant => "plant",
        ContentCategory::ResourceNode => "resource_node",
        ContentCategory::Drop => "drop",
        ContentCategory::Item => "item",
    }
}

fn status_name(status: ContentStatus) -> &'static str {
    match status {
        ContentStatus::Active => "active",
        ContentStatus::Planned => "planned",
    }
}

fn csv_cell(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{content_markdown, csv_cell};
    use lk2_core::content::game_content_registry;

    #[test]
    fn csv_cells_escape_delimiters_and_quotes() {
        assert_eq!(csv_cell("plain"), "plain");
        assert_eq!(csv_cell("a,b"), "\"a,b\"");
        assert_eq!(csv_cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn markdown_report_is_grouped_and_keeps_recipes_readable() {
        let report = content_markdown(&game_content_registry().export());

        assert!(report.contains("# Content Export"));
        assert!(report.contains("### Creature (4)"));
        assert!(report.contains("| `creature.chicken` |"));
        assert!(report.contains("## Recipes"));
        assert!(report.contains("resource.dragon_heart x 1"));
    }
}
