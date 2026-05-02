// src-tauri/src/storage.rs
// Owns user-data persistence: builds.json, settings.json, and timestamped
// backups. Seeds builds.json from a bundled resource on first launch and
// runs forward-only schema migrations driven by SCHEMA_VERSION.

use crate::types::{Build, BuildsData, Race, Settings, UserDataPaths};
use crate::utils;
use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

pub const SCHEMA_VERSION: u32 = 4;
pub const DEFAULT_RATE_LIMIT_MS: u64 = 2300;
pub const DEFAULT_USER_AGENT: &str = concat!(
    "BWBuildOverlay/",
    env!("CARGO_PKG_VERSION"),
    " (local-use; set-your-email-in-settings)"
);

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid builds payload: expected {{ builds: [...] }}")]
    InvalidPayload,
}

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone)]
pub struct UserPaths {
    pub user_data: PathBuf,
    pub user_builds_path: PathBuf,
    pub settings_path: PathBuf,
    pub seed_builds_path: PathBuf,
}

impl UserPaths {
    pub fn new(user_data: PathBuf, seed_builds_path: PathBuf) -> Self {
        let user_builds_path = user_data.join("builds.json");
        let settings_path = user_data.join("settings.json");
        UserPaths {
            user_data,
            user_builds_path,
            settings_path,
            seed_builds_path,
        }
    }

    pub fn to_dto(&self) -> UserDataPaths {
        UserDataPaths {
            user_builds_path: self.user_builds_path.to_string_lossy().into_owned(),
            settings_path: self.settings_path.to_string_lossy().into_owned(),
            user_data: self.user_data.to_string_lossy().into_owned(),
        }
    }
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

pub fn migrate_build(build: Value) -> Build {
    let mut obj = match build {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    let id = obj
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let race_str = obj
        .get("race")
        .and_then(|v| v.as_str())
        .unwrap_or("Protoss")
        .to_string();
    obj.insert("race".to_string(), Value::String(race_str.clone()));

    let opp_str = obj
        .get("opponent")
        .and_then(|v| v.as_str())
        .unwrap_or("Terran")
        .to_string();
    obj.insert("opponent".to_string(), Value::String(opp_str.clone()));

    if !obj.get("matchup").map(|v| v.is_string()).unwrap_or(false) {
        obj.insert(
            "matchup".to_string(),
            Value::String(utils::derive_matchup(&race_str, &opp_str)),
        );
    }

    if !obj.contains_key("variantOf") {
        obj.insert("variantOf".to_string(), Value::Null);
    }
    if !obj.get("tags").map(|v| v.is_array()).unwrap_or(false) {
        obj.insert("tags".to_string(), Value::Array(vec![]));
    }
    if !obj.contains_key("difficulty") {
        obj.insert("difficulty".to_string(), Value::Null);
    }
    if !obj.contains_key("userNotes") {
        obj.insert("userNotes".to_string(), Value::String(String::new()));
    }
    if !obj.contains_key("notes") {
        obj.insert("notes".to_string(), Value::String(String::new()));
    }
    if !obj.contains_key("favorite") {
        obj.insert("favorite".to_string(), Value::Bool(false));
    }
    if !obj.contains_key("recentlyUsedAt") {
        obj.insert("recentlyUsedAt".to_string(), Value::Null);
    }
    if !obj.contains_key("revisionId") {
        obj.insert("revisionId".to_string(), Value::Null);
    }
    if !obj.contains_key("lastImportedAt") {
        obj.insert("lastImportedAt".to_string(), Value::Null);
    }
    if !obj.contains_key("lastCheckedAt") {
        obj.insert("lastCheckedAt".to_string(), Value::Null);
    }
    if !obj.get("counters").map(|v| v.is_array()).unwrap_or(false) {
        obj.insert("counters".to_string(), Value::Array(vec![]));
    }
    if !obj
        .get("counteredBy")
        .map(|v| v.is_array())
        .unwrap_or(false)
    {
        obj.insert("counteredBy".to_string(), Value::Array(vec![]));
    }
    if !obj.get("steps").map(|v| v.is_array()).unwrap_or(false) {
        obj.insert("steps".to_string(), Value::Array(vec![]));
    }
    if !obj.contains_key("customEdited") {
        obj.insert("customEdited".to_string(), Value::Bool(false));
    }
    if !obj.contains_key("sourcePageTitle") {
        obj.insert("sourcePageTitle".to_string(), Value::Null);
    }
    if !obj.contains_key("sourceName") {
        obj.insert(
            "sourceName".to_string(),
            Value::String("Manual".to_string()),
        );
    }
    if !obj.contains_key("sourceUrl") {
        obj.insert("sourceUrl".to_string(), Value::String(String::new()));
    }
    obj.insert("id".to_string(), Value::String(id));

    serde_json::from_value::<Build>(Value::Object(obj))
        .expect("migrate_build: well-defaulted object should always deserialise into Build")
}

pub fn migrate_data(data: Value) -> BuildsData {
    let mut obj = match data {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };

    let raw_builds = obj
        .remove("builds")
        .and_then(|v| match v {
            Value::Array(a) => Some(a),
            _ => None,
        })
        .unwrap_or_default();

    let migrated: Vec<Build> = raw_builds.into_iter().map(migrate_build).collect();
    let last_updated = obj
        .get("lastUpdated")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(today);

    BuildsData {
        version: SCHEMA_VERSION,
        last_updated,
        builds: migrated,
    }
}

async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

async fn read_json_value(path: &Path) -> StorageResult<Value> {
    let bytes = fs::read(path).await?;
    Ok(serde_json::from_slice(&bytes)?)
}

async fn write_json_pretty<T: serde::Serialize>(path: &Path, value: &T) -> StorageResult<()> {
    let pretty = serde_json::to_string_pretty(value)?;
    fs::write(path, pretty).await?;
    Ok(())
}

pub async fn ensure_user_files(paths: &UserPaths) -> StorageResult<()> {
    fs::create_dir_all(&paths.user_data).await?;

    if !file_exists(&paths.user_builds_path).await {
        if file_exists(&paths.seed_builds_path).await {
            fs::copy(&paths.seed_builds_path, &paths.user_builds_path).await?;
        } else {
            let empty = BuildsData {
                version: SCHEMA_VERSION,
                last_updated: today(),
                builds: Vec::new(),
            };
            write_json_pretty(&paths.user_builds_path, &empty).await?;
        }
    }

    if !file_exists(&paths.settings_path).await {
        let defaults = Settings {
            version: 1,
            liquipedia_user_agent: DEFAULT_USER_AGENT.to_string(),
            rate_limit_ms: DEFAULT_RATE_LIMIT_MS,
            compact_overlay: false,
            overlay_opacity: 1.0,
            auto_check_updates_on_launch: false,
            page_size: 25,
            default_race: Race::Protoss,
        };
        write_json_pretty(&paths.settings_path, &defaults).await?;
    }

    let raw = read_json_value(&paths.user_builds_path).await?;
    let stored_version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
    if stored_version < SCHEMA_VERSION {
        let migrated = migrate_data(raw);
        write_json_pretty(&paths.user_builds_path, &migrated).await?;
    }

    Ok(())
}

pub async fn read_builds(paths: &UserPaths) -> StorageResult<BuildsData> {
    ensure_user_files(paths).await?;
    let raw = read_json_value(&paths.user_builds_path).await?;
    Ok(migrate_data(raw))
}

pub async fn save_builds(paths: &UserPaths, mut builds: BuildsData) -> StorageResult<BuildsData> {
    ensure_user_files(paths).await?;
    builds.last_updated = today();
    let migrated = migrate_data(serde_json::to_value(&builds)?);
    write_json_pretty(&paths.user_builds_path, &migrated).await?;
    Ok(migrated)
}

pub async fn read_settings(paths: &UserPaths) -> StorageResult<Settings> {
    ensure_user_files(paths).await?;
    let raw = fs::read(&paths.settings_path).await?;
    Ok(serde_json::from_slice(&raw)?)
}

pub async fn save_settings(paths: &UserPaths, partial: Value) -> StorageResult<Settings> {
    ensure_user_files(paths).await?;
    let existing = read_settings(paths).await?;
    let mut merged = serde_json::to_value(&existing)?;
    if let (Value::Object(target), Value::Object(source)) = (&mut merged, partial) {
        for (k, v) in source {
            target.insert(k, v);
        }
        target.insert("version".to_string(), Value::from(1u32));
    }
    let next: Settings = serde_json::from_value(merged)?;
    write_json_pretty(&paths.settings_path, &next).await?;
    Ok(next)
}

pub async fn backup_builds(paths: &UserPaths) -> StorageResult<String> {
    ensure_user_files(paths).await?;
    let stamp = now_iso().replace([':', '.'], "-");
    let backup_path = paths
        .user_data
        .join(format!("builds-backup-{}.json", stamp));
    fs::copy(&paths.user_builds_path, &backup_path).await?;
    Ok(backup_path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn migrate_build_populates_required_v4_defaults() {
        let v3 = json!({
            "id": "pvt-1-gate-core",
            "race": "Protoss",
            "opponent": "Terran",
            "name": "1 Gate Core",
            "tags": ["standard"],
            "sourceName": "Liquipedia",
            "sourceUrl": "https://liquipedia.net/starcraft/1_Gate_Core_%28vs._Terran%29",
            "notes": "old notes",
            "customEdited": false,
            "steps": ["8 - Pylon", "10 - Gateway"]
        });
        let out = migrate_build(v3);
        assert_eq!(out.matchup, "PvT");
        assert_eq!(out.user_notes, "");
        assert!(!out.favorite);
        assert_eq!(out.notes, "old notes");
    }

    #[tokio::test]
    async fn read_builds_seeds_a_fresh_user_data_folder() {
        let user_data_tmp = tempfile::tempdir().unwrap();
        let seed_tmp = tempfile::tempdir().unwrap();
        let seed_path = seed_tmp.path().join("builds.json");
        std::fs::write(
            &seed_path,
            r#"{ "version": 4, "lastUpdated": "2024-01-01", "builds": [] }"#,
        )
        .unwrap();
        let paths = UserPaths::new(user_data_tmp.path().to_path_buf(), seed_path);

        let data = read_builds(&paths).await.unwrap();
        assert_eq!(data.version, 4);
        assert!(data.builds.is_empty());
    }

    #[tokio::test]
    async fn save_builds_roundtrip_preserves_builds_and_bumps_last_updated() {
        let user_data_tmp = tempfile::tempdir().unwrap();
        let seed_tmp = tempfile::tempdir().unwrap();
        let seed_path = seed_tmp.path().join("builds.json");
        std::fs::write(
            &seed_path,
            r#"{ "version": 4, "lastUpdated": "2024-01-01", "builds": [] }"#,
        )
        .unwrap();
        let paths = UserPaths::new(user_data_tmp.path().to_path_buf(), seed_path);

        let mut data = read_builds(&paths).await.unwrap();
        let sample: Build = serde_json::from_value(json!({
            "id": "test-build",
            "race": "Terran",
            "opponent": "Zerg",
            "matchup": "TvZ",
            "name": "14 CC",
            "variantOf": null,
            "tags": ["macro"],
            "difficulty": "intermediate",
            "sourceName": "Manual",
            "sourceUrl": "",
            "sourcePageTitle": null,
            "notes": "",
            "userNotes": "",
            "customEdited": true,
            "favorite": false,
            "recentlyUsedAt": null,
            "revisionId": null,
            "lastImportedAt": null,
            "lastCheckedAt": null,
            "counters": [],
            "counteredBy": [],
            "steps": ["9 - Supply Depot", "14 - Command Center"]
        }))
        .unwrap();
        data.builds.push(sample);

        let saved = save_builds(&paths, data).await.unwrap();
        assert_eq!(saved.last_updated, today());

        let reloaded = read_builds(&paths).await.unwrap();
        let round = reloaded
            .builds
            .iter()
            .find(|b| b.id == "test-build")
            .expect("saved build should be readable on next load");
        assert_eq!(round.matchup, "TvZ");
        assert_eq!(round.steps.len(), 2);
    }

    #[tokio::test]
    async fn settings_roundtrip_preserves_merged_values() {
        let user_data_tmp = tempfile::tempdir().unwrap();
        let seed_tmp = tempfile::tempdir().unwrap();
        let seed_path = seed_tmp.path().join("builds.json");
        std::fs::write(
            &seed_path,
            r#"{ "version": 4, "lastUpdated": "2024-01-01", "builds": [] }"#,
        )
        .unwrap();
        let paths = UserPaths::new(user_data_tmp.path().to_path_buf(), seed_path);

        save_settings(
            &paths,
            json!({ "rateLimitMs": 4000, "compactOverlay": true }),
        )
        .await
        .unwrap();
        let settings = read_settings(&paths).await.unwrap();
        assert_eq!(settings.rate_limit_ms, 4000);
        assert!(settings.compact_overlay);
        assert!(!settings.liquipedia_user_agent.is_empty());
    }

    #[test]
    fn migrate_data_normalises_envelope_and_stamps_schema_version() {
        let pre_v4 = json!({
            "version": 1,
            "lastUpdated": "2024-06-15",
            "builds": [
                { "id": "old", "race": "Zerg", "opponent": "Protoss",
                  "name": "9 Pool", "steps": ["9 - Spawning Pool"] }
            ]
        });
        let migrated = migrate_data(pre_v4);
        assert_eq!(migrated.version, SCHEMA_VERSION);
        assert_eq!(migrated.last_updated, "2024-06-15");
        assert_eq!(migrated.builds.len(), 1);
        let only = &migrated.builds[0];
        assert_eq!(only.matchup, "ZvP");
        assert_eq!(only.user_notes, "");
        assert!(only.tags.is_empty());
        assert!(!only.custom_edited);
    }

    #[tokio::test]
    async fn backup_builds_creates_timestamped_copy_of_user_builds() {
        let user_data_tmp = tempfile::tempdir().unwrap();
        let seed_tmp = tempfile::tempdir().unwrap();
        let seed_path = seed_tmp.path().join("builds.json");
        std::fs::write(
            &seed_path,
            r#"{ "version": 4, "lastUpdated": "2024-01-01", "builds": [] }"#,
        )
        .unwrap();
        let paths = UserPaths::new(user_data_tmp.path().to_path_buf(), seed_path);

        let backup_path_str = backup_builds(&paths).await.unwrap();
        let backup_path = std::path::Path::new(&backup_path_str);
        assert!(backup_path.exists(), "backup file should be on disk");
        assert!(
            backup_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .starts_with("builds-backup-"),
            "backup filename should be timestamped: {backup_path_str}"
        );

        let original = std::fs::read_to_string(&paths.user_builds_path).unwrap();
        let copied = std::fs::read_to_string(backup_path).unwrap();
        assert_eq!(
            original, copied,
            "backup should be a byte-identical copy of builds.json"
        );
    }
}
