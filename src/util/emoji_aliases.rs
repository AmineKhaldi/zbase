//! Shared stock-emoji resolution. Keybase aliases take precedence over library
//! names; conversation-specific custom emoji remain the renderer's first choice.
use std::{collections::HashMap, sync::OnceLock};

pub fn standard_emoji_for_alias(alias: &str) -> Option<&'static str> {
    static LOOKUP: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    let key = normalize_emoji_alias(alias);
    if key.is_empty() {
        return None;
    }
    LOOKUP
        .get_or_init(|| {
            let mut lookup = HashMap::new();
            for emoji in emojis::iter() {
                lookup
                    .entry(normalize_emoji_alias(emoji.name()))
                    .or_insert(emoji.as_str());
                for shortcode in emoji.shortcodes() {
                    lookup
                        .entry(normalize_emoji_alias(shortcode))
                        .or_insert(emoji.as_str());
                }
            }
            let keybase: HashMap<&'static str, &'static str> =
                serde_json::from_str(include_str!("../../assets/emoji/keybase-shortcodes.json"))
                    .expect("valid embedded Keybase emoji aliases");
            for (alias, unicode) in keybase {
                lookup.insert(normalize_emoji_alias(alias), unicode);
            }
            lookup
        })
        .get(&key)
        .copied()
}

pub fn normalize_emoji_alias(alias: &str) -> String {
    let mut out = String::with_capacity(alias.len());
    let mut previous_underscore = false;
    for ch in alias.trim().trim_matches(':').chars() {
        let normalized = if ch.is_ascii_alphanumeric() {
            previous_underscore = false;
            Some(ch.to_ascii_lowercase())
        } else if ch == '+' {
            previous_underscore = false;
            Some('+')
        } else if matches!(ch, '_' | '-' | ' ') {
            if previous_underscore {
                None
            } else {
                previous_underscore = true;
                Some('_')
            }
        } else if previous_underscore {
            None
        } else {
            previous_underscore = true;
            Some('_')
        };
        if let Some(value) = normalized {
            out.push(value);
        }
    }
    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_keybase_standard_alias_matches_the_embedded_table() {
        let aliases: HashMap<&str, &str> =
            serde_json::from_str(include_str!("../../assets/emoji/keybase-shortcodes.json"))
                .unwrap();
        for (alias, expected) in aliases {
            assert_eq!(standard_emoji_for_alias(alias), Some(expected), "{alias}");
        }
    }

    #[test]
    fn keybase_aliases_and_library_names_share_one_unicode_resolver() {
        for (alias, expected) in [
            (":large_green_circle:", "🟢"),
            ("green_circle", "🟢"),
            ("large_red_square", "🟥"),
            ("large_orange_circle", "🟠"),
            ("rolling_on_the_floor_laughing", "🤣"),
            ("+1", "👍"),
            ("-1", "👎"),
        ] {
            assert_eq!(standard_emoji_for_alias(alias), Some(expected), "{alias}");
        }
        assert_eq!(
            standard_emoji_for_alias("team_specific_unknown_emoji"),
            None
        );
    }
}
