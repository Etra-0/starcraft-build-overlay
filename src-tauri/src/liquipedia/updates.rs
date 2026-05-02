// src-tauri/src/liquipedia/updates.rs
// Diffs each Liquipedia-sourced build's stored revisionId against the latest
// revision on the wiki (one batched API call) and exposes single / multi-
// build refresh that re-imports a page while preserving favorite, userNotes,
// and recentlyUsedAt. Skips customEdited unless forced. Mirrors
// src/main/liquipedia/updates.ts.

use crate::liquipedia::api;
use crate::liquipedia::import::{builds_from_page, ImportError, ImportResult, ProgressFn};
use crate::liquipedia::parser;
use crate::storage::{self, UserPaths};
use crate::types::{
    Build, CheckUpdatesResult, RefreshBuildsFailure, RefreshBuildsOptions, RefreshBuildsResult,
    UpdateInfo, UpdateReason,
};
use regex::Regex;
use std::collections::HashSet;

fn send_progress(progress: Option<&ProgressFn>, message: &str) {
    if let Some(cb) = progress {
        cb(message);
    }
}

fn now_iso() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

pub async fn check_for_updates(
    paths: &UserPaths,
    progress: Option<&ProgressFn>,
) -> ImportResult<CheckUpdatesResult> {
    let settings = storage::read_settings(paths).await?;
    let mut data = storage::read_builds(paths).await?;

    let liquipedia_titles: Vec<String> = {
        let mut seen = HashSet::new();
        data.builds
            .iter()
            .filter(|b| b.source_name == "Liquipedia")
            .filter_map(|b| b.source_page_title.clone())
            .filter(|t| !t.is_empty())
            .filter(|t| seen.insert(t.clone()))
            .collect()
    };
    let total_liquipedia = data
        .builds
        .iter()
        .filter(|b| b.source_name == "Liquipedia" && b.source_page_title.is_some())
        .count();

    if liquipedia_titles.is_empty() {
        return Ok(CheckUpdatesResult {
            checked: 0,
            outdated: vec![],
            all: vec![],
        });
    }
    send_progress(
        progress,
        &format!(
            "Checking {} Liquipedia page(s) for updates...",
            liquipedia_titles.len()
        ),
    );

    let revs_by_title = api::get_revisions_for_titles(&liquipedia_titles, &settings).await?;
    let now = now_iso();

    let mut all: Vec<UpdateInfo> = Vec::new();
    for b in data
        .builds
        .iter()
        .filter(|b| b.source_name == "Liquipedia" && b.source_page_title.is_some())
    {
        let title = b.source_page_title.clone().unwrap_or_default();
        let rev = revs_by_title.get(&title);
        let latest_rev_id = rev.and_then(|r| r.revision_id);
        let latest_timestamp = rev.and_then(|r| r.revision_timestamp.clone());
        let outdated = matches!((latest_rev_id, b.revision_id), (Some(latest), Some(stored)) if latest != stored);
        let unknown = b.revision_id.is_none() && latest_rev_id.is_some();
        let reason = if rev.is_none() {
            UpdateReason::MissingOnServer
        } else if outdated {
            UpdateReason::NewerRevision
        } else if unknown {
            UpdateReason::NoStoredRev
        } else {
            UpdateReason::UpToDate
        };
        all.push(UpdateInfo {
            build_id: b.id.clone(),
            name: b.name.clone(),
            matchup: b.matchup.clone(),
            source_page_title: title.clone(),
            source_url: if b.source_url.is_empty() {
                api::page_url(&title)
            } else {
                b.source_url.clone()
            },
            current_rev_id: b.revision_id,
            latest_rev_id,
            latest_timestamp,
            outdated: outdated || unknown,
            reason,
            custom_edited: b.custom_edited,
        });
    }

    for b in data
        .builds
        .iter_mut()
        .filter(|b| b.source_name == "Liquipedia" && b.source_page_title.is_some())
    {
        b.last_checked_at = Some(now.clone());
    }
    storage::save_builds(paths, data).await?;

    let outdated: Vec<UpdateInfo> = all.iter().filter(|u| u.outdated).cloned().collect();
    Ok(CheckUpdatesResult {
        checked: total_liquipedia,
        outdated,
        all,
    })
}

fn variant_suffix_for(name: &str, page_title: &str) -> String {
    let paren_re = Regex::new(r"\s*\(.*?\)\s*$").expect("paren-suffix");
    let trimmed_title = paren_re.replace(page_title, "").trim().to_string();
    let prefix = format!("{} - ", trimmed_title);
    if let Some(rest) = name.strip_prefix(&prefix) {
        rest.trim().to_string()
    } else {
        name.replace(&prefix, "").trim().to_string()
    }
}

fn pick_chosen<'a>(fresh: &'a [Build], existing_name: &str, page_title: &str) -> Option<&'a Build> {
    if let Some(exact) = fresh.iter().find(|f| f.name == existing_name) {
        return Some(exact);
    }
    let suffix = variant_suffix_for(existing_name, page_title);
    if !suffix.is_empty() {
        if let Some(by_suffix) = fresh.iter().find(|f| f.name.ends_with(&suffix)) {
            return Some(by_suffix);
        }
    }
    fresh.first()
}

fn apply_refreshed(existing: &mut Build, mut chosen: Build) {
    let preserved_id = std::mem::take(&mut existing.id);
    let preserved_favorite = existing.favorite;
    let preserved_user_notes = std::mem::take(&mut existing.user_notes);
    let preserved_recently = existing.recently_used_at.take();

    chosen.id = preserved_id;
    chosen.custom_edited = false;
    chosen.favorite = preserved_favorite;
    chosen.user_notes = preserved_user_notes;
    chosen.recently_used_at = preserved_recently;

    *existing = chosen;
}

pub async fn refresh_build(
    paths: &UserPaths,
    build_id: &str,
    progress: Option<&ProgressFn>,
) -> ImportResult<Build> {
    let settings = storage::read_settings(paths).await?;
    let mut data = storage::read_builds(paths).await?;
    let position = data
        .builds
        .iter()
        .position(|b| b.id == build_id)
        .ok_or_else(|| ImportError::Api(api::ApiError::NotFound("Build not found.".to_string())))?;

    let title = data.builds[position]
        .source_page_title
        .clone()
        .ok_or_else(|| {
            ImportError::Api(api::ApiError::NotFound(
                "This build has no Liquipedia source.".to_string(),
            ))
        })?;
    let existing_name = data.builds[position].name.clone();
    send_progress(progress, &format!("Refreshing: {}", existing_name));

    let page = api::get_page_wikitext(&title, &settings).await?;
    let parsed = parser::parse_liquipedia_page(&page.page_title, &page.wikitext);
    let fresh = builds_from_page(&page.page_title, &parsed, page.revision_id);
    let chosen = pick_chosen(&fresh, &existing_name, &page.page_title)
        .cloned()
        .ok_or_else(|| {
            ImportError::Api(api::ApiError::NotFound(
                "Source page no longer contains a parseable build.".to_string(),
            ))
        })?;
    apply_refreshed(&mut data.builds[position], chosen);

    let saved = storage::save_builds(paths, data).await?;
    let refreshed = saved
        .builds
        .iter()
        .find(|b| b.id == build_id)
        .cloned()
        .ok_or_else(|| {
            ImportError::Api(api::ApiError::NotFound(
                "Refreshed build vanished from storage.".to_string(),
            ))
        })?;
    Ok(refreshed)
}

pub async fn refresh_builds(
    paths: &UserPaths,
    build_ids: &[String],
    options: RefreshBuildsOptions,
    progress: Option<&ProgressFn>,
) -> ImportResult<RefreshBuildsResult> {
    let settings = storage::read_settings(paths).await?;
    let mut data = storage::read_builds(paths).await?;
    let mut refreshed: Vec<String> = Vec::new();
    let mut failed: Vec<RefreshBuildsFailure> = Vec::new();
    let mut skipped_custom: Vec<String> = Vec::new();

    for build_id in build_ids {
        let position = match data.builds.iter().position(|b| &b.id == build_id) {
            Some(p) => p,
            None => {
                failed.push(RefreshBuildsFailure {
                    build_id: build_id.clone(),
                    error: "not-found".to_string(),
                });
                continue;
            }
        };
        let title = match &data.builds[position].source_page_title {
            Some(t) if !t.is_empty() => t.clone(),
            _ => {
                failed.push(RefreshBuildsFailure {
                    build_id: build_id.clone(),
                    error: "no-source".to_string(),
                });
                continue;
            }
        };
        if data.builds[position].custom_edited && !options.force {
            skipped_custom.push(build_id.clone());
            continue;
        }
        let existing_name = data.builds[position].name.clone();
        send_progress(progress, &format!("Refreshing: {}", existing_name));

        let page = match api::get_page_wikitext(&title, &settings).await {
            Ok(p) => p,
            Err(err) => {
                failed.push(RefreshBuildsFailure {
                    build_id: build_id.clone(),
                    error: err.to_string(),
                });
                continue;
            }
        };
        let parsed = parser::parse_liquipedia_page(&page.page_title, &page.wikitext);
        let fresh = builds_from_page(&page.page_title, &parsed, page.revision_id);
        let chosen = match pick_chosen(&fresh, &existing_name, &page.page_title).cloned() {
            Some(c) => c,
            None => {
                failed.push(RefreshBuildsFailure {
                    build_id: build_id.clone(),
                    error: "No build template on source page.".to_string(),
                });
                continue;
            }
        };
        apply_refreshed(&mut data.builds[position], chosen);
        refreshed.push(build_id.clone());
    }

    storage::save_builds(paths, data).await?;
    Ok(RefreshBuildsResult {
        refreshed,
        failed,
        skipped_custom,
    })
}
