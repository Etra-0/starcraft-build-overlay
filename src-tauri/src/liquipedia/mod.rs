// src-tauri/src/liquipedia/mod.rs
// Re-exports for the Liquipedia subsystem: rate-limited MediaWiki client,
// wikitext parser for {{build}} / {{Infobox strategy}} templates, and the
// import / update orchestration that talks to storage.

pub mod api;
pub mod import;
pub mod parser;
pub mod updates;
