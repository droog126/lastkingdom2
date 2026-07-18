//! Versioned, simulation-only persistence DTOs.

use bevy::prelude::*;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::simulation::regions::{DEFAULT_REGION_SIZE, NatureRegionId, NatureRegionWorld};
use lk2_core::simulation::{NatureSnapshot, cadence::RegionLod};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::authority::{LatestNatureReport, NatureAuthoritySet};
use lk2_core::world::{TerrainEdit, World as GameWorld};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
const LEGACY_NATURE_SCHEMA_VERSION: u32 = 0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureSave<S> {
    pub schema_version: u32,
    pub tick: u64,
    pub state: S,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureRegionSave<S> {
    pub schema_version: u32,
    pub tick: u64,
    pub center: [f32; 2],
    pub radius: f32,
    pub lod: RegionLod,
    pub state: S,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainSave {
    pub schema_version: u32,
    pub revision: u64,
    pub edits: Vec<TerrainEdit>,
}

impl TerrainSave {
    pub fn from_world(world: &GameWorld) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            revision: world.terrain_revision(),
            edits: world.terrain_edits_since(0).copied().collect(),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unsupported terrain save schema");
        }
        let mut previous = 0;
        for edit in &self.edits {
            if edit.revision <= previous {
                return Err("terrain edit revisions must be strictly increasing");
            }
            previous = edit.revision;
        }
        if self.revision < previous {
            return Err("terrain save revision precedes its edit log");
        }
        Ok(())
    }

    pub fn apply_to_world(&self, world: &mut GameWorld) -> Result<(), String> {
        self.validate().map_err(str::to_owned)?;
        for edit in self.edits.iter().copied() {
            world.apply_terrain_edit(edit)?;
        }
        Ok(())
    }
}

#[derive(Resource, Clone, Debug)]
pub struct TerrainSavePath(pub Option<PathBuf>);

impl TerrainSavePath {
    pub fn from_env() -> Self {
        Self(std::env::var_os("LK2_TERRAIN_SAVE_PATH").map(PathBuf::from))
    }
}

/// Optional on-disk path for the authoritative regional ecology state.
///
/// Keeping this opt-in avoids creating save files during focused tests while
/// still giving a running server a real persistence path when configured.
#[derive(Resource, Clone, Debug)]
pub struct NatureRegionSavePath(pub Option<PathBuf>);

impl NatureRegionSavePath {
    pub fn from_env() -> Self {
        Self(std::env::var_os("LK2_NATURE_REGION_SAVE_PATH").map(PathBuf::from))
    }
}

pub fn write_terrain_save_atomic(path: &Path, save: &TerrainSave) -> Result<(), String> {
    save.validate().map_err(str::to_owned)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create terrain save directory: {error}"))?;
    }

    let temp = temporary_path(path);
    let backup = backup_path(path);
    let bytes =
        serde_json::to_vec_pretty(save).map_err(|error| format!("encode terrain save: {error}"))?;
    let mut temp_file = fs::File::create(&temp)
        .map_err(|error| format!("create terrain save temp file: {error}"))?;
    temp_file
        .write_all(&bytes)
        .map_err(|error| format!("write terrain save temp file: {error}"))?;
    temp_file
        .sync_all()
        .map_err(|error| format!("flush terrain save temp file: {error}"))?;

    let _ = fs::remove_file(&backup);
    if path.exists() {
        fs::rename(path, &backup)
            .map_err(|error| format!("stage previous terrain save: {error}"))?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!("commit terrain save: {error}"));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

pub fn read_terrain_save(path: &Path) -> Result<TerrainSave, String> {
    let primary = fs::read_to_string(path);
    let contents = match primary {
        Ok(contents) => contents,
        Err(primary_error) => {
            let backup = backup_path(path);
            fs::read_to_string(&backup).map_err(|backup_error| {
                format!("read terrain save ({primary_error}); backup also failed ({backup_error})")
            })?
        }
    };
    let save: TerrainSave =
        serde_json::from_str(&contents).map_err(|error| format!("decode terrain save: {error}"))?;
    save.validate().map_err(str::to_owned)?;
    Ok(save)
}

fn temporary_path(path: &Path) -> PathBuf {
    path.with_extension(format!("tmp-{}", std::process::id()))
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("bak")
}

impl<S> NatureRegionSave<S> {
    pub fn new(tick: u64, center: [f32; 2], radius: f32, lod: RegionLod, state: S) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            tick,
            center,
            radius,
            lod,
            state,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unsupported nature save schema");
        }
        if !self.center.iter().all(|value| value.is_finite())
            || !self.radius.is_finite()
            || self.radius < 0.0
        {
            return Err("invalid nature region bounds");
        }
        Ok(())
    }
}

impl NatureRegionSave<NatureRegionPersistedState> {
    pub fn validate_persisted(&self) -> Result<(), &'static str> {
        self.validate()?;
        if !self.state.snapshot.is_finite() {
            return Err("nature region snapshot contains non-finite values");
        }
        if self.tick != self.state.snapshot.tick {
            return Err("nature region save tick does not match snapshot tick");
        }
        self.state
            .resources
            .verify_conservation()
            .map_err(|_| "nature region resources violate conservation")
    }
}

impl<S> NatureSave<S> {
    pub fn new(tick: u64, state: S) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            tick,
            state,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err("unsupported nature save schema");
        }
        Ok(())
    }
}

#[derive(Resource, Default)]
pub struct LatestNatureSave(pub Option<NatureSave<NatureSnapshot>>);

#[derive(Resource, Default)]
pub struct LatestNatureRegionSaves(
    pub BTreeMap<NatureRegionId, NatureRegionSave<NatureRegionPersistedState>>,
);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NatureRegionPersistedState {
    pub snapshot: NatureSnapshot,
    pub resources: GlobalResourcePool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct NatureRegionSaveFile {
    schema_version: u32,
    regions: Vec<NatureRegionSaveRecord>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct NatureRegionSaveRecord {
    id: NatureRegionId,
    save: NatureRegionSave<NatureRegionPersistedState>,
}

#[derive(Debug, Deserialize)]
struct NatureRegionSaveEnvelope {
    schema_version: u32,
    regions: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct LegacyNatureRegionSaveRecord {
    id: NatureRegionId,
    save: LegacyNatureRegionSave,
}

#[derive(Debug, Deserialize)]
struct LegacyNatureRegionSave {
    tick: u64,
    center: [f32; 2],
    radius: f32,
    state: NatureRegionPersistedState,
}

pub fn write_nature_region_saves_atomic(
    path: &Path,
    saves: &BTreeMap<NatureRegionId, NatureRegionSave<NatureRegionPersistedState>>,
) -> Result<(), String> {
    for save in saves.values() {
        save.validate_persisted().map_err(str::to_owned)?;
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create nature save directory: {error}"))?;
    }

    let temp = temporary_path(path);
    let backup = backup_path(path);
    let file = NatureRegionSaveFile {
        schema_version: CURRENT_SCHEMA_VERSION,
        regions: saves
            .iter()
            .map(|(id, save)| NatureRegionSaveRecord {
                id: *id,
                save: save.clone(),
            })
            .collect(),
    };
    let bytes = serde_json::to_vec_pretty(&file)
        .map_err(|error| format!("encode nature region saves: {error}"))?;
    let mut temp_file = fs::File::create(&temp)
        .map_err(|error| format!("create nature save temp file: {error}"))?;
    temp_file
        .write_all(&bytes)
        .map_err(|error| format!("write nature save temp file: {error}"))?;
    temp_file
        .sync_all()
        .map_err(|error| format!("flush nature save temp file: {error}"))?;

    let _ = fs::remove_file(&backup);
    if path.exists() {
        fs::rename(path, &backup)
            .map_err(|error| format!("stage previous nature save: {error}"))?;
    }
    if let Err(error) = fs::rename(&temp, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(format!("commit nature save: {error}"));
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

pub fn read_nature_region_saves(
    path: &Path,
) -> Result<BTreeMap<NatureRegionId, NatureRegionSave<NatureRegionPersistedState>>, String> {
    let backup = backup_path(path);
    let primary = fs::read_to_string(path);
    let primary_result = primary
        .and_then(|contents| decode_nature_region_saves(&contents).map_err(std::io::Error::other));
    match primary_result {
        Ok(saves) => Ok(saves),
        Err(primary_error) => {
            let backup_contents = fs::read_to_string(&backup).map_err(|backup_error| {
                format!("read nature save ({primary_error}); backup also failed ({backup_error})")
            })?;
            decode_nature_region_saves(&backup_contents).map_err(|backup_error| {
                format!("decode nature save ({primary_error}); backup also failed ({backup_error})")
            })
        }
    }
}

fn decode_nature_region_saves(
    contents: &str,
) -> Result<BTreeMap<NatureRegionId, NatureRegionSave<NatureRegionPersistedState>>, String> {
    let file = serde_json::from_str::<NatureRegionSaveEnvelope>(contents)
        .map_err(|error| format!("decode nature region saves: {error}"))?;

    if !matches!(
        file.schema_version,
        CURRENT_SCHEMA_VERSION | LEGACY_NATURE_SCHEMA_VERSION
    ) {
        return Err("unsupported nature region save file schema".to_owned());
    }

    let mut saves = BTreeMap::new();
    let mut invalid_records = 0;
    for record in file.regions {
        let decoded = if file.schema_version == CURRENT_SCHEMA_VERSION {
            serde_json::from_value::<NatureRegionSaveRecord>(record)
                .map(|record| (record.id, record.save))
        } else {
            serde_json::from_value::<LegacyNatureRegionSaveRecord>(record).map(|record| {
                let save = record.save;
                (
                    record.id,
                    NatureRegionSave {
                        schema_version: CURRENT_SCHEMA_VERSION,
                        tick: save.tick,
                        center: save.center,
                        radius: save.radius,
                        // Version zero did not persist scheduling state. Active
                        // keeps the old one-tick semantics until a caller
                        // applies its current interest-based LOD.
                        lod: RegionLod::Active,
                        state: save.state,
                    },
                )
            })
        };
        let Ok((id, save)) = decoded else {
            invalid_records += 1;
            continue;
        };
        if save.validate_persisted().is_err() {
            invalid_records += 1;
            continue;
        }
        if saves.insert(id, save).is_some() {
            return Err("nature region save contains duplicate region ids".to_owned());
        }
    }

    if invalid_records > 0 && saves.is_empty() {
        return Err("nature region save contains no valid records".to_owned());
    }
    Ok(saves)
}

#[derive(Resource, Default)]
pub struct LatestTerrainSave(pub Option<TerrainSave>);

pub struct NaturePersistencePlugin;

impl Plugin for NaturePersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureSave>()
            .init_resource::<LatestNatureRegionSaves>()
            .init_resource::<NatureRegionWorld>()
            .init_resource::<LatestTerrainSave>()
            .insert_resource(TerrainSavePath::from_env())
            .insert_resource(NatureRegionSavePath::from_env())
            .add_systems(Startup, load_nature_region_saves)
            .add_systems(
                FixedUpdate,
                (
                    stage_save_snapshot,
                    stage_save_region_snapshots,
                    stage_save_terrain,
                )
                    .in_set(NatureAuthoritySet::PublishReport),
            );
    }
}

pub fn load_nature_region_saves(
    path: Res<NatureRegionSavePath>,
    mut saves: ResMut<LatestNatureRegionSaves>,
) {
    let Some(path) = path.0.as_deref() else {
        return;
    };
    match read_nature_region_saves(path) {
        Ok(loaded) => {
            info!(
                "[nature] restored {} region saves from {:?}",
                loaded.len(),
                path
            );
            saves.0 = loaded;
        }
        Err(error) => warn!("[nature] no usable region save {:?}: {}", path, error),
    }
}

fn stage_save_region_snapshots(
    regions: Res<NatureRegionWorld>,
    path: Res<NatureRegionSavePath>,
    mut saves: ResMut<LatestNatureRegionSaves>,
) {
    let mut changed = false;
    for (id, region) in regions.iter() {
        let snapshot = region.latest_snapshot();
        if !snapshot.is_finite() {
            continue;
        }
        let candidate = NatureRegionSave::new(
            snapshot.tick,
            id.center(DEFAULT_REGION_SIZE),
            DEFAULT_REGION_SIZE * 0.5,
            region.scheduler.lod(),
            NatureRegionPersistedState {
                snapshot,
                resources: region.resources.clone(),
            },
        );
        if saves.0.get(id).is_some_and(|save| save == &candidate) {
            continue;
        }
        saves.0.insert(*id, candidate);
        changed = true;
    }
    if changed {
        if let Some(path) = path.0.as_deref() {
            if let Err(error) = write_nature_region_saves_atomic(path, &saves.0) {
                error!(
                    "[nature] failed to persist region saves {:?}: {}",
                    path, error
                );
            }
        }
    }
}

fn stage_save_snapshot(report: Res<LatestNatureReport>, mut save: ResMut<LatestNatureSave>) {
    let Some(report) = report.0.as_ref() else {
        return;
    };
    if save
        .0
        .as_ref()
        .is_some_and(|value| value.tick == report.tick)
    {
        return;
    }
    save.0 = Some(NatureSave::new(report.tick, report.snapshot.clone()));
}

fn stage_save_terrain(
    world: Res<GameWorld>,
    path: Res<TerrainSavePath>,
    mut save: ResMut<LatestTerrainSave>,
) {
    let revision = world.terrain_revision();
    if save
        .0
        .as_ref()
        .is_some_and(|value| value.revision == revision)
    {
        return;
    }
    let terrain_save = TerrainSave::from_world(&world);
    if let Some(path) = path.0.as_deref() {
        if let Err(error) = write_terrain_save_atomic(path, &terrain_save) {
            error!("[terrain] failed to persist save {:?}: {}", path, error);
        }
    }
    save.0 = Some(terrain_save);
}

#[cfg(test)]
mod tests {
    use super::*;
    use lk2_core::ecology::EcoCycle;
    use lk2_core::world::{BlockType, TerrainChunkCoord, terrain};

    #[test]
    fn nature_region_saves_round_trip_authoritative_state() {
        let id = NatureRegionId { x: 2, z: -1 };
        let ecology = EcoCycle::default();
        let save = NatureRegionSave::new(
            7,
            id.center(DEFAULT_REGION_SIZE),
            DEFAULT_REGION_SIZE * 0.5,
            RegionLod::Nearby,
            NatureRegionPersistedState {
                snapshot: NatureSnapshot::from_ecology(7, &ecology),
                resources: GlobalResourcePool::new(),
            },
        );
        let mut saves = BTreeMap::new();
        saves.insert(id, save);
        let path = std::env::temp_dir().join(format!(
            "lastkingdom2-nature-save-test-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(backup_path(&path));

        write_nature_region_saves_atomic(&path, &saves).expect("nature saves should write");
        assert_eq!(
            read_nature_region_saves(&path).expect("nature saves should read"),
            saves
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(backup_path(&path));
    }

    #[test]
    fn legacy_nature_region_fixture_migrates_to_current_schema() {
        let state = NatureRegionPersistedState {
            snapshot: NatureSnapshot::from_ecology(7, &EcoCycle::default()),
            resources: GlobalResourcePool::new(),
        };
        let fixture = serde_json::json!({
            "schema_version": 0,
            "regions": [{
                "id": { "x": 2, "z": -1 },
                "save": {
                    "tick": 7,
                    "center": [80.0, -16.0],
                    "radius": 16.0,
                    "state": state,
                }
            }]
        });

        let saves = decode_nature_region_saves(
            &serde_json::to_string(&fixture).expect("legacy fixture should encode"),
        )
        .expect("legacy fixture should migrate");
        let save = saves
            .get(&NatureRegionId { x: 2, z: -1 })
            .expect("migrated region should be retained");

        assert_eq!(save.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(save.tick, 7);
        assert_eq!(save.lod, RegionLod::Active);
        assert_eq!(save.state.snapshot.tick, 7);
        assert!(save.validate_persisted().is_ok());
    }

    #[test]
    fn corrupted_region_record_is_skipped_when_other_records_are_valid() {
        let id = NatureRegionId { x: 1, z: 1 };
        let valid = NatureRegionSaveRecord {
            id,
            save: NatureRegionSave::new(
                4,
                id.center(DEFAULT_REGION_SIZE),
                DEFAULT_REGION_SIZE * 0.5,
                RegionLod::Nearby,
                NatureRegionPersistedState {
                    snapshot: NatureSnapshot::from_ecology(4, &EcoCycle::default()),
                    resources: GlobalResourcePool::new(),
                },
            ),
        };
        let fixture = serde_json::json!({
            "schema_version": CURRENT_SCHEMA_VERSION,
            "regions": [
                serde_json::to_value(valid).expect("valid record should encode"),
                { "id": { "x": 9, "z": 9 }, "save": { "schema_version": CURRENT_SCHEMA_VERSION, "state": "broken" } }
            ]
        });

        let saves = decode_nature_region_saves(
            &serde_json::to_string(&fixture).expect("fixture should encode"),
        )
        .expect("one broken record must not reject valid regions");

        assert_eq!(saves.len(), 1);
        assert!(saves.contains_key(&id));
    }

    #[test]
    fn corrupted_primary_nature_save_falls_back_to_backup() {
        let id = NatureRegionId { x: 3, z: 2 };
        let save = NatureRegionSave::new(
            8,
            id.center(DEFAULT_REGION_SIZE),
            DEFAULT_REGION_SIZE * 0.5,
            RegionLod::Active,
            NatureRegionPersistedState {
                snapshot: NatureSnapshot::from_ecology(8, &EcoCycle::default()),
                resources: GlobalResourcePool::new(),
            },
        );
        let mut expected = BTreeMap::new();
        expected.insert(id, save);
        let path = std::env::temp_dir().join(format!(
            "lastkingdom2-nature-backup-test-{}.json",
            std::process::id()
        ));
        let backup = backup_path(&path);
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(&backup);

        write_nature_region_saves_atomic(&path, &expected).expect("nature save should write");
        fs::copy(&path, &backup).expect("backup fixture should copy");
        fs::write(&path, "{ not valid json").expect("primary fixture should corrupt");

        assert_eq!(
            read_nature_region_saves(&path).expect("backup should load"),
            expected
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(backup);
    }

    #[test]
    fn terrain_save_round_trips_edits_and_rejects_duplicate_revisions() {
        let mut world = GameWorld::with_pipeline(32, terrain::presets::default_preset());
        let position = [TerrainChunkCoord::new(0, 0, 0).origin()[0] + 1, 4, 1];
        world.set(position[0], position[1], position[2], BlockType::Stone);

        let save = TerrainSave::from_world(&world);
        assert_eq!(save.edits.len(), 1);
        assert!(save.validate().is_ok());
        let mut restored = GameWorld::with_pipeline(32, terrain::presets::default_preset());
        save.apply_to_world(&mut restored)
            .expect("terrain save should apply to matching world");
        assert_eq!(
            restored.get(position[0], position[1], position[2]),
            BlockType::Stone
        );
        let encoded = serde_json::to_string(&save).expect("terrain save should encode");
        let decoded: TerrainSave =
            serde_json::from_str(&encoded).expect("terrain save should decode");
        assert_eq!(decoded, save);

        let path = std::env::temp_dir().join(format!(
            "lastkingdom2-terrain-save-test-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(backup_path(&path));
        write_terrain_save_atomic(&path, &save).expect("terrain save should write");
        assert_eq!(
            read_terrain_save(&path).expect("terrain save should read"),
            save
        );

        let mut invalid = save.clone();
        invalid.edits.push(invalid.edits[0]);
        assert_eq!(
            invalid.validate(),
            Err("terrain edit revisions must be strictly increasing")
        );
        let _ = std::fs::remove_file(path);
    }
}
