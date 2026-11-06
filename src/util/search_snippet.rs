//! Turns a raw search-index snippet (message source text) into the text a
//! result row shows: one line, `:shortcode:` emoji rendered, Markdown markers
//! removed, `@user` / `#channel` mentions marked so they can be styled like
//! the timeline. Highlight ranges are byte ranges into the raw snippet and are
//! carried over to the cleaned text.
//!
//! GPUI's wrapper may break before any punctuation that isn't a word
//! character, so a path such as `assets/release/banner.png` could wrap with
//! `/banner.png` leading the next line. A hair space after each `/` inside a
//! word gives the wrapper a later break: after the slash. (A zero-width space
//! would not work: GPUI wraps on shaped glyphs and it shapes to none.)

use crate::util::emoji_aliases::standard_emoji_for_alias;

/// Cleaned snippet text, the match ranges (byte offsets) inside it, and the
/// mentions it contains, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CleanSnippet {
    pub text: String,
    pub ranges: Vec<(usize, usize)>,
    pub mentions: Vec<SnippetMention>,
}

/// An `@user`, `@here`/`@channel` or `#channel` mention in a cleaned snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetMention {
    pub range: (usize, usize),
    pub kind: MentionKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MentionKind {
    /// A person, by username (without the `@`).
    User(String),
    /// `@here`, `@channel` or `@everyone`.
    Broadcast,
    Channel,
}

/// Hair space: a break opportunity that shapes to a glyph but is all but invisible.
const BREAK_AFTER: char = '\u{200A}';

/// One run of a snippet with uniform styling: the mention it belongs to, if
/// any, and whether it is part of a query match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnippetRun<'a> {
    pub range: (usize, usize),
    pub mention: Option<&'a MentionKind>,
    pub matched: bool,
}

impl CleanSnippet {
    /// Splits the text into runs at every match and mention boundary, so a
    /// match inside a mention keeps the mention's styling around it.
    pub fn runs(&self) -> Vec<SnippetRun<'_>> {
        let mut cuts = vec![0, self.text.len()];
        for &(start, end) in &self.ranges {
            cuts.extend([start, end]);
        }
        for mention in &self.mentions {
            cuts.extend([mention.range.0, mention.range.1]);
        }
        cuts.sort_unstable();
        cuts.dedup();
        cuts.windows(2)
            .filter(|cut| cut[0] < cut[1])
            .map(|cut| {
                let (start, end) = (cut[0], cut[1]);
                let inside = |range: (usize, usize)| range.0 <= start && end <= range.1;
                SnippetRun {
                    range: (start, end),
                    mention: self
                        .mentions
                        .iter()
                        .find(|mention| inside(mention.range))
                        .map(|mention| &mention.kind),
                    matched: self.ranges.iter().any(|&range| inside(range)),
                }
            })
            .collect()
    }
}

/// Builds the cleaned text and, for every raw byte offset, where it lands in
/// the output. A range start maps through `starts`, a range end through
/// `ends`; they differ only inside a replaced span such as an emoji
/// shortcode, so a match inside `:tada:` highlights the whole emoji.
struct Builder<'a> {
    raw: &'a str,
    out: String,
    starts: Vec<usize>,
    ends: Vec<usize>,
    pending_space: bool,
    mentions: Vec<SnippetMention>,
}

impl<'a> Builder<'a> {
    fn new(raw: &'a str) -> Self {
        Self {
            raw,
            out: String::with_capacity(raw.len()),
            starts: vec![0; raw.len() + 1],
            ends: vec![0; raw.len() + 1],
            pending_space: false,
            mentions: Vec::new(),
        }
    }

    /// Drops `raw[from..to]` from the output. A match starting here begins at
    /// the next visible character; one ending here ends at the last one.
    fn skip(&mut self, from: usize, to: usize) {
        let end = self.out.len();
        let start = end + usize::from(self.pending_space);
        for offset in from..to {
            self.starts[offset] = start;
            self.ends[offset] = end;
        }
    }

    /// Collapses `raw[from..to]` (whitespace or a removed block marker) into at
    /// most one space before the next visible character.
    fn space(&mut self, from: usize, to: usize) {
        if !self.out.is_empty() {
            self.pending_space = true;
        }
        self.skip(from, to);
    }

    fn flush_space(&mut self) {
        if self.pending_space {
            self.out.push(' ');
            self.pending_space = false;
        }
    }

    /// Emits `text` in place of `raw[from..to]`.
    fn replace(&mut self, from: usize, to: usize, text: &str) {
        self.flush_space();
        let start = self.out.len();
        self.out.push_str(text);
        let end = self.out.len();
        self.starts[from] = start;
        self.ends[from] = start;
        for offset in from + 1..to {
            self.starts[offset] = start;
            self.ends[offset] = end;
        }
    }

    fn copy(&mut self, from: usize, to: usize) {
        let text = &self.raw[from..to];
        self.replace(from, to, text);
    }

    fn finish(mut self, ranges: &[(usize, usize)]) -> CleanSnippet {
        let len = self.raw.len();
        self.starts[len] = self.out.len();
        self.ends[len] = self.out.len();
        let mut mapped: Vec<(usize, usize)> = ranges
            .iter()
            .filter(|(start, end)| start < end && *end <= len)
            .map(|&(start, end)| (self.starts[start], self.ends[end]))
            .filter(|(start, end)| start < end)
            .collect();
        mapped.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(mapped.len());
        for (start, end) in mapped {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        CleanSnippet {
            text: self.out,
            ranges: merged,
            mentions: self.mentions,
        }
    }
}

/// Cleans a raw snippet for display. `ranges` are byte ranges of query
/// matches in `raw`; ranges that don't sit on char boundaries are dropped.
pub fn clean_search_snippet(raw: &str, ranges: &[(usize, usize)]) -> CleanSnippet {
    let ranges: Vec<(usize, usize)> = ranges
        .iter()
        .copied()
        .filter(|&(start, end)| {
            start < end
                && end <= raw.len()
                && raw.is_char_boundary(start)
                && raw.is_char_boundary(end)
        })
        .collect();
    let mut builder = Builder::new(raw);
    let chars: Vec<(usize, char)> = raw.char_indices().collect();
    let byte_at = |index: usize| chars.get(index).map_or(raw.len(), |(offset, _)| *offset);
    let mut in_code = false;
    let mut at_line_start = true;
    let mut index = 0;
    while index < chars.len() {
        let (offset, ch) = chars[index];
        // Code fence, with an optional language tag after the opening fence.
        if raw[offset..].starts_with("```") {
            let mut end = index + 3;
            if !in_code {
                while end < chars.len() && chars[end].1.is_ascii_alphanumeric() {
                    end += 1;
                }
            }
            in_code = !in_code;
            builder.space(offset, byte_at(end));
            index = end;
            at_line_start = false;
            continue;
        }
        if ch == '`' {
            builder.skip(offset, offset + 1);
            in_code = !in_code;
            index += 1;
            at_line_start = false;
            continue;
        }
        if ch.is_whitespace() {
            let is_newline = ch == '\n';
            builder.space(offset, offset + ch.len_utf8());
            index += 1;
            at_line_start = is_newline || at_line_start;
            continue;
        }
        // Block quote marker at the start of a line.
        if at_line_start && ch == '>' && !in_code {
            builder.space(offset, offset + 1);
            index += 1;
            continue;
        }
        at_line_start = false;
        if !in_code {
            if let Some((end, kind)) = mention_at(&chars, index) {
                builder.flush_space();
                let start = builder.out.len();
                // Char by char, so a match inside the name maps exactly.
                for &(char_offset, name_char) in &chars[index..end] {
                    builder.copy(char_offset, char_offset + name_char.len_utf8());
                }
                builder.mentions.push(SnippetMention {
                    range: (start, builder.out.len()),
                    kind,
                });
                index = end;
                continue;
            }
            if ch == ':'
                && let Some((end, emoji)) = emoji_shortcode_at(&chars, index)
            {
                builder.replace(offset, byte_at(end), emoji);
                index = end;
                continue;
            }
            if matches!(ch, '*' | '_' | '~') && is_emphasis_marker(&chars, index) {
                builder.skip(offset, offset + 1);
                index += 1;
                continue;
            }
        }
        builder.copy(offset, offset + ch.len_utf8());
        if ch == '/'
            && index > 0
            && !chars[index - 1].1.is_whitespace()
            && chars.get(index + 1).is_some_and(|(_, next)| !next.is_whitespace())
        {
            builder.out.push(BREAK_AFTER);
        }
        index += 1;
    }
    builder.finish(&ranges)
}

/// A mention starting at `index`: `@name` or `#name` at the start of a word.
/// Returns the char index just past the name and what it mentions. A trailing
/// `.` or `-` is punctuation, not part of the name.
fn mention_at(chars: &[(usize, char)], index: usize) -> Option<(usize, MentionKind)> {
    let sigil = chars[index].1;
    if sigil != '@' && sigil != '#' {
        return None;
    }
    if index > 0 {
        let prev = chars[index - 1].1;
        if prev.is_alphanumeric() || matches!(prev, '_' | '@' | '#' | '/' | '.') {
            return None;
        }
    }
    let is_name_char = |ch: char| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.');
    let mut end = index + 1;
    while end < chars.len() && is_name_char(chars[end].1) {
        end += 1;
    }
    while end > index + 1 && matches!(chars[end - 1].1, '.' | '-') {
        end -= 1;
    }
    if end == index + 1 {
        return None;
    }
    let name: String = chars[index + 1..end].iter().map(|(_, ch)| *ch).collect();
    let kind = if sigil == '#' {
        MentionKind::Channel
    } else if matches!(name.to_ascii_lowercase().as_str(), "here" | "channel" | "everyone") {
        MentionKind::Broadcast
    } else {
        MentionKind::User(name)
    };
    Some((end, kind))
}

/// `:alias:` starting at `index` that names a standard emoji: the char index
/// just past the closing colon and the emoji. Custom emoji stay as written.
fn emoji_shortcode_at(chars: &[(usize, char)], index: usize) -> Option<(usize, &'static str)> {
    const MAX_ALIAS_CHARS: usize = 48;
    let mut alias = String::new();
    for (offset, &(_, ch)) in chars.iter().enumerate().skip(index + 1) {
        if ch == ':' {
            if alias.is_empty() {
                return None;
            }
            return standard_emoji_for_alias(&alias).map(|emoji| (offset + 1, emoji));
        }
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '+'))
            || alias.len() >= MAX_ALIAS_CHARS
        {
            return None;
        }
        alias.push(ch);
    }
    None
}

/// A `*`, `_` or `~` that opens or closes emphasis rather than sitting inside a
/// word (`snake_case`) or standing alone between spaces (`2 * 3`).
fn is_emphasis_marker(chars: &[(usize, char)], index: usize) -> bool {
    let marker = chars[index].1;
    // Treat a run such as `**` like a single marker.
    let mut before = index;
    while before > 0 && chars[before - 1].1 == marker {
        before -= 1;
    }
    let mut after = index;
    while after + 1 < chars.len() && chars[after + 1].1 == marker {
        after += 1;
    }
    let prev = before.checked_sub(1).map(|i| chars[i].1);
    let next = chars.get(after + 1).map(|(_, ch)| *ch);
    let is_word = |ch: Option<char>| ch.is_some_and(char::is_alphanumeric);
    let is_space = |ch: Option<char>| ch.is_none_or(char::is_whitespace);
    if is_word(prev) && is_word(next) {
        return false;
    }
    if is_space(prev) && is_space(next) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn highlighted(snippet: &CleanSnippet) -> Vec<&str> {
        snippet
            .ranges
            .iter()
            .map(|&(start, end)| &snippet.text[start..end])
            .collect()
    }

    fn ranges_of(raw: &str, needle: &str) -> Vec<(usize, usize)> {
        raw.to_lowercase()
            .match_indices(&needle.to_lowercase())
            .map(|(start, found)| (start, start + found.len()))
            .collect()
    }

    #[test]
    fn collapses_lines_and_keeps_plain_matches() {
        let raw = "Release notes draft:\nPlease  skim it before the release.";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "release"));
        assert_eq!(
            clean.text,
            "Release notes draft: Please skim it before the release."
        );
        assert_eq!(highlighted(&clean), ["Release", "release"]);
    }

    #[test]
    fn renders_emoji_shortcodes_and_shifts_later_matches() {
        let raw = "Shipped :tada: the release :rocket:";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "release"));
        assert_eq!(clean.text, "Shipped 🎉 the release 🚀");
        assert_eq!(highlighted(&clean), ["release"]);
    }

    #[test]
    fn a_match_inside_a_shortcode_highlights_the_emoji() {
        let raw = "party :tada: time";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "tada"));
        assert_eq!(clean.text, "party 🎉 time");
        assert_eq!(highlighted(&clean), ["🎉"]);
    }

    #[test]
    fn unknown_shortcodes_and_clock_times_stay_as_written() {
        let raw = "at 10:15:30 we add :lighthouse_logo:";
        assert_eq!(clean_search_snippet(raw, &[]).text, raw);
    }

    #[test]
    fn strips_markdown_markers_but_not_word_internal_punctuation() {
        let raw = "Use `lh migrate --dry-run` and *read* _carefully_ ~old~ my_var 2 * 3";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "migrate"));
        assert_eq!(
            clean.text,
            "Use lh migrate --dry-run and read carefully old my_var 2 * 3"
        );
        assert_eq!(highlighted(&clean), ["migrate"]);
    }

    #[test]
    fn removes_code_fences_language_tags_and_quote_markers() {
        let raw = "> quoted release\n```rust\nlet release = 1;\n```\ndone";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "release"));
        assert_eq!(clean.text, "quoted release let release = 1; done");
        assert_eq!(highlighted(&clean), ["release", "release"]);
    }

    #[test]
    fn code_spans_keep_their_contents_verbatim() {
        let raw = "rename `:tada:` and `*ptr`";
        assert_eq!(
            clean_search_snippet(raw, &[]).text,
            "rename :tada: and *ptr"
        );
    }

    #[test]
    fn a_match_that_spans_a_removed_marker_stays_contiguous() {
        let raw = "the **release** build";
        let clean = clean_search_snippet(raw, &[(4, 15)]);
        assert_eq!(clean.text, "the release build");
        assert_eq!(highlighted(&clean), ["release"]);
    }

    #[test]
    fn invalid_or_out_of_bounds_ranges_are_dropped() {
        let raw = "héllo";
        let clean = clean_search_snippet(raw, &[(2, 3), (0, 99), (3, 3)]);
        assert_eq!(clean.text, "héllo");
        assert!(clean.ranges.is_empty());
    }

    #[test]
    fn breaks_after_a_slash_inside_a_path_never_before_it() {
        let raw = "The export lives in assets/release/banner@2x.png, please";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "release"));
        assert!(clean.text.contains("assets/\u{200A}release/\u{200A}banner@2x.png"));
        // Every break opportunity at a slash follows it.
        for (offset, ch) in clean.text.char_indices() {
            if ch == BREAK_AFTER {
                assert!(clean.text[..offset].ends_with('/'));
            }
        }
        assert_eq!(highlighted(&clean), ["release"]);
        assert_eq!(clean.text.replace(BREAK_AFTER, ""), raw);
    }

    #[test]
    fn standalone_and_trailing_slashes_get_no_break_hint() {
        let raw = "either / or and a trailing dir/";
        assert_eq!(clean_search_snippet(raw, &[]).text, raw);
    }

    #[test]
    fn marks_user_broadcast_and_channel_mentions() {
        let raw = "@robin can you take it? Checklist is in #releases. cc @here, mail me@example.com";
        let clean = clean_search_snippet(raw, &[]);
        let mentions: Vec<(&str, &MentionKind)> = clean
            .mentions
            .iter()
            .map(|mention| (&clean.text[mention.range.0..mention.range.1], &mention.kind))
            .collect();
        assert_eq!(
            mentions,
            [
                ("@robin", &MentionKind::User("robin".to_string())),
                ("#releases", &MentionKind::Channel),
                ("@here", &MentionKind::Broadcast),
            ]
        );
    }

    #[test]
    fn mentions_in_code_are_left_alone() {
        let raw = "run `git log @{u}` then ping @sam";
        let clean = clean_search_snippet(raw, &[]);
        assert_eq!(clean.mentions.len(), 1);
        assert_eq!(clean.mentions[0].kind, MentionKind::User("sam".to_string()));
    }

    #[test]
    fn runs_split_a_match_inside_a_mention_and_keep_the_mention() {
        let raw = "in #releases now";
        let clean = clean_search_snippet(raw, &ranges_of(raw, "release"));
        let runs: Vec<(&str, bool, bool)> = clean
            .runs()
            .into_iter()
            .map(|run| {
                (
                    &clean.text[run.range.0..run.range.1],
                    run.mention.is_some(),
                    run.matched,
                )
            })
            .collect();
        assert_eq!(
            runs,
            [
                ("in ", false, false),
                ("#", true, false),
                ("release", true, true),
                ("s", true, false),
                (" now", false, false),
            ]
        );
    }
}
