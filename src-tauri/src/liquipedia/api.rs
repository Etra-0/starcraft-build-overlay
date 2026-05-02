// src-tauri/src/liquipedia/api.rs
// Rate-limited MediaWiki client for liquipedia.net/starcraft. Mirrors
// src/main/liquipedia/api.ts: a settings-driven User-Agent + post-request
// delay, plus helpers for fetching page wikitext, walking categories, and
// batching revision lookups.

use crate::storage::DEFAULT_USER_AGENT;
use crate::types::Settings;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use thiserror::Error;
use tokio::time::sleep;

const API_URL: &str = "https://liquipedia.net/starcraft/api.php";

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("http {0}")]
    Http(reqwest::Error),
    #[error("Liquipedia HTTP {status}: {body}")]
    BadStatus { status: u16, body: String },
    #[error("Liquipedia API error {code}: {info}")]
    Api { code: String, info: String },
    #[error("Page not found: {0}")]
    NotFound(String),
    #[error("Enter a Liquipedia URL or page title.")]
    EmptyInput,
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<reqwest::Error> for ApiError {
    fn from(value: reqwest::Error) -> Self {
        ApiError::Http(value)
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize, Clone)]
struct LiquipediaPageRevisionSlots {
    main: Option<LiquipediaPageRevisionMain>,
}

#[derive(Debug, Deserialize, Clone)]
struct LiquipediaPageRevisionMain {
    content: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct LiquipediaPageRevision {
    revid: Option<i64>,
    timestamp: Option<String>,
    slots: Option<LiquipediaPageRevisionSlots>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct LiquipediaPage {
    pageid: Option<i64>,
    title: String,
    #[serde(default)]
    missing: bool,
    #[serde(default)]
    ns: i64,
    revisions: Option<Vec<LiquipediaPageRevision>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LiquipediaCategoryMember {
    pub pageid: i64,
    pub ns: i64,
    pub title: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LiquipediaNormalized {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LiquipediaErrorBody {
    code: String,
    info: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct LiquipediaQueryInner {
    #[serde(default)]
    pages: Vec<LiquipediaPage>,
    #[serde(default)]
    categorymembers: Vec<LiquipediaCategoryMember>,
    #[serde(default)]
    normalized: Vec<LiquipediaNormalized>,
    #[serde(default)]
    redirects: Vec<LiquipediaNormalized>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct LiquipediaContinue {
    cmcontinue: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
struct LiquipediaQueryResponse {
    error: Option<LiquipediaErrorBody>,
    #[serde(default)]
    query: Option<LiquipediaQueryInner>,
    #[serde(rename = "continue")]
    cont: Option<LiquipediaContinue>,
}

#[derive(Debug, Clone)]
pub struct PageWikitext {
    pub page_title: String,
    pub page_id: Option<i64>,
    pub revision_id: Option<i64>,
    pub revision_timestamp: Option<String>,
    pub wikitext: String,
}

#[derive(Debug, Clone)]
pub struct RevisionInfo {
    pub revision_id: Option<i64>,
    pub revision_timestamp: Option<String>,
    pub missing: bool,
}

fn user_agent_for(settings: &Settings) -> String {
    if settings.liquipedia_user_agent.trim().is_empty() {
        DEFAULT_USER_AGENT.to_string()
    } else {
        settings.liquipedia_user_agent.clone()
    }
}

fn rate_limit_for(settings: &Settings) -> u64 {
    settings.rate_limit_ms.max(2000)
}

async fn liquipedia_query(
    client: &reqwest::Client,
    params: &[(&str, &str)],
    settings: &Settings,
) -> ApiResult<LiquipediaQueryResponse> {
    let response = client
        .get(API_URL)
        .header(reqwest::header::USER_AGENT, user_agent_for(settings))
        .header(reqwest::header::ACCEPT_ENCODING, "gzip")
        .query(params)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        let cap = body.chars().take(500).collect::<String>();
        return Err(ApiError::BadStatus {
            status: status.as_u16(),
            body: cap,
        });
    }

    let bytes = response.bytes().await?;
    let parsed: LiquipediaQueryResponse = serde_json::from_slice(&bytes)?;
    if let Some(err) = &parsed.error {
        return Err(ApiError::Api {
            code: err.code.clone(),
            info: err.info.clone(),
        });
    }
    sleep(Duration::from_millis(rate_limit_for(settings))).await;
    Ok(parsed)
}

pub fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .gzip(true)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("reqwest::Client::builder should not fail with default config")
}

pub async fn get_page_wikitext(title: &str, settings: &Settings) -> ApiResult<PageWikitext> {
    let client = build_client();
    let params = [
        ("action", "query"),
        ("prop", "revisions"),
        ("titles", title),
        ("rvprop", "content|ids|timestamp"),
        ("rvslots", "main"),
        ("format", "json"),
        ("formatversion", "2"),
        ("redirects", "1"),
    ];
    let json = liquipedia_query(&client, &params, settings).await?;
    let page = json
        .query
        .and_then(|q| q.pages.into_iter().next())
        .ok_or_else(|| ApiError::NotFound(title.to_string()))?;
    if page.missing {
        return Err(ApiError::NotFound(page.title));
    }
    let rev = page
        .revisions
        .and_then(|mut v| v.drain(..).next())
        .unwrap_or(LiquipediaPageRevision {
            revid: None,
            timestamp: None,
            slots: None,
        });
    let wikitext = rev
        .slots
        .and_then(|s| s.main)
        .and_then(|m| m.content)
        .unwrap_or_default();
    Ok(PageWikitext {
        page_title: page.title,
        page_id: page.pageid,
        revision_id: rev.revid,
        revision_timestamp: rev.timestamp,
        wikitext,
    })
}

pub async fn get_category_members(
    category_title: &str,
    settings: &Settings,
) -> ApiResult<Vec<LiquipediaCategoryMember>> {
    let client = build_client();
    let mut members: Vec<LiquipediaCategoryMember> = Vec::new();
    let mut cmcontinue: Option<String> = None;
    loop {
        let cont_value = cmcontinue.clone().unwrap_or_default();
        let params = [
            ("action", "query"),
            ("list", "categorymembers"),
            ("cmtitle", category_title),
            ("cmlimit", "100"),
            ("cmcontinue", cont_value.as_str()),
            ("format", "json"),
            ("formatversion", "2"),
        ];
        let json = liquipedia_query(&client, &params, settings).await?;
        if let Some(query) = json.query {
            members.extend(query.categorymembers);
        }
        cmcontinue = json.cont.and_then(|c| c.cmcontinue);
        if cmcontinue.is_none() {
            break;
        }
    }
    Ok(members)
}

pub async fn get_revisions_for_titles(
    titles: &[String],
    settings: &Settings,
) -> ApiResult<HashMap<String, RevisionInfo>> {
    let client = build_client();
    let mut out: HashMap<String, RevisionInfo> = HashMap::new();
    for chunk in titles.chunks(50) {
        let joined = chunk.join("|");
        let params = [
            ("action", "query"),
            ("prop", "revisions"),
            ("titles", joined.as_str()),
            ("rvprop", "ids|timestamp"),
            ("format", "json"),
            ("formatversion", "2"),
            ("redirects", "1"),
        ];
        let json = liquipedia_query(&client, &params, settings).await?;
        let query = match json.query {
            Some(q) => q,
            None => continue,
        };
        for page in &query.pages {
            let rev = page
                .revisions
                .as_ref()
                .and_then(|v| v.first())
                .cloned()
                .unwrap_or(LiquipediaPageRevision {
                    revid: None,
                    timestamp: None,
                    slots: None,
                });
            if rev.revid.is_none() && rev.timestamp.is_none() {
                continue;
            }
            out.insert(
                page.title.clone(),
                RevisionInfo {
                    revision_id: rev.revid,
                    revision_timestamp: rev.timestamp,
                    missing: page.missing,
                },
            );
        }
        for n in &query.normalized {
            if let Some(rev) = out.get(&n.to).cloned() {
                out.insert(n.from.clone(), rev);
            }
        }
        for r in &query.redirects {
            if let Some(rev) = out.get(&r.to).cloned() {
                out.insert(r.from.clone(), rev);
            }
        }
    }
    Ok(out)
}

pub fn page_url(title: &str) -> String {
    let encoded = urlencoding::encode(&title.replace(' ', "_")).into_owned();
    format!("https://liquipedia.net/starcraft/{}", encoded)
}

pub fn parse_liquipedia_title(input: &str) -> ApiResult<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ApiError::EmptyInput);
    }
    if let Ok(parsed) = url::Url::parse(trimmed) {
        let segments: Vec<&str> = parsed
            .path_segments()
            .map(|s| s.filter(|seg| !seg.is_empty()).collect())
            .unwrap_or_default();
        if segments.len() < 2 || !segments[0].eq_ignore_ascii_case("starcraft") {
            return Err(ApiError::InvalidUrl(
                "Expected a liquipedia.net/starcraft URL.".to_string(),
            ));
        }
        let joined = segments[1..].join("/");
        let decoded = urlencoding::decode(&joined)
            .map(|c| c.into_owned())
            .unwrap_or(joined);
        return Ok(decoded.replace('_', " "));
    }
    let mut s = trimmed.to_string();
    let lc = s.to_lowercase();
    if let Some(idx) = lc.find("liquipedia.net/starcraft/") {
        let cut = idx + "liquipedia.net/starcraft/".len();
        s = s[cut..].to_string();
    } else if let Some(idx) = lc.find("https://") {
        s = s[idx..].to_string();
    } else if let Some(idx) = lc.find("http://") {
        s = s[idx..].to_string();
    }
    Ok(s.replace('_', " "))
}

mod urlencoding {
    pub fn encode(input: &str) -> std::borrow::Cow<'_, str> {
        let needs_encoding = input.bytes().any(|b| !is_unreserved(b));
        if !needs_encoding {
            return std::borrow::Cow::Borrowed(input);
        }
        let mut out = String::with_capacity(input.len() * 3);
        for b in input.bytes() {
            if is_unreserved(b) {
                out.push(b as char);
            } else {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
        std::borrow::Cow::Owned(out)
    }

    pub fn decode(input: &str) -> Result<std::borrow::Cow<'_, str>, std::str::Utf8Error> {
        if !input.contains('%') {
            return Ok(std::borrow::Cow::Borrowed(input));
        }
        let bytes = input.as_bytes();
        let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                let hi = hex_digit(bytes[i + 1]);
                let lo = hex_digit(bytes[i + 2]);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi << 4) | lo);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        match String::from_utf8(out) {
            Ok(s) => Ok(std::borrow::Cow::Owned(s)),
            Err(e) => Err(e.utf8_error()),
        }
    }

    fn is_unreserved(b: u8) -> bool {
        b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/')
    }

    fn hex_digit(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_url_encodes_spaces_as_underscores() {
        assert_eq!(
            page_url("1 Gate Core (vs. Terran)"),
            "https://liquipedia.net/starcraft/1_Gate_Core_%28vs._Terran%29"
        );
    }

    #[test]
    fn parse_liquipedia_title_accepts_url_or_title() {
        assert_eq!(
            parse_liquipedia_title("https://liquipedia.net/starcraft/1_Gate_Core_(vs._Terran)")
                .unwrap(),
            "1 Gate Core (vs. Terran)"
        );
        assert_eq!(
            parse_liquipedia_title("1 Gate Core (vs. Terran)").unwrap(),
            "1 Gate Core (vs. Terran)"
        );
        assert_eq!(
            parse_liquipedia_title("liquipedia.net/starcraft/Some_Build").unwrap(),
            "Some Build"
        );
    }

    #[test]
    fn parse_liquipedia_title_rejects_empty() {
        assert!(matches!(
            parse_liquipedia_title("   "),
            Err(ApiError::EmptyInput)
        ));
    }
}
