// src-tauri/src/utils.rs
// Pure helpers shared between storage, the Liquipedia parser, and the
// importer: race-initial constants, matchup derivation, kebab-case slugging,
// and "X - Y" build-step splitting. Mirrors src/shared/utils.ts.

use crate::types::{Opponent, Race};

pub fn race_initial(race: &str) -> char {
    match race {
        "Terran" => 'T',
        "Protoss" => 'P',
        "Zerg" => 'Z',
        "Random" => 'R',
        _ => '?',
    }
}

pub fn derive_matchup(race: &str, opponent: &str) -> String {
    let r = race_initial(race);
    let o = race_initial(opponent);
    format!("{}v{}", r, o)
}

pub fn derive_matchup_typed(race: Race, opponent: Opponent) -> String {
    format!("{}v{}", race.initial(), opponent.initial())
}

pub fn slugify(value: &str) -> String {
    let lowered = value.to_lowercase();
    let no_amp = lowered.replace('&', "and");
    let mut buf = String::with_capacity(no_amp.len());
    let mut depth = 0i32;
    for ch in no_amp.chars() {
        if ch == '(' {
            depth += 1;
            continue;
        }
        if ch == ')' {
            if depth > 0 {
                depth -= 1;
            }
            continue;
        }
        if depth == 0 {
            buf.push(ch);
        }
    }

    let mut out = String::with_capacity(buf.len());
    let mut last_was_dash = true;
    for ch in buf.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }

    while out.ends_with('-') {
        out.pop();
    }
    while out.starts_with('-') {
        out.remove(0);
    }

    if out.len() > 80 {
        out.truncate(80);
        while out.ends_with('-') {
            out.pop();
        }
    }

    if out.is_empty() {
        "build".to_string()
    } else {
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitStep {
    pub supply: Option<String>,
    pub action: String,
}

pub fn split_step(step: &str) -> SplitStep {
    let s = step.trim();
    if s.is_empty() {
        return SplitStep {
            supply: None,
            action: String::new(),
        };
    }

    let mut chars = s.char_indices().peekable();
    let supply_start = chars.peek().map(|(i, _)| *i);
    let mut supply_end: Option<usize> = None;
    let mut digit_run = 0usize;
    let mut saw_slash_block = false;

    while let Some(&(i, ch)) = chars.peek() {
        if ch.is_ascii_digit() {
            digit_run += 1;
            if digit_run > 3 {
                supply_end = None;
                break;
            }
            chars.next();
            supply_end = Some(i + ch.len_utf8());
        } else {
            break;
        }
    }
    if digit_run == 0 {
        return SplitStep {
            supply: None,
            action: s.to_string(),
        };
    }

    while let Some(&(_, ch)) = chars.peek() {
        if ch == ' ' {
            chars.next();
        } else {
            break;
        }
    }
    if let Some(&(i, ch)) = chars.peek() {
        if ch == '/' {
            chars.next();
            supply_end = Some(i + ch.len_utf8());
            saw_slash_block = true;
            while let Some(&(_, ch)) = chars.peek() {
                if ch == ' ' {
                    chars.next();
                } else {
                    break;
                }
            }
            digit_run = 0;
            while let Some(&(i2, ch2)) = chars.peek() {
                if ch2.is_ascii_digit() {
                    digit_run += 1;
                    if digit_run > 3 {
                        return SplitStep {
                            supply: None,
                            action: s.to_string(),
                        };
                    }
                    chars.next();
                    supply_end = Some(i2 + ch2.len_utf8());
                } else {
                    break;
                }
            }
            if digit_run == 0 {
                return SplitStep {
                    supply: None,
                    action: s.to_string(),
                };
            }
        }
    }

    while let Some(&(_, ch)) = chars.peek() {
        if ch == ' ' {
            chars.next();
        } else {
            break;
        }
    }

    let separator_idx;
    match chars.peek() {
        Some(&(i, ch)) if matches!(ch, '-' | '\u{2013}' | '\u{2014}' | ':' | '.' | ')') => {
            separator_idx = i + ch.len_utf8();
            chars.next();
        }
        _ => {
            return SplitStep {
                supply: None,
                action: s.to_string(),
            };
        }
    }

    let action = s[separator_idx..].trim().to_string();
    let supply_text = match (supply_start, supply_end) {
        (Some(start), Some(end)) => s[start..end].to_string(),
        _ => {
            return SplitStep {
                supply: None,
                action: s.to_string(),
            }
        }
    };
    let supply = supply_text.split_whitespace().collect::<String>();
    let _ = saw_slash_block;

    SplitStep {
        supply: Some(supply),
        action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_matchup_composes_initials_including_random() {
        assert_eq!(derive_matchup("Protoss", "Terran"), "PvT");
        assert_eq!(derive_matchup("Zerg", "Random"), "ZvR");
        assert_eq!(derive_matchup("Terran", "Terran"), "TvT");
        assert_eq!(derive_matchup("Bogus", "Terran"), "?vT");
    }

    #[test]
    fn slugify_produces_stable_kebab_case_ids() {
        assert_eq!(
            slugify("1 Gate Cybernetics Core (vs. Terran)"),
            "1-gate-cybernetics-core"
        );
        assert_eq!(slugify("   double  spaces   "), "double-spaces");
        assert_eq!(
            slugify("---leading-and-trailing---"),
            "leading-and-trailing"
        );
        assert_eq!(slugify(""), "build");
    }

    #[test]
    fn split_step_peels_supply_prefixes() {
        assert_eq!(
            split_step("8 - Pylon"),
            SplitStep {
                supply: Some("8".to_string()),
                action: "Pylon".to_string()
            }
        );
        assert_eq!(
            split_step("12/18 - Cybernetics Core"),
            SplitStep {
                supply: Some("12/18".to_string()),
                action: "Cybernetics Core".to_string()
            }
        );
        assert_eq!(
            split_step("Scout for proxy"),
            SplitStep {
                supply: None,
                action: "Scout for proxy".to_string()
            }
        );
        assert_eq!(
            split_step(""),
            SplitStep {
                supply: None,
                action: String::new()
            }
        );
    }
}
