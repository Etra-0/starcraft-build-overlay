// src-tauri/src/types.rs
// Domain types shared across storage, the Liquipedia client, and the IPC
// command surface. Mirrors src/shared/types.ts so the on-disk JSON layout is
// identical between the Rust backend and the TypeScript renderer.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Race {
    Terran,
    Protoss,
    Zerg,
}

impl Race {
    pub fn initial(&self) -> char {
        match self {
            Race::Terran => 'T',
            Race::Protoss => 'P',
            Race::Zerg => 'Z',
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Race::Terran => "Terran",
            Race::Protoss => "Protoss",
            Race::Zerg => "Zerg",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Opponent {
    Terran,
    Protoss,
    Zerg,
    Random,
}

impl Opponent {
    pub fn initial(&self) -> char {
        match self {
            Opponent::Terran => 'T',
            Opponent::Protoss => 'P',
            Opponent::Zerg => 'Z',
            Opponent::Random => 'R',
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Opponent::Terran => "Terran",
            Opponent::Protoss => "Protoss",
            Opponent::Zerg => "Zerg",
            Opponent::Random => "Random",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Build {
    pub id: String,
    pub race: Race,
    pub opponent: Opponent,
    pub matchup: String,
    pub name: String,
    #[serde(default)]
    #[serde(rename = "variantOf")]
    pub variant_of: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub difficulty: Option<Difficulty>,
    #[serde(default = "default_source_name")]
    #[serde(rename = "sourceName")]
    pub source_name: String,
    #[serde(default)]
    #[serde(rename = "sourceUrl")]
    pub source_url: String,
    #[serde(default)]
    #[serde(rename = "sourcePageTitle")]
    pub source_page_title: Option<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    #[serde(rename = "userNotes")]
    pub user_notes: String,
    #[serde(default)]
    #[serde(rename = "customEdited")]
    pub custom_edited: bool,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    #[serde(rename = "recentlyUsedAt")]
    pub recently_used_at: Option<String>,
    #[serde(default)]
    #[serde(rename = "revisionId")]
    pub revision_id: Option<i64>,
    #[serde(default)]
    #[serde(rename = "lastImportedAt")]
    pub last_imported_at: Option<String>,
    #[serde(default)]
    #[serde(rename = "lastCheckedAt")]
    pub last_checked_at: Option<String>,
    #[serde(default)]
    pub counters: Vec<String>,
    #[serde(default)]
    #[serde(rename = "counteredBy")]
    pub countered_by: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
}

fn default_source_name() -> String {
    "Manual".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildsData {
    #[serde(default)]
    pub version: u32,
    #[serde(default, rename = "lastUpdated")]
    pub last_updated: String,
    #[serde(default)]
    pub builds: Vec<Build>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_settings_version")]
    pub version: u32,
    #[serde(rename = "liquipediaUserAgent")]
    pub liquipedia_user_agent: String,
    #[serde(rename = "rateLimitMs")]
    pub rate_limit_ms: u64,
    #[serde(rename = "compactOverlay")]
    pub compact_overlay: bool,
    #[serde(rename = "overlayOpacity")]
    pub overlay_opacity: f64,
    #[serde(rename = "autoCheckUpdatesOnLaunch")]
    pub auto_check_updates_on_launch: bool,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "defaultRace")]
    pub default_race: Race,
}

fn default_settings_version() -> u32 {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            liquipedia_user_agent: crate::storage::DEFAULT_USER_AGENT.to_string(),
            rate_limit_ms: crate::storage::DEFAULT_RATE_LIMIT_MS,
            compact_overlay: false,
            overlay_opacity: 1.0,
            auto_check_updates_on_launch: false,
            page_size: 25,
            default_race: Race::Protoss,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDataPaths {
    pub user_builds_path: String,
    pub settings_path: String,
    pub user_data: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParsedInfobox {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<Race>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matchups: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub popularized: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedVariant {
    pub variant_name: String,
    pub heading: Option<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedLiquipediaPage {
    pub infobox: Option<ParsedInfobox>,
    pub player_race: Option<Race>,
    pub opponent: Option<Opponent>,
    pub difficulty: Option<Difficulty>,
    pub counters: Vec<String>,
    pub countered_by: Vec<String>,
    pub variants: Vec<ParsedVariant>,
}

/// Race-or-Random helper used by the wikitext parser when normalising the
/// `R` from `PvR` etc. before deciding if it counts as a player race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceOrRandom {
    Race(Race),
    Random,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BulkImportMode {
    #[default]
    Common,
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    #[serde(default)]
    pub update_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportOptions {
    #[serde(default)]
    pub races: Vec<Race>,
    #[serde(default = "default_bulk_mode")]
    pub mode: BulkImportMode,
    #[serde(default)]
    pub update_existing: bool,
}

fn default_bulk_mode() -> BulkImportMode {
    BulkImportMode::Common
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImportMergeResult {
    Added,
    Updated,
    Skipped,
    SkippedCustom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSinglePageResult {
    pub page_title: String,
    pub added_or_updated: Vec<Build>,
    pub results: Vec<ImportMergeResult>,
    pub total_variants: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkImportResult {
    pub mode: BulkImportMode,
    pub races: Vec<Race>,
    pub selected: usize,
    pub total_discovered: usize,
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
    pub variants_total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UpdateReason {
    NewerRevision,
    NoStoredRev,
    MissingOnServer,
    UpToDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub build_id: String,
    pub name: String,
    pub matchup: String,
    pub source_page_title: String,
    pub source_url: String,
    pub current_rev_id: Option<i64>,
    pub latest_rev_id: Option<i64>,
    pub latest_timestamp: Option<String>,
    pub outdated: bool,
    pub reason: UpdateReason,
    pub custom_edited: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckUpdatesResult {
    pub checked: usize,
    pub outdated: Vec<UpdateInfo>,
    pub all: Vec<UpdateInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RefreshBuildsOptions {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshBuildsFailure {
    pub build_id: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshBuildsResult {
    pub refreshed: Vec<String>,
    pub failed: Vec<RefreshBuildsFailure>,
    pub skipped_custom: Vec<String>,
}

/// Discriminated hotkey union; serialised as the kebab-case string the
/// renderer's existing `HotkeyAction` union expects ("race-terran" etc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HotkeyAction {
    RaceTerran,
    RaceProtoss,
    RaceZerg,
    OppTerran,
    OppZerg,
    OppProtoss,
    OppRandom,
    NextBuild,
    PrevBuild,
    NextPage,
    PrevPage,
    FirstPage,
    ToggleFavorite,
    ToggleCompact,
    ToggleWindow,
}
