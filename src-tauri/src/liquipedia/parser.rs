// src-tauri/src/liquipedia/parser.rs
// Pure wikitext parser ported from src/main/liquipedia/parser.ts. Locates
// {{build}} and {{Infobox strategy}} templates with brace-aware scanning,
// extracts step bullets and infobox fields, detects player race / opponent
// / difficulty, and pulls the "Counter To" / "Countered By" sections.

use crate::types::{
    Difficulty, Opponent, ParsedInfobox, ParsedLiquipediaPage, ParsedVariant, Race,
};
use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

static REGEX_HTML_COMMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static REGEX_REF_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?is)<ref[^>]*>.*?</ref>").unwrap());
static REGEX_BR_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>").unwrap());
static REGEX_HTML_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
static REGEX_RACE_P: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[Pp]\}\}").unwrap());
static REGEX_RACE_T: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[Tt]\}\}").unwrap());
static REGEX_RACE_Z: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[Zz]\}\}").unwrap());
static REGEX_TEMPLATE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{\{(?:[^{}]|\{[^{}]*\})*\}\}").unwrap());
static REGEX_FILE_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[\[(?:File|Image):[^\]]+\]\]").unwrap());
static REGEX_PIPED_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^|\]]+)\|([^\]]+)\]\]").unwrap());
static REGEX_PLAIN_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]\]").unwrap());
static REGEX_EXTERNAL_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[https?://[^\s\]]+\s*([^\]]*)\]").unwrap());
static REGEX_TRIPLE_QUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"'''").unwrap());
static REGEX_DOUBLE_QUOTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"''").unwrap());
static REGEX_WHITESPACE_RUN: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());
static REGEX_NAMED_PARAM_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_-]*\s*=").unwrap());
static REGEX_BULLET_PREFIX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\*+\s*").unwrap());
static REGEX_LEADING_BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*\*+").unwrap());
static REGEX_LEADING_BULLET_STRIP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*\*+\s*").unwrap());
static REGEX_SUPPLY_PREFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{1,3}(?:\s*/\s*\d{1,3})?\s*[-\u{2013}\u{2014}:.\)]").unwrap());
static REGEX_BEGINNER_CAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[\[Category:[^\]]*Beginner\s*Strategy\]\]").unwrap());
static REGEX_ADVANCED_CAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[\[Category:[^\]]*Advanced\s*Strategy\]\]").unwrap());
static REGEX_INTERMEDIATE_CAT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\[\[Category:[^\]]*Intermediate\s*Strategy\]\]").unwrap());
static REGEX_PVT_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b([PTZ])v([PTZR])\b").unwrap());
static REGEX_VS_TERRAN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:vs\.?|versus)\s*Terran\b").unwrap());
static REGEX_VS_ZERG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:vs\.?|versus)\s*Zerg\b").unwrap());
static REGEX_VS_PROTOSS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:vs\.?|versus)\s*Protoss\b").unwrap());
static REGEX_VS_RANDOM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:vs\.?|versus)\s*Random\b").unwrap());
// Original TS regex was \bv?T\b(?!ower); the lookahead is redundant because
// \bT\b already requires a non-word char after T, so "Tower" can never match.
// Rust's regex crate doesn't support lookaround, so we drop the assertion.
static REGEX_VT_TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bv?T\b").unwrap());
static REGEX_PVT_HAYSTACK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[PTZ]vT\b").unwrap());
static REGEX_PVZ_HAYSTACK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[PTZ]vZ\b").unwrap());
static REGEX_PVP_HAYSTACK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[PTZ]vP\b").unwrap());
static REGEX_PVR_HAYSTACK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[PTZ]vR\b").unwrap());
static REGEX_MATCHUP_FIRST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^([PTZ])v[PTZR]").unwrap());
static REGEX_MATCHUP_SECOND: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^[PTZ]v([PTZR])").unwrap());
static REGEX_INFOBOX_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^Infobox\s*strategy$").unwrap());
static REGEX_BUILD_TEMPLATE_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^build$").unwrap());

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Template {
    name: String,
    body: String,
    start: usize,
    end: usize,
}

#[derive(Debug, Default)]
struct TemplateParams {
    named: std::collections::BTreeMap<String, String>,
    positional: Vec<String>,
}

pub fn strip_wiki_markup(line: &str) -> String {
    let mut s = line.to_string();
    s = REGEX_HTML_COMMENT.replace_all(&s, "").into_owned();
    s = REGEX_REF_TAG.replace_all(&s, "").into_owned();
    s = REGEX_BR_TAG.replace_all(&s, " ").into_owned();
    s = REGEX_HTML_TAG.replace_all(&s, "").into_owned();
    s = REGEX_RACE_P.replace_all(&s, "Protoss").into_owned();
    s = REGEX_RACE_T.replace_all(&s, "Terran").into_owned();
    s = REGEX_RACE_Z.replace_all(&s, "Zerg").into_owned();
    s = REGEX_TEMPLATE.replace_all(&s, "").into_owned();
    s = REGEX_FILE_LINK.replace_all(&s, "").into_owned();
    s = REGEX_PIPED_LINK.replace_all(&s, "$2").into_owned();
    s = REGEX_PLAIN_LINK.replace_all(&s, "$1").into_owned();
    s = REGEX_EXTERNAL_LINK.replace_all(&s, "$1").into_owned();
    s = REGEX_TRIPLE_QUOTE.replace_all(&s, "").into_owned();
    s = REGEX_DOUBLE_QUOTE.replace_all(&s, "").into_owned();
    s = s.replace("&nbsp;", " ");
    s = s.replace("&ndash;", "\u{2013}");
    s = s.replace("&mdash;", "\u{2014}");
    s = REGEX_WHITESPACE_RUN.replace_all(&s, " ").into_owned();
    s.trim().to_string()
}

fn find_templates(wikitext: &str, name_re: &Regex) -> Vec<Template> {
    let bytes = wikitext.as_bytes();
    let len = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 1 < len {
        if bytes[i] != b'{' || bytes[i + 1] != b'{' {
            i += 1;
            continue;
        }
        let mut depth = 1i32;
        let mut j = i + 2;
        while j < len && depth > 0 {
            let b = bytes[j];
            let next = if j + 1 < len { bytes[j + 1] } else { 0 };
            if b == b'{' && next == b'{' {
                depth += 1;
                j += 2;
                continue;
            }
            if b == b'}' && next == b'}' {
                depth -= 1;
                j += 2;
                continue;
            }
            j += 1;
        }
        if depth != 0 {
            break;
        }
        let inner_bytes = &bytes[i + 2..j - 2];
        let inner = match std::str::from_utf8(inner_bytes) {
            Ok(s) => s,
            Err(_) => {
                i = j;
                continue;
            }
        };
        let pipe_idx = inner.find('|');
        let (name, body) = match pipe_idx {
            None => (inner, ""),
            Some(idx) => (&inner[..idx], &inner[idx + 1..]),
        };
        let trimmed_name = name.trim();
        if name_re.is_match(trimmed_name) {
            out.push(Template {
                name: trimmed_name.to_string(),
                body: body.to_string(),
                start: i,
                end: j,
            });
        }
        i = j;
    }
    out
}

fn split_template_body(body: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut depth_brace = 0i32;
    let mut depth_bracket = 0i32;
    let mut buf = String::new();
    let mut i = 0usize;
    while i < len {
        let c = bytes[i];
        let n = if i + 1 < len { bytes[i + 1] } else { 0 };
        if c == b'{' && n == b'{' {
            depth_brace += 1;
            buf.push('{');
            i += 1;
            continue;
        }
        if c == b'}' && n == b'}' {
            depth_brace = (depth_brace - 1).max(0);
            buf.push('}');
            i += 1;
            continue;
        }
        if c == b'[' && n == b'[' {
            depth_bracket += 1;
            buf.push('[');
            i += 1;
            continue;
        }
        if c == b']' && n == b']' {
            depth_bracket = (depth_bracket - 1).max(0);
            buf.push(']');
            i += 1;
            continue;
        }
        if c == b'|' && depth_brace == 0 && depth_bracket == 0 {
            parts.push(std::mem::take(&mut buf));
            i += 1;
            continue;
        }
        let ch_end = next_utf8_boundary(bytes, i);
        buf.push_str(std::str::from_utf8(&bytes[i..ch_end]).unwrap_or(""));
        i = ch_end;
    }
    if !buf.is_empty() || !parts.is_empty() {
        parts.push(buf);
    }
    parts
}

fn next_utf8_boundary(bytes: &[u8], start: usize) -> usize {
    let mut end = start + 1;
    while end < bytes.len() && (bytes[end] & 0b1100_0000) == 0b1000_0000 {
        end += 1;
    }
    end
}

fn parse_template_params(body: &str) -> TemplateParams {
    let mut params = TemplateParams::default();
    for part in split_template_body(body) {
        match part.find('=') {
            None => params.positional.push(part.trim().to_string()),
            Some(idx) => {
                let key = part[..idx].trim().to_lowercase();
                let val = part[idx + 1..].to_string();
                params.named.insert(key, val);
            }
        }
    }
    params
}

pub fn parse_build_body(body: &str) -> Vec<String> {
    let mut steps = Vec::new();
    for part in split_template_body(body) {
        let trimmed = part.trim();
        if trimmed.contains('=') && REGEX_NAMED_PARAM_KEY.is_match(&trimmed.to_lowercase()) {
            continue;
        }
        for raw in part.split('\n') {
            let raw = raw.trim_end_matches('\r');
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if !REGEX_BULLET_PREFIX.is_match(line) {
                continue;
            }
            let stripped = REGEX_BULLET_PREFIX.replace(line, "");
            let cleaned = strip_wiki_markup(&stripped);
            if !cleaned.is_empty() {
                steps.push(cleaned);
            }
        }
    }
    steps
}

/// Returns (level, inner_text) when `line` is a wiki heading like `== Foo ==`.
/// Replaces the JS regex `^(={2,5})\s*(.*?)\s*\1\s*$` (which uses a
/// backreference unsupported by the Rust `regex` crate).
fn parse_heading(line: &str) -> Option<(usize, String)> {
    let trimmed = line.trim();
    let opening = trimmed.bytes().take_while(|&b| b == b'=').count();
    if !(2..=5).contains(&opening) {
        return None;
    }
    let trailing = trimmed.bytes().rev().take_while(|&b| b == b'=').count();
    if trailing != opening {
        return None;
    }
    if trimmed.len() < opening + trailing {
        return None;
    }
    let inner = &trimmed[opening..trimmed.len() - trailing];
    Some((opening, inner.trim().to_string()))
}

fn find_preceding_heading(wikitext: &str, position: usize) -> Option<String> {
    let slice = &wikitext[..position.min(wikitext.len())];
    let mut last: Option<String> = None;
    for line in slice.lines() {
        if let Some((_, inner)) = parse_heading(line) {
            if !inner.is_empty() {
                last = Some(strip_wiki_markup(&inner));
            }
        }
    }
    last
}

pub fn parse_infobox(wikitext: &str) -> Option<ParsedInfobox> {
    let found = find_templates(wikitext, &REGEX_INFOBOX_NAME);
    let template = found.first()?;
    let params = parse_template_params(&template.body);
    let mut result = ParsedInfobox::default();

    if let Some(name) = params.named.get("name") {
        result.name = Some(strip_wiki_markup(name).trim().to_string());
    }
    if let Some(race) = params.named.get("race") {
        if let Some(crate::types::RaceOrRandom::Race(r)) = normalize_race(race) {
            result.race = Some(r);
        }
    }
    if let Some(matchups) = params.named.get("matchups") {
        let cleaned = strip_wiki_markup(matchups);
        let parts: Vec<String> = cleaned
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        result.matchups = Some(parts);
    }
    if let Some(creator) = params.named.get("creator") {
        result.creator = Some(strip_wiki_markup(creator).trim().to_string());
    }
    if let Some(popularized) = params.named.get("popularized") {
        result.popularized = Some(strip_wiki_markup(popularized).trim().to_string());
    }

    Some(result)
}

pub fn normalize_race(value: &str) -> Option<crate::types::RaceOrRandom> {
    let s = value.trim();
    if s.is_empty() {
        return None;
    }
    let upper = s.to_uppercase();
    match upper.as_str() {
        "P" => return Some(crate::types::RaceOrRandom::Race(Race::Protoss)),
        "T" => return Some(crate::types::RaceOrRandom::Race(Race::Terran)),
        "Z" => return Some(crate::types::RaceOrRandom::Race(Race::Zerg)),
        "R" => return Some(crate::types::RaceOrRandom::Random),
        _ => {}
    }
    let lower = s.to_lowercase();
    if lower.starts_with('p') {
        Some(crate::types::RaceOrRandom::Race(Race::Protoss))
    } else if lower.starts_with('t') {
        Some(crate::types::RaceOrRandom::Race(Race::Terran))
    } else if lower.starts_with('z') {
        Some(crate::types::RaceOrRandom::Race(Race::Zerg))
    } else if lower.starts_with('r') {
        Some(crate::types::RaceOrRandom::Random)
    } else {
        None
    }
}

pub fn detect_player_race(
    title: &str,
    infobox: Option<&ParsedInfobox>,
    wikitext: &str,
) -> Option<Race> {
    if let Some(ib) = infobox {
        if let Some(r) = ib.race {
            return Some(r);
        }
    }
    let head = if wikitext.len() > 2000 {
        let mut idx = 2000;
        while !wikitext.is_char_boundary(idx) {
            idx -= 1;
        }
        &wikitext[..idx]
    } else {
        wikitext
    };
    let haystack = format!("{}\n{}", title, head);
    if let Some(caps) = REGEX_PVT_PATTERN.captures(&haystack) {
        if let Some(letter) = caps.get(1) {
            if let Some(crate::types::RaceOrRandom::Race(r)) = normalize_race(letter.as_str()) {
                return Some(r);
            }
        }
    }
    if let Some(ib) = infobox {
        if let Some(matchups) = &ib.matchups {
            if let Some(first) = matchups.first() {
                if let Some(caps) = REGEX_MATCHUP_FIRST.captures(first) {
                    if let Some(letter) = caps.get(1) {
                        if let Some(crate::types::RaceOrRandom::Race(r)) =
                            normalize_race(letter.as_str())
                        {
                            return Some(r);
                        }
                    }
                }
            }
        }
    }
    None
}

pub fn detect_opponent(
    title: &str,
    infobox: Option<&ParsedInfobox>,
    wikitext: &str,
) -> Option<Opponent> {
    let head = if wikitext.len() > 3000 {
        let mut idx = 3000;
        while !wikitext.is_char_boundary(idx) {
            idx -= 1;
        }
        &wikitext[..idx]
    } else {
        wikitext
    };
    let haystack = format!("{}\n{}", title, head);

    if REGEX_VS_TERRAN.is_match(&haystack)
        || REGEX_VT_TITLE.is_match(title)
        || REGEX_PVT_HAYSTACK.is_match(&haystack)
    {
        return Some(Opponent::Terran);
    }
    if REGEX_VS_ZERG.is_match(&haystack) || REGEX_PVZ_HAYSTACK.is_match(&haystack) {
        return Some(Opponent::Zerg);
    }
    if REGEX_VS_PROTOSS.is_match(&haystack) || REGEX_PVP_HAYSTACK.is_match(&haystack) {
        return Some(Opponent::Protoss);
    }
    if REGEX_VS_RANDOM.is_match(&haystack) || REGEX_PVR_HAYSTACK.is_match(&haystack) {
        return Some(Opponent::Random);
    }
    if let Some(ib) = infobox {
        if let Some(matchups) = &ib.matchups {
            if let Some(first) = matchups.first() {
                if let Some(caps) = REGEX_MATCHUP_SECOND.captures(first) {
                    if let Some(letter) = caps.get(1) {
                        return match letter.as_str().to_uppercase().as_str() {
                            "P" => Some(Opponent::Protoss),
                            "T" => Some(Opponent::Terran),
                            "Z" => Some(Opponent::Zerg),
                            "R" => Some(Opponent::Random),
                            _ => None,
                        };
                    }
                }
            }
        }
    }
    None
}

pub fn detect_difficulty(wikitext: &str) -> Option<Difficulty> {
    if REGEX_BEGINNER_CAT.is_match(wikitext) {
        return Some(Difficulty::Beginner);
    }
    if REGEX_ADVANCED_CAT.is_match(wikitext) {
        return Some(Difficulty::Advanced);
    }
    if REGEX_INTERMEDIATE_CAT.is_match(wikitext) {
        return Some(Difficulty::Intermediate);
    }
    None
}

fn extract_list_section(wikitext: &str, heading_re: &Regex) -> Vec<String> {
    let mut items = Vec::new();
    let mut in_section = false;
    for line in wikitext.lines() {
        if let Some((_, inner)) = parse_heading(line) {
            in_section = heading_re.is_match(&inner);
            continue;
        }
        if !in_section {
            continue;
        }
        if !REGEX_LEADING_BULLET.is_match(line) {
            continue;
        }
        let stripped = REGEX_LEADING_BULLET_STRIP.replace(line, "");
        let cleaned = strip_wiki_markup(&stripped);
        if !cleaned.is_empty() {
            items.push(cleaned);
        }
    }
    items
}

fn extract_fallback_steps(wikitext: &str) -> Vec<String> {
    let mut steps = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for line in wikitext.lines() {
        if parse_heading(line).is_some() {
            continue;
        }
        if !REGEX_LEADING_BULLET.is_match(line) {
            continue;
        }
        let stripped = REGEX_LEADING_BULLET_STRIP.replace(line, "");
        let cleaned = strip_wiki_markup(&stripped);
        if cleaned.len() < 3 || cleaned.len() > 220 {
            continue;
        }
        if !REGEX_SUPPLY_PREFIX.is_match(&cleaned) {
            continue;
        }
        let key = cleaned.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        steps.push(cleaned);
        if steps.len() >= 60 {
            break;
        }
    }
    steps
}

pub fn parse_liquipedia_page(page_title: &str, wikitext: &str) -> ParsedLiquipediaPage {
    let infobox = parse_infobox(wikitext);
    let player_race = detect_player_race(page_title, infobox.as_ref(), wikitext);
    let opponent = detect_opponent(page_title, infobox.as_ref(), wikitext);
    let difficulty = detect_difficulty(wikitext);
    let counters_re = Regex::new(r"(?i)Counter\s*To").unwrap();
    let countered_by_re = Regex::new(r"(?i)Counter(?:ed)?\s*By").unwrap();
    let counters = extract_list_section(wikitext, &counters_re);
    let countered_by = extract_list_section(wikitext, &countered_by_re);

    let templates = find_templates(wikitext, &REGEX_BUILD_TEMPLATE_NAME);
    let mut variants: Vec<ParsedVariant> = Vec::new();
    for tmpl in templates {
        let params = parse_template_params(&tmpl.body);
        let steps = parse_build_body(&tmpl.body);
        if steps.is_empty() {
            continue;
        }
        let heading = find_preceding_heading(wikitext, tmpl.start);
        let raw_name = params
            .named
            .get("name")
            .map(|v| strip_wiki_markup(v))
            .unwrap_or_default();
        let mut variant_name = raw_name.trim_matches('"').to_string();
        if variant_name.is_empty() {
            if let Some(h) = &heading {
                variant_name = h.clone();
            }
        }
        variants.push(ParsedVariant {
            variant_name,
            heading,
            steps,
        });
    }

    if variants.is_empty() {
        let fallback = extract_fallback_steps(wikitext);
        if !fallback.is_empty() {
            variants.push(ParsedVariant {
                variant_name: String::new(),
                heading: None,
                steps: fallback,
            });
        }
    }

    ParsedLiquipediaPage {
        infobox,
        player_race,
        opponent,
        difficulty,
        counters,
        countered_by,
        variants,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_build_template_variants_into_distinct_builds() {
        let wikitext = [
            "{{Infobox strategy",
            "|name=1 Gate Cybernetics Core",
            "|race=P",
            "|matchups=PvT",
            "|creator=Unknown",
            "|popularized=Unknown",
            "}}",
            "",
            "==Build Order==",
            "===No Zealot before Cybernetics Core===",
            "{{build|name=\"One Gate Cybernetics Core\"|width=400px|race=Protoss|",
            "*8 - Pylon",
            "*10 - Gateway",
            "*12 - Assimilator",
            "*13 - Cybernetics Core",
            "}}",
            "",
            "===One Zealot before Cybernetics Core===",
            "{{build|name=\"One Gate Cybernetics Core\"|width=400px|race=Protoss|",
            "*8 - Pylon",
            "*10 - Gate",
            "*12 - Assimilator",
            "*13 - Zealot",
            "*16 - Pylon",
            "*18 - Cybernetics Core",
            "}}",
        ]
        .join("\n");

        let parsed = parse_liquipedia_page("1 Gate Core (vs. Terran)", &wikitext);
        assert_eq!(
            parsed.variants.len(),
            2,
            "expected two variants, got {}",
            parsed.variants.len()
        );
        assert_ne!(parsed.variants[0].steps, parsed.variants[1].steps);
        assert!(!parsed.variants[0].steps.is_empty());
        assert!(parsed.variants[1].steps.len() > parsed.variants[0].steps.len());
    }

    #[test]
    fn fallback_extracts_supply_steps_from_bullet_only_pages() {
        let wikitext = "{{Infobox strategy\n|name=14 CC\n|race=T\n|matchups=TvZ\n}}\n==Build Order==\n*9/10 - Supply Depot\n*14/18 - Command Center\n*15/28 - Barracks\n*17/36 - Barracks\n*18/36 - Refinery\n*22/36 - Academy\n*40/52 - Barracks";
        let parsed = parse_liquipedia_page("14 CC (vs. Zerg)", wikitext);
        assert!(!parsed.variants.is_empty());
        assert!(parsed.variants[0].steps.len() >= 6);
    }

    #[test]
    fn strip_wiki_markup_handles_links_and_templates() {
        assert_eq!(strip_wiki_markup("[[Foo|Bar]]"), "Bar");
        assert_eq!(strip_wiki_markup("[[Plain]]"), "Plain");
        assert_eq!(
            strip_wiki_markup("'''bold''' and ''italic''"),
            "bold and italic"
        );
        assert_eq!(strip_wiki_markup("hello {{T}} world"), "hello Terran world");
    }

    #[test]
    fn parse_heading_detects_balanced_levels() {
        assert_eq!(parse_heading("== Foo =="), Some((2, "Foo".to_string())));
        assert_eq!(parse_heading("=== Bar ==="), Some((3, "Bar".to_string())));
        assert_eq!(parse_heading("== Mismatch ==="), None);
        assert_eq!(parse_heading("not a heading"), None);
    }

    #[test]
    fn parse_infobox_extracts_name_race_matchups_and_credits() {
        let wikitext = [
            "{{Infobox strategy",
            "|name='''2 Hatch Lurker'''",
            "|race=Z",
            "|matchups=ZvT, ZvP",
            "|creator=[[Some Player|SomePlayer]]",
            "|popularized=AnotherPlayer",
            "}}",
            "",
            "==Build Order==",
        ]
        .join("\n");

        let parsed = parse_infobox(&wikitext).expect("infobox should parse");
        assert_eq!(parsed.name.as_deref(), Some("2 Hatch Lurker"));
        assert_eq!(parsed.race, Some(Race::Zerg));
        assert_eq!(
            parsed.matchups.as_deref(),
            Some(&["ZvT".to_string(), "ZvP".to_string()][..])
        );
        assert_eq!(parsed.creator.as_deref(), Some("SomePlayer"));
        assert_eq!(parsed.popularized.as_deref(), Some("AnotherPlayer"));
    }
}
