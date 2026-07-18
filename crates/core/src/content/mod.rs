//! Unified, stable content registration for gameplay and presentation metadata.
//!
//! `world::content` remains responsible for spatial content generation. This module
//! owns the cross-cutting registry used by inventory, crafting, discovery and future
//! content-management UIs.

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::ecology::animals::CreatureKind;
use crate::ecology::{
    ECOLOGY_CATALOG, EcologyKind, ResourceDropKind, ResourceNodeKind, TreeKind, WildlifeKind,
};
use crate::legendary::{LegendaryWeapon, legendary_recipe};
use crate::resource::{GlobalResourcePool, ResourceKind};
use crate::world::{Biome, BlockType};

/// Stable ID derived from a versioned key, never from enum declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId(pub u32);

impl ContentId {
    /// FNV-1a is small, deterministic and available in const contexts.
    #[must_use]
    pub const fn from_key(key: &str) -> Self {
        let bytes = key.as_bytes();
        let mut hash = 2_166_136_261u32;
        let mut index = 0;
        while index < bytes.len() {
            hash ^= bytes[index] as u32;
            hash = hash.wrapping_mul(16_777_619);
            index += 1;
        }
        Self(hash)
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:08x}", self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentCategory {
    Resource,
    Creature,
    Wildlife,
    Plant,
    ResourceNode,
    Drop,
    Item,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentStatus {
    Active,
    Planned,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentVisual {
    pub model_path: String,
    pub scale: [f32; 3],
    pub preferred_biome: Option<Biome>,
    pub source_block: Option<BlockType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentSource {
    Resource(ResourceKind),
    Ecology(EcologyKind),
    Item(LegendaryWeapon),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContentDefinition {
    pub id: ContentId,
    pub key: String,
    pub display_name: String,
    pub category: ContentCategory,
    pub status: ContentStatus,
    pub description: String,
    pub tags: Vec<String>,
    pub source: ContentSource,
    pub visual: Option<ContentVisual>,
    pub produced_resources: Vec<ResourceYield>,
    pub stack_limit: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceYield {
    pub resource: ResourceKind,
    pub amount: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub resource: ResourceKind,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeDefinition {
    pub id: ContentId,
    pub key: String,
    pub display_name: String,
    pub output: ContentId,
    pub output_amount: i64,
    pub ingredients: Vec<RecipeIngredient>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingIngredient {
    pub resource: ResourceKind,
    pub required: i64,
    pub current: i64,
}

impl RecipeDefinition {
    #[must_use]
    pub fn missing_ingredients(&self, resources: &GlobalResourcePool) -> Vec<MissingIngredient> {
        self.ingredients
            .iter()
            .filter_map(|ingredient| {
                let current = resources.get(ingredient.resource);
                (current < ingredient.amount).then_some(MissingIngredient {
                    resource: ingredient.resource,
                    required: ingredient.amount,
                    current,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn is_ready(&self, resources: &GlobalResourcePool) -> bool {
        self.missing_ingredients(resources).is_empty()
    }
}

#[derive(Resource, Clone, Debug)]
pub struct ContentRegistry {
    definitions: Vec<ContentDefinition>,
    definition_indices: HashMap<ContentId, usize>,
    recipes: Vec<RecipeDefinition>,
    recipe_indices: HashMap<ContentId, usize>,
}

/// A cross-domain reference used by exported data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContentReference {
    pub id: ContentId,
    pub key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContentExportYield {
    pub content: ContentReference,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ContentExportDefinition {
    pub id: ContentId,
    pub key: String,
    pub display_name: String,
    pub category: ContentCategory,
    pub status: ContentStatus,
    pub description: String,
    pub tags: Vec<String>,
    pub source: ContentSource,
    pub visual: Option<ContentVisual>,
    pub produced_resources: Vec<ContentExportYield>,
    pub stack_limit: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContentExportIngredient {
    pub content: ContentReference,
    pub amount: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContentExportRecipe {
    pub id: ContentId,
    pub key: String,
    pub display_name: String,
    pub output: ContentReference,
    pub output_amount: i64,
    pub ingredients: Vec<ContentExportIngredient>,
}

/// Stable, sorted wire format for tools, editors and data review.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ContentRegistryExport {
    pub schema_version: u32,
    pub content: Vec<ContentExportDefinition>,
    pub recipes: Vec<ContentExportRecipe>,
}

impl ContentRegistry {
    pub const EXPORT_SCHEMA_VERSION: u32 = 2;

    #[must_use]
    pub fn definitions(&self) -> &[ContentDefinition] {
        &self.definitions
    }

    #[must_use]
    pub fn definitions_in(
        &self,
        category: ContentCategory,
    ) -> impl Iterator<Item = &ContentDefinition> {
        self.definitions
            .iter()
            .filter(move |definition| definition.category == category)
    }

    #[must_use]
    pub fn get(&self, id: ContentId) -> Option<&ContentDefinition> {
        self.definition_indices
            .get(&id)
            .and_then(|index| self.definitions.get(*index))
    }

    #[must_use]
    pub fn id_for_key(&self, key: &str) -> ContentId {
        ContentId::from_key(key)
    }

    #[must_use]
    pub fn recipes(&self) -> &[RecipeDefinition] {
        &self.recipes
    }

    #[must_use]
    pub fn recipe(&self, id: ContentId) -> Option<&RecipeDefinition> {
        self.recipe_indices
            .get(&id)
            .and_then(|index| self.recipes.get(*index))
    }

    /// Clone into a deterministic export representation. HashMap internals never
    /// affect the order of generated files.
    #[must_use]
    pub fn export(&self) -> ContentRegistryExport {
        let mut content = self
            .definitions
            .iter()
            .map(|definition| ContentExportDefinition {
                id: definition.id,
                key: definition.key.clone(),
                display_name: definition.display_name.clone(),
                category: definition.category,
                status: definition.status,
                description: definition.description.clone(),
                tags: definition.tags.clone(),
                source: definition.source,
                visual: definition.visual.clone(),
                produced_resources: definition
                    .produced_resources
                    .iter()
                    .map(|yield_| ContentExportYield {
                        content: resource_reference(yield_.resource),
                        amount: yield_.amount,
                    })
                    .collect(),
                stack_limit: definition.stack_limit,
            })
            .collect::<Vec<_>>();
        content.sort_by(|left, right| left.key.cmp(&right.key));
        let mut recipes = self
            .recipes
            .iter()
            .map(|recipe| ContentExportRecipe {
                id: recipe.id,
                key: recipe.key.clone(),
                display_name: recipe.display_name.clone(),
                output: self
                    .get(recipe.output)
                    .map(|definition| ContentReference {
                        id: definition.id,
                        key: definition.key.clone(),
                    })
                    .expect("validated recipe output must be registered"),
                output_amount: recipe.output_amount,
                ingredients: recipe
                    .ingredients
                    .iter()
                    .map(|ingredient| ContentExportIngredient {
                        content: resource_reference(ingredient.resource),
                        amount: ingredient.amount,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        recipes.sort_by(|left, right| left.key.cmp(&right.key));
        ContentRegistryExport {
            schema_version: Self::EXPORT_SCHEMA_VERSION,
            content,
            recipes,
        }
    }

    pub fn export_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.export())
    }

    #[must_use]
    pub fn export_status(&self, status: ContentStatus) -> ContentRegistryExport {
        let mut export = self.export();
        let allowed_outputs = export
            .content
            .iter()
            .filter(|definition| definition.status == status)
            .map(|definition| definition.id)
            .collect::<HashSet<_>>();
        export
            .content
            .retain(|definition| definition.status == status);
        export
            .recipes
            .retain(|recipe| allowed_outputs.contains(&recipe.output.id));
        export
    }
}

fn resource_reference(resource: ResourceKind) -> ContentReference {
    ContentReference {
        id: resource_content_id(resource),
        key: resource_key(resource).to_string(),
    }
}

#[derive(Default)]
pub struct ContentRegistryBuilder {
    definitions: Vec<ContentDefinition>,
    recipes: Vec<RecipeDefinition>,
}

impl ContentRegistryBuilder {
    #[must_use]
    pub fn definition(mut self, definition: ContentDefinition) -> Self {
        self.definitions.push(definition);
        self
    }

    #[must_use]
    pub fn recipe(mut self, recipe: RecipeDefinition) -> Self {
        self.recipes.push(recipe);
        self
    }

    pub fn build(self) -> Result<ContentRegistry, ContentRegistryError> {
        if self.definitions.is_empty() {
            return Err(ContentRegistryError::Empty);
        }

        let mut definition_indices = HashMap::with_capacity(self.definitions.len());
        let mut keys = HashSet::with_capacity(self.definitions.len());
        for (index, definition) in self.definitions.iter().enumerate() {
            if definition.id != ContentId::from_key(&definition.key) {
                return Err(ContentRegistryError::IdMismatch(definition.key.clone()));
            }
            if definition.stack_limit.is_some_and(|limit| limit <= 0) {
                return Err(ContentRegistryError::InvalidStackLimit(
                    definition.key.clone(),
                ));
            }
            if definition_indices.insert(definition.id, index).is_some() {
                return Err(ContentRegistryError::DuplicateId(definition.id));
            }
            if !keys.insert(definition.key.clone()) {
                return Err(ContentRegistryError::DuplicateKey(definition.key.clone()));
            }
        }

        let mut recipe_indices = HashMap::with_capacity(self.recipes.len());
        for (index, recipe) in self.recipes.iter().enumerate() {
            if recipe.id != ContentId::from_key(&recipe.key) {
                return Err(ContentRegistryError::IdMismatch(recipe.key.clone()));
            }
            if recipe.output_amount <= 0 {
                return Err(ContentRegistryError::InvalidOutputAmount(
                    recipe.key.clone(),
                ));
            }
            if !definition_indices.contains_key(&recipe.output) {
                return Err(ContentRegistryError::UnknownOutput(recipe.output));
            }
            for ingredient in &recipe.ingredients {
                if ingredient.amount <= 0 {
                    return Err(ContentRegistryError::InvalidIngredient(recipe.key.clone()));
                }
                let ingredient_id = resource_content_id(ingredient.resource);
                if !definition_indices.contains_key(&ingredient_id) {
                    return Err(ContentRegistryError::UnknownIngredient(ingredient_id));
                }
            }
            if recipe_indices.insert(recipe.id, index).is_some() {
                return Err(ContentRegistryError::DuplicateRecipeId(recipe.id));
            }
        }

        Ok(ContentRegistry {
            definitions: self.definitions,
            definition_indices,
            recipes: self.recipes,
            recipe_indices,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentRegistryError {
    Empty,
    DuplicateId(ContentId),
    DuplicateKey(String),
    DuplicateRecipeId(ContentId),
    IdMismatch(String),
    InvalidStackLimit(String),
    InvalidOutputAmount(String),
    InvalidIngredient(String),
    UnknownOutput(ContentId),
    UnknownIngredient(ContentId),
}

impl fmt::Display for ContentRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "content registry is empty"),
            Self::DuplicateId(id) => write!(formatter, "duplicate content id {id}"),
            Self::DuplicateKey(key) => write!(formatter, "duplicate content key {key}"),
            Self::DuplicateRecipeId(id) => write!(formatter, "duplicate recipe id {id}"),
            Self::IdMismatch(key) => write!(formatter, "content id does not match key {key}"),
            Self::InvalidStackLimit(key) => write!(formatter, "invalid stack limit for {key}"),
            Self::InvalidOutputAmount(key) => write!(formatter, "invalid output amount for {key}"),
            Self::InvalidIngredient(key) => write!(formatter, "invalid ingredient in {key}"),
            Self::UnknownOutput(id) => write!(formatter, "recipe output {id} is not registered"),
            Self::UnknownIngredient(id) => {
                write!(formatter, "recipe ingredient {id} is not registered")
            }
        }
    }
}

impl std::error::Error for ContentRegistryError {}

#[must_use]
pub fn resource_content_id(kind: ResourceKind) -> ContentId {
    ContentId::from_key(resource_key(kind))
}

#[must_use]
pub fn ecology_content_id(kind: EcologyKind) -> ContentId {
    ContentId::from_key(ecology_key(kind))
}

#[must_use]
pub fn item_content_id(item: LegendaryWeapon) -> ContentId {
    ContentId::from_key(item_key(item))
}

#[must_use]
pub fn game_content_registry() -> ContentRegistry {
    let mut builder = ContentRegistryBuilder::default();

    for &resource in ResourceKind::ALL {
        let key = resource_key(resource);
        builder = builder.definition(ContentDefinition {
            id: ContentId::from_key(key),
            key: key.to_string(),
            display_name: resource.label_zh().to_string(),
            category: ContentCategory::Resource,
            status: builtin_status(ContentSource::Resource(resource)),
            description: format!(
                "Registered resource with a stack limit of {}",
                resource.max()
            ),
            tags: vec!["resource".into()],
            source: ContentSource::Resource(resource),
            visual: None,
            produced_resources: Vec::new(),
            stack_limit: Some(resource.max()),
        });
    }

    for entry in ECOLOGY_CATALOG {
        let key = ecology_key(entry.kind);
        let category = ecology_category(entry.kind);
        let produced_resources = entry
            .produced_resource
            .map(|resource| {
                vec![ResourceYield {
                    resource,
                    amount: 1,
                }]
            })
            .unwrap_or_default();
        builder = builder.definition(ContentDefinition {
            id: ContentId::from_key(key),
            key: key.to_string(),
            display_name: humanize_key(key),
            category,
            status: builtin_status(ContentSource::Ecology(entry.kind)),
            description: format!("Registered ecology content: {key}"),
            tags: vec![category_tag(category).into()],
            source: ContentSource::Ecology(entry.kind),
            visual: Some(ContentVisual {
                model_path: entry.model_path.to_string(),
                scale: [
                    entry.visual_scale.x,
                    entry.visual_scale.y,
                    entry.visual_scale.z,
                ],
                preferred_biome: entry.preferred_biome,
                source_block: entry.source_block,
            }),
            produced_resources,
            stack_limit: None,
        });
    }

    for item in [LegendaryWeapon::ReaperScythe, LegendaryWeapon::DragonKatana] {
        let key = item_key(item);
        builder = builder.definition(ContentDefinition {
            id: ContentId::from_key(key),
            key: key.to_string(),
            display_name: item.label().to_string(),
            category: ContentCategory::Item,
            status: builtin_status(ContentSource::Item(item)),
            description: format!("Legendary equipment: {key}"),
            tags: vec!["item".into(), "equipment".into()],
            source: ContentSource::Item(item),
            visual: Some(ContentVisual {
                model_path: item.model_path().to_string(),
                scale: [1.0, 1.0, 1.0],
                preferred_biome: None,
                source_block: None,
            }),
            produced_resources: Vec::new(),
            stack_limit: Some(1),
        });

        let ingredients = legendary_recipe(item)
            .iter()
            .map(|&(resource, amount)| RecipeIngredient { resource, amount })
            .collect::<Vec<_>>();
        if !ingredients.is_empty() {
            let recipe_key = format!("recipe.{key}");
            builder = builder.recipe(RecipeDefinition {
                id: ContentId::from_key(&recipe_key),
                key: recipe_key,
                display_name: format!("Craft {}", item.label()),
                output: ContentId::from_key(key),
                output_amount: 1,
                ingredients,
            });
        }
    }

    builder
        .build()
        .expect("built-in content registry must be valid")
}

fn ecology_category(kind: EcologyKind) -> ContentCategory {
    match kind {
        EcologyKind::Creature(_) => ContentCategory::Creature,
        EcologyKind::Wildlife(_) => ContentCategory::Wildlife,
        EcologyKind::Tree(_) => ContentCategory::Plant,
        EcologyKind::ResourceNode(_) => ContentCategory::ResourceNode,
        EcologyKind::ResourceDrop(_) => ContentCategory::Drop,
    }
}

fn builtin_status(source: ContentSource) -> ContentStatus {
    match source {
        ContentSource::Resource(resource) => match resource {
            ResourceKind::Wood
            | ResourceKind::Stone
            | ResourceKind::Apple
            | ResourceKind::Food
            | ResourceKind::WheatSeeds
            | ResourceKind::Carrot
            | ResourceKind::Potato
            | ResourceKind::Sunstone
            | ResourceKind::Frostcore
            | ResourceKind::LivingRoot => ContentStatus::Active,
            _ => ContentStatus::Planned,
        },
        ContentSource::Ecology(kind) => match kind {
            EcologyKind::Wildlife(_)
            | EcologyKind::Tree(_)
            | EcologyKind::ResourceNode(ResourceNodeKind::BerryBush)
            | EcologyKind::ResourceNode(ResourceNodeKind::Grass)
            | EcologyKind::ResourceNode(ResourceNodeKind::Flower)
            | EcologyKind::ResourceNode(ResourceNodeKind::RockMid)
            | EcologyKind::ResourceNode(ResourceNodeKind::RockMoss)
            | EcologyKind::ResourceNode(ResourceNodeKind::SunstoneCrystal)
            | EcologyKind::ResourceNode(ResourceNodeKind::FrostCrystal)
            | EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit)
            | EcologyKind::ResourceDrop(ResourceDropKind::Stone) => ContentStatus::Active,
            EcologyKind::Creature(_) => ContentStatus::Planned,
        },
        ContentSource::Item(LegendaryWeapon::DragonKatana) => ContentStatus::Active,
        ContentSource::Item(LegendaryWeapon::ReaperScythe) => ContentStatus::Planned,
    }
}

fn category_tag(category: ContentCategory) -> &'static str {
    match category {
        ContentCategory::Resource => "resource",
        ContentCategory::Creature => "creature",
        ContentCategory::Wildlife => "wildlife",
        ContentCategory::Plant => "plant",
        ContentCategory::ResourceNode => "resource-node",
        ContentCategory::Drop => "drop",
        ContentCategory::Item => "item",
    }
}

fn humanize_key(key: &str) -> String {
    key.replace(['.', '_'], " ")
}

fn resource_key(kind: ResourceKind) -> &'static str {
    use ResourceKind::*;
    match kind {
        Wood => "resource.wood",
        Stone => "resource.stone",
        HardenedWood => "resource.hardened_wood",
        Apple => "resource.apple",
        WheatSeeds => "resource.wheat_seeds",
        Carrot => "resource.carrot",
        Potato => "resource.potato",
        Food => "resource.food",
        Soul => "resource.soul",
        Sunstone => "resource.sunstone",
        Frostcore => "resource.frostcore",
        LivingRoot => "resource.living_root",
        BloodthistleSeeds => "resource.bloodthistle_seeds",
        FrostleafSeeds => "resource.frostleaf_seeds",
        VoidEssence => "resource.void_essence",
        GripOfFirelord => "resource.grip_of_firelord",
        CoreOfIceGiant => "resource.core_of_ice_giant",
        WhisperOfTreant => "resource.whisper_of_treant",
        EyeOfTheDeep => "resource.eye_of_the_deep",
        SandsOfTime => "resource.sands_of_time",
        WraithFiber => "resource.wraith_fiber",
        GuardianFragment => "resource.guardian_fragment",
        StormCore => "resource.storm_core",
        EarthRune => "resource.earth_rune",
        VampireFang => "resource.vampire_fang",
        PhoenixFeather => "resource.phoenix_feather",
        SpiritEssence => "resource.spirit_essence",
        RuneStone => "resource.rune_stone",
        RunePowder => "resource.rune_powder",
        StarSand => "resource.star_sand",
        RelicCore => "resource.relic_core",
        SovereignSpark => "resource.sovereign_spark",
        SparkFragment => "resource.spark_fragment",
        DragonHeart => "resource.dragon_heart",
        DragonScale => "resource.dragon_scale",
    }
}

fn ecology_key(kind: EcologyKind) -> &'static str {
    match kind {
        EcologyKind::Creature(CreatureKind::Pig) => "creature.pig",
        EcologyKind::Creature(CreatureKind::Sheep) => "creature.sheep",
        EcologyKind::Creature(CreatureKind::Cow) => "creature.cow",
        EcologyKind::Creature(CreatureKind::Chicken) => "creature.chicken",
        EcologyKind::Wildlife(WildlifeKind::Rabbit) => "wildlife.rabbit",
        EcologyKind::Wildlife(WildlifeKind::Deer) => "wildlife.deer",
        EcologyKind::Wildlife(WildlifeKind::Fox) => "wildlife.fox",
        EcologyKind::Wildlife(WildlifeKind::Bear) => "wildlife.bear",
        EcologyKind::Wildlife(WildlifeKind::Wolf) => "wildlife.wolf",
        EcologyKind::Tree(TreeKind::Sokpop) => "plant.sokpop_tree",
        EcologyKind::Tree(TreeKind::FallenStick) => "plant.fallen_stick",
        EcologyKind::Tree(TreeKind::Palm) => "plant.palm",
        EcologyKind::ResourceNode(ResourceNodeKind::BerryBush) => "resource_node.berry_bush",
        EcologyKind::ResourceNode(ResourceNodeKind::Grass) => "resource_node.grass",
        EcologyKind::ResourceNode(ResourceNodeKind::Flower) => "resource_node.flower",
        EcologyKind::ResourceNode(ResourceNodeKind::RockMid) => "resource_node.rock_mid",
        EcologyKind::ResourceNode(ResourceNodeKind::RockMoss) => "resource_node.rock_moss",
        EcologyKind::ResourceNode(ResourceNodeKind::SunstoneCrystal) => {
            "resource_node.sunstone_crystal"
        }
        EcologyKind::ResourceNode(ResourceNodeKind::FrostCrystal) => "resource_node.frost_crystal",
        EcologyKind::ResourceDrop(ResourceDropKind::BerryFruit) => "drop.berry_fruit",
        EcologyKind::ResourceDrop(ResourceDropKind::Stone) => "drop.stone",
    }
}

fn item_key(item: LegendaryWeapon) -> &'static str {
    match item {
        LegendaryWeapon::ReaperScythe => "item.reaper_scythe",
        LegendaryWeapon::DragonKatana => "item.dragon_katana",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_registry_has_unique_stable_definitions() {
        let registry = game_content_registry();
        let ids = registry
            .definitions()
            .iter()
            .map(|definition| definition.id)
            .collect::<HashSet<_>>();

        assert_eq!(ids.len(), registry.definitions().len());
        assert!(registry.definitions_in(ContentCategory::Resource).count() >= 30);
        assert!(registry.definitions_in(ContentCategory::Plant).count() >= 3);
        assert!(registry.definitions_in(ContentCategory::Creature).count() >= 4);
        assert!(registry.definitions_in(ContentCategory::Wildlife).count() >= 5);
        assert!(
            registry
                .definitions_in(ContentCategory::ResourceNode)
                .count()
                >= 6
        );
    }

    #[test]
    fn recipes_reference_registered_outputs_and_report_missing_materials() {
        let registry = game_content_registry();
        let recipe = registry
            .recipes()
            .iter()
            .find(|recipe| recipe.key == "recipe.item.dragon_katana")
            .expect("dragon recipe");
        assert!(registry.get(recipe.output).is_some());

        let empty = GlobalResourcePool::new();
        assert!(!recipe.is_ready(&empty));
        assert_eq!(recipe.missing_ingredients(&empty).len(), 2);
    }

    #[test]
    fn content_ids_do_not_depend_on_enum_order() {
        assert_eq!(
            resource_content_id(ResourceKind::Wood),
            ContentId::from_key("resource.wood")
        );
        assert_eq!(
            ecology_content_id(EcologyKind::Tree(TreeKind::Palm)),
            ContentId::from_key("plant.palm")
        );
        assert_eq!(
            item_content_id(LegendaryWeapon::DragonKatana),
            ContentId::from_key("item.dragon_katana")
        );
    }

    #[test]
    fn resource_definitions_use_localized_resource_names() {
        let registry = game_content_registry();

        assert_eq!(
            registry
                .get(resource_content_id(ResourceKind::Wood))
                .expect("wood should be registered")
                .display_name,
            ResourceKind::Wood.label_zh()
        );
        assert_eq!(
            registry
                .get(resource_content_id(ResourceKind::Stone))
                .expect("stone should be registered")
                .display_name,
            ResourceKind::Stone.label_zh()
        );
    }

    #[test]
    fn export_is_versioned_and_sorted_for_stable_diffs() {
        let export = game_content_registry().export();

        assert_eq!(
            export.schema_version,
            ContentRegistry::EXPORT_SCHEMA_VERSION
        );
        assert!(
            export
                .content
                .windows(2)
                .all(|entries| entries[0].key <= entries[1].key)
        );
        assert!(
            export
                .recipes
                .windows(2)
                .all(|entries| entries[0].key <= entries[1].key)
        );

        let stone = export
            .content
            .iter()
            .find(|definition| definition.key == "resource_node.rock_mid")
            .expect("rock content should be exported");
        assert_eq!(stone.produced_resources[0].content.key, "resource.stone");

        let recipe = &export.recipes[0];
        assert_eq!(recipe.output.key, "item.dragon_katana");
        assert!(
            recipe
                .ingredients
                .iter()
                .any(|ingredient| ingredient.content.key == "resource.dragon_heart")
        );

        let active = game_content_registry().export_status(ContentStatus::Active);
        let planned = game_content_registry().export_status(ContentStatus::Planned);
        assert_eq!(active.content.len(), 28);
        assert_eq!(planned.content.len(), 30);
        assert_eq!(active.recipes.len(), 1);
    }
}
