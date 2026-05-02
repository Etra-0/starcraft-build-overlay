// src-tauri/src/liquipedia/import.rs
// High-level import orchestration: single-page preview/import and bulk
// category walks per race ("common" curated subset vs. "all"). Mirrors
// src/main/liquipedia/import.ts: turns a parsed page into one Build per
// {{build}} variant, links variants via variantOf, and merges into
// builds.json while preserving favorite/userNotes/customEdited.

use crate::liquipedia::api::{self, ApiError, ApiResult, LiquipediaCategoryMember};
use crate::liquipedia::parser;
use crate::storage::{self, StorageError, UserPaths};
use crate::types::{
    Build, BuildsData, BulkImportMode, BulkImportOptions, BulkImportResult, ImportMergeResult,
    ImportSinglePageResult, Opponent, ParsedLiquipediaPage, ParsedVariant, Race,
};
use crate::utils;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

pub type ImportResult<T> = Result<T, ImportError>;

pub type ProgressFn = std::sync::Arc<dyn Fn(&str) + Send + Sync>;

static REGEX_PAREN_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*\(.*?\)\s*$").unwrap());

static COMMON_TITLE_PATTERNS_SRC: &[&str] = &[
    r"(?i)1\s*Gate\s*Core",
    r"(?i)10/15\s*Gates",
    r"(?i)2\s*Gate\s*Range",
    r"(?i)12\s*Nexus",
    r"(?i)14\s*Nexus",
    r"(?i)Reaver",
    r"(?i)Dark\s*Templar",
    r"(?i)Forge\s*FE",
    r"(?i)Forge\s*Fast\s*Expand",
    r"(?i)2\s*Gateway",
    r"(?i)1\s*Gate\s*Stargate",
    r"(?i)Corsair",
    r"(?i)3\s*Gate\s*Robo",
    r"(?i)Gateway\s*Robo",
    r"(?i)Observer",
    r"(?i)4\s*Gate",
    r"(?i)Goon",
    r"(?i)14\s*CC",
    r"(?i)1\s*Rax\s*FE",
    r"(?i)2\s*Rax",
    r"(?i)Siege\s*Expand",
    r"(?i)1\s*Fact",
    r"(?i)2\s*Fact",
    r"(?i)SK\s*Terran",
    r"(?i)Mech",
    r"(?i)Wraith",
    r"(?i)BBS",
    r"(?i)9\s*Pool",
    r"(?i)Overpool",
    r"(?i)12\s*Pool",
    r"(?i)12\s*Hatch",
    r"(?i)3\s*Hatch",
    r"(?i)5\s*Hatch",
    r"(?i)2\s*Hatch",
    r"(?i)Lurker",
    r"(?i)Mutalisk",
    r"(?i)Hydralisk",
    r"(?i)Hatch\s*first",
];

static COMMON_TITLE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    COMMON_TITLE_PATTERNS_SRC
        .iter()
        .map(|p| Regex::new(p).expect("static common-title pattern compiles"))
        .collect()
});

pub fn race_roots(race: Race) -> &'static [&'static str] {
    match race {
        Race::Protoss => &[
            "Category:Protoss_Build_Orders",
            "Category:Protoss_Builds",
            "Category:Protoss_Strategy",
        ],
        Race::Terran => &[
            "Category:Terran_Build_Orders",
            "Category:Terran_Builds",
            "Category:Terran_Strategy",
        ],
        Race::Zerg => &[
            "Category:Zerg_Build_Orders",
            "Category:Zerg_Builds",
            "Category:Zerg_Strategy",
        ],
    }
}

pub fn is_common_title(title: &str) -> bool {
    COMMON_TITLE_PATTERNS.iter().any(|p| p.is_match(title))
}

fn send_progress(progress: Option<&ProgressFn>, message: &str) {
    if let Some(cb) = progress {
        cb(message);
    }
}

fn make_build_id(player_race: &str, opponent: &str, name: &str, variant_name: &str) -> String {
    let race = if player_race.is_empty() {
        "unknown"
    } else {
        player_race
    };
    let opp = if opponent.is_empty() { "any" } else { opponent };
    let base = if variant_name.is_empty() {
        name.to_string()
    } else {
        format!("{}-{}", name, variant_name)
    };
    format!(
        "liquipedia-{}-{}-{}",
        utils::slugify(race),
        utils::slugify(opp),
        utils::slugify(&base)
    )
}

fn strip_paren_suffix(title: &str) -> String {
    REGEX_PAREN_SUFFIX.replace(title, "").trim().to_string()
}

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

fn build_from_variant(
    page_title: &str,
    parsed: &ParsedLiquipediaPage,
    variant: &ParsedVariant,
    revision_id: Option<i64>,
) -> Build {
    let race = parsed.player_race.unwrap_or(Race::Protoss);
    let opponent = parsed.opponent.unwrap_or(Opponent::Random);
    let base_name = strip_paren_suffix(page_title);
    let final_name = if variant.variant_name.is_empty() {
        base_name.clone()
    } else {
        format!("{} - {}", base_name, variant.variant_name)
    };
    let id = make_build_id(
        race.as_str(),
        opponent.as_str(),
        &base_name,
        &variant.variant_name,
    );

    let mut tags: Vec<String> = vec!["liquipedia".to_string()];
    if !variant.steps.is_empty() {
        tags.push("imported".to_string());
    } else {
        tags.push("needs-review".to_string());
    }
    if let Some(d) = parsed.difficulty {
        tags.push(
            match d {
                crate::types::Difficulty::Beginner => "beginner",
                crate::types::Difficulty::Intermediate => "intermediate",
                crate::types::Difficulty::Advanced => "advanced",
            }
            .to_string(),
        );
    }

    let creator = parsed
        .infobox
        .as_ref()
        .and_then(|i| i.creator.clone())
        .unwrap_or_default();
    let popularized = parsed
        .infobox
        .as_ref()
        .and_then(|i| i.popularized.clone())
        .unwrap_or_default();

    let notes = if !variant.steps.is_empty() {
        let mut s = "Imported from Liquipedia.".to_string();
        if !creator.is_empty() {
            s.push_str(&format!(" Creator: {}.", creator));
        }
        if !popularized.is_empty() {
            s.push_str(&format!(" Popularized: {}.", popularized));
        }
        s.trim().to_string()
    } else {
        "Could not parse a build template - open Source and copy the order manually.".to_string()
    };

    let steps = if variant.steps.is_empty() {
        vec!["No build steps were parsed — add them manually from the Liquipedia page source or in the Build Manager."
            .to_string()]
    } else {
        variant.steps.clone()
    };

    let now = now_iso();
    Build {
        id,
        race,
        opponent,
        matchup: utils::derive_matchup_typed(race, opponent),
        name: final_name,
        variant_of: None,
        tags,
        difficulty: parsed.difficulty,
        source_name: "Liquipedia".to_string(),
        source_url: api::page_url(page_title),
        source_page_title: Some(page_title.to_string()),
        notes,
        user_notes: String::new(),
        custom_edited: false,
        favorite: false,
        recently_used_at: None,
        revision_id,
        last_imported_at: Some(now.clone()),
        last_checked_at: Some(now),
        counters: parsed.counters.clone(),
        countered_by: parsed.countered_by.clone(),
        steps,
    }
}

pub fn builds_from_page(
    page_title: &str,
    parsed: &ParsedLiquipediaPage,
    revision_id: Option<i64>,
) -> Vec<Build> {
    let mut variants: Vec<ParsedVariant> = parsed.variants.clone();
    let mut counts: HashMap<String, u32> = HashMap::new();
    for v in &variants {
        let key = v.variant_name.to_lowercase();
        *counts.entry(key).or_insert(0) += 1;
    }
    for v in variants.iter_mut() {
        let key = v.variant_name.to_lowercase();
        if counts.get(&key).copied().unwrap_or(0) > 1 {
            if let Some(h) = &v.heading {
                v.variant_name = h.clone();
            }
        }
    }

    let mut out: Vec<Build> = variants
        .iter()
        .map(|v| build_from_variant(page_title, parsed, v, revision_id))
        .collect();
    if out.len() > 1 {
        let parent_id = out[0].id.clone();
        for next in out.iter_mut().skip(1) {
            next.variant_of = Some(parent_id.clone());
        }
    }
    out
}

pub fn merge_build(
    data: &mut BuildsData,
    imported: Build,
    update_existing: bool,
) -> ImportMergeResult {
    let position = data.builds.iter().position(|b| {
        let same_source = match (&b.source_page_title, &imported.source_page_title) {
            (Some(a), Some(b2)) => {
                !a.is_empty() && !b2.is_empty() && a == b2 && b.name == imported.name
            }
            _ => false,
        };
        same_source || b.id == imported.id
    });

    let position = match position {
        None => {
            data.builds.push(imported);
            return ImportMergeResult::Added;
        }
        Some(idx) => idx,
    };

    if !update_existing {
        return ImportMergeResult::Skipped;
    }
    if data.builds[position].custom_edited {
        return ImportMergeResult::SkippedCustom;
    }

    let preserved_id = data.builds[position].id.clone();
    let preserved_custom = data.builds[position].custom_edited;
    let preserved_favorite = data.builds[position].favorite;
    let preserved_user_notes = data.builds[position].user_notes.clone();
    let preserved_recently = data.builds[position].recently_used_at.clone();

    let mut next = imported;
    next.id = preserved_id;
    next.custom_edited = preserved_custom;
    next.favorite = preserved_favorite;
    next.user_notes = preserved_user_notes;
    next.recently_used_at = preserved_recently;

    data.builds[position] = next;
    ImportMergeResult::Updated
}

pub async fn preview_single_page(
    paths: &UserPaths,
    input: &str,
    progress: Option<&ProgressFn>,
) -> ImportResult<Vec<Build>> {
    let settings = storage::read_settings(paths).await?;
    let title = api::parse_liquipedia_title(input)?;
    send_progress(progress, &format!("Fetching preview: {}", title));
    let page = api::get_page_wikitext(&title, &settings).await?;
    let parsed = parser::parse_liquipedia_page(&page.page_title, &page.wikitext);
    Ok(builds_from_page(
        &page.page_title,
        &parsed,
        page.revision_id,
    ))
}

pub async fn import_single_page(
    paths: &UserPaths,
    input: &str,
    update_existing: bool,
    progress: Option<&ProgressFn>,
) -> ImportResult<ImportSinglePageResult> {
    let settings = storage::read_settings(paths).await?;
    let mut data = storage::read_builds(paths).await?;
    let title = api::parse_liquipedia_title(input)?;
    send_progress(progress, &format!("Fetching: {}", title));
    let page = api::get_page_wikitext(&title, &settings).await?;
    let parsed = parser::parse_liquipedia_page(&page.page_title, &page.wikitext);
    if parsed.variants.is_empty() {
        send_progress(
            progress,
            &format!("No build templates found on {}.", page.page_title),
        );
    }
    let builds = builds_from_page(&page.page_title, &parsed, page.revision_id);
    let mut results: Vec<ImportMergeResult> = Vec::with_capacity(builds.len());
    for b in builds.iter() {
        results.push(merge_build(&mut data, b.clone(), update_existing));
    }
    let _ = storage::save_builds(paths, data).await?;
    let total = builds.len();
    Ok(ImportSinglePageResult {
        page_title: page.page_title,
        added_or_updated: builds,
        results,
        total_variants: total,
    })
}

async fn get_race_category_titles(
    race: Race,
    settings: &crate::types::Settings,
    progress: Option<&ProgressFn>,
) -> ApiResult<Vec<String>> {
    let mut seen_categories: HashSet<String> = HashSet::new();
    let mut seen_pages: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = race_roots(race).iter().map(|s| s.to_string()).collect();
    let nested_re =
        Regex::new(&format!(r"(?i){}|Pv|Tv|Zv|Build", race.as_str())).expect("nested filter");
    let category_prefix_re = Regex::new(r"(?i)^Category:").expect("category prefix");

    while let Some(category) = queue.pop() {
        if seen_categories.contains(&category) {
            continue;
        }
        seen_categories.insert(category.clone());
        send_progress(progress, &format!("Reading category: {}", category));
        let members: Vec<LiquipediaCategoryMember> =
            match api::get_category_members(&category, settings).await {
                Ok(m) => m,
                Err(err) => {
                    send_progress(progress, &format!("Skipped {}: {}", category, err));
                    continue;
                }
            };
        for member in members {
            if member.ns == 14 && category_prefix_re.is_match(&member.title) {
                if nested_re.is_match(&member.title) {
                    queue.push(member.title);
                }
            } else if member.ns == 0 {
                seen_pages.insert(member.title);
            }
        }
    }

    let mut out: Vec<String> = seen_pages.into_iter().collect();
    out.sort();
    Ok(out)
}

pub async fn bulk_import(
    paths: &UserPaths,
    options: BulkImportOptions,
    progress: Option<&ProgressFn>,
) -> ImportResult<BulkImportResult> {
    let settings = storage::read_settings(paths).await?;
    let mut data = storage::read_builds(paths).await?;
    let races: Vec<Race> = if options.races.is_empty() {
        vec![Race::Protoss]
    } else {
        options.races.clone()
    };

    let mut all_titles: Vec<(Race, String)> = Vec::new();
    for race in &races {
        let titles = get_race_category_titles(*race, &settings, progress).await?;
        for t in titles {
            all_titles.push((*race, t));
        }
    }
    let mut seen: HashSet<String> = HashSet::new();
    all_titles.retain(|(_, title)| seen.insert(title.clone()));
    let total_discovered = all_titles.len();

    let filtered: Vec<(Race, String)> = match options.mode {
        BulkImportMode::All => all_titles,
        BulkImportMode::Common => all_titles
            .into_iter()
            .filter(|(_, title)| is_common_title(title))
            .collect(),
    };
    send_progress(
        progress,
        &format!(
            "Found {} page(s). Selected {} page(s).",
            total_discovered,
            filtered.len()
        ),
    );

    let mut added = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;
    let mut variants_total = 0u32;

    let selected = filtered.len();
    for (_, title) in filtered {
        send_progress(progress, &format!("Fetching: {}", title));
        let page = match api::get_page_wikitext(&title, &settings).await {
            Ok(p) => p,
            Err(err) => {
                failed += 1;
                send_progress(progress, &format!("Failed: {} - {}", title, err));
                continue;
            }
        };
        let parsed = parser::parse_liquipedia_page(&page.page_title, &page.wikitext);
        let builds = builds_from_page(&page.page_title, &parsed, page.revision_id);
        variants_total += builds.len() as u32;
        for b in builds {
            match merge_build(&mut data, b, options.update_existing) {
                ImportMergeResult::Added => added += 1,
                ImportMergeResult::Updated => updated += 1,
                ImportMergeResult::Skipped | ImportMergeResult::SkippedCustom => skipped += 1,
            }
        }
    }
    storage::save_builds(paths, data).await?;

    Ok(BulkImportResult {
        mode: options.mode,
        races,
        selected,
        total_discovered,
        added,
        updated,
        skipped,
        failed,
        variants_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::liquipedia::parser;

    #[test]
    fn builds_from_page_links_variants_via_variant_of() {
        let wikitext = [
            "{{Infobox strategy",
            "|name=1 Gate Cybernetics Core",
            "|race=P",
            "|matchups=PvT",
            "}}",
            "==Build Order==",
            "===No Zealot before Cybernetics Core===",
            "{{build|name=\"One Gate Cybernetics Core\"|race=Protoss|",
            "*8 - Pylon",
            "*10 - Gateway",
            "}}",
            "===One Zealot before Cybernetics Core===",
            "{{build|name=\"One Gate Cybernetics Core\"|race=Protoss|",
            "*8 - Pylon",
            "*10 - Gate",
            "*13 - Zealot",
            "}}",
        ]
        .join("\n");
        let parsed = parser::parse_liquipedia_page("1 Gate Core (vs. Terran)", &wikitext);
        let builds = builds_from_page("1 Gate Core (vs. Terran)", &parsed, Some(99999));
        assert_eq!(builds.len(), 2);
        assert_ne!(builds[0].id, builds[1].id);
        assert_eq!(builds[1].variant_of.as_deref(), Some(builds[0].id.as_str()));
    }

    #[test]
    fn is_common_title_recognises_curated_patterns() {
        assert!(is_common_title("1 Gate Core (vs. Terran)"));
        assert!(is_common_title("9 Pool"));
        assert!(is_common_title("Forge FE"));
        assert!(!is_common_title("Random Garbage Build"));
    }
}
