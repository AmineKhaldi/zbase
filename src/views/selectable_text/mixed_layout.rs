//! Layout for paragraphs with smaller inline-code runs. GPUI's built-in text
//! layout accepts one font size per paragraph; keep sizing, painting and hit
//! testing together here so code can be 12px without shrinking the body text.
use gpui_kit::{App, Bounds, Pixels, Point, ShapedLine, Size, TextRun, Window, point, px, size};
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Debug)]
struct Span {
    range: Range<usize>,
    run: usize,
    font_size: Pixels,
    origin: Point<Pixels>,
    width: Pixels,
}

#[derive(Default, Debug)]
struct Plan {
    spans: Vec<Span>,
    line_starts: Vec<usize>,
    width: Pixels,
}

struct PaintedSpan {
    span: Span,
    line: ShapedLine,
    baseline_offset: Pixels,
}

#[derive(Clone)]
pub(super) struct MixedLayout {
    spans: Arc<Vec<PaintedSpan>>,
    line_starts: Arc<Vec<usize>>,
    runs: Arc<Vec<TextRun>>,
    body_size: Pixels,
    wrap_width: Pixels,
    line_height: Pixels,
    text_len: usize,
    measured: Size<Pixels>,
}

fn plan(
    text: &str,
    runs: &[TextRun],
    code_ranges: &[Range<usize>],
    body_size: Pixels,
    line_height: Pixels,
    wrap_width: Pixels,
    mut advances: impl FnMut(Range<usize>, usize, Pixels) -> Vec<(usize, Pixels)>,
) -> Plan {
    let mut styled = Vec::new();
    let mut start = 0;
    for (index, run) in runs.iter().enumerate() {
        let end = (start + run.len).min(text.len());
        let font_size = if code_ranges
            .iter()
            .any(|range| range.start <= start && range.end >= end)
        {
            crate::views::inline_code_font_size().min(body_size)
        } else {
            body_size
        };
        styled.push((start..end, index, font_size));
        start = end;
    }
    let mut result = Plan {
        line_starts: vec![0],
        ..Default::default()
    };
    let mut x = px(0.);
    let mut y = px(0.);
    let width = wrap_width.max(px(1.));

    for (word_start, word) in break_units(text) {
        if word == "\n" || word == "\r\n" {
            result.line_starts.push(word_start + word.len());
            x = px(0.);
            y += line_height;
            continue;
        }
        let word_end = word_start + word.len();
        // Trailing spaces hang past the edge instead of forcing a wrap.
        let visible_end = word_start + word.trim_end_matches(is_space).len();
        let pieces = styled
            .iter()
            .filter_map(|(range, run, font_size)| {
                let range = range.start.max(word_start)..range.end.min(word_end);
                (range.start < range.end).then(|| {
                    let measured = advances(range.clone(), *run, *font_size);
                    (range, *run, *font_size, measured)
                })
            })
            .collect::<Vec<_>>();
        let visible = pieces
            .iter()
            .map(|(_, _, _, advances)| {
                advances
                    .iter()
                    .take_while(|(end, _)| *end <= visible_end)
                    .last()
                    .map_or(px(0.), |(_, x)| *x)
            })
            .fold(px(0.), |sum, value| sum + value);
        if x > px(0.) && x + visible > width {
            result.line_starts.push(word_start);
            x = px(0.);
            y += line_height;
        }
        for (range, run, font_size, advances) in pieces {
            let mut start = range.start;
            let mut first = 0;
            let mut consumed = px(0.);
            while first < advances.len() {
                if start >= visible_end {
                    // Only trailing whitespace is left: let it hang, clipped to the edge.
                    let (end, advance) = advances[advances.len() - 1];
                    let measured = (advance - consumed).min((width - x).max(px(0.)));
                    result.spans.push(Span {
                        range: start..end,
                        run,
                        font_size,
                        origin: point(x, y),
                        width: measured,
                    });
                    x += measured;
                    result.width = result.width.max(x);
                    break;
                }
                // Shape the piece once. Break by cumulative grapheme advances rather
                // than repeatedly shaping the remaining suffix on every line.
                let fitting = advances[first..]
                    .partition_point(|(_, advance)| *advance - consumed <= width - x);
                let last = first + fitting.max(1) - 1;
                let (end, advance) = advances[last];
                let measured = advance - consumed;
                result.spans.push(Span {
                    range: start..end,
                    run,
                    font_size,
                    origin: point(x, y),
                    width: measured,
                });
                x += measured;
                result.width = result.width.max(x);
                start = end;
                first = last + 1;
                consumed = advance;
                if start < range.end && start < visible_end {
                    result.line_starts.push(start);
                    x = px(0.);
                    y += line_height;
                }
            }
        }
    }
    result
}

/// Splits text into runs that must stay on one line: a word with its trailing
/// spaces and any punctuation glued to it ("please!", "(maybe)", "\"Skip\"?").
/// Newlines come out as their own units.
fn break_units(text: &str) -> Vec<(usize, &str)> {
    let mut units: Vec<(usize, usize)> = Vec::new();
    let mut previous: Option<&str> = None;
    for (start, token) in text.split_word_bound_indices() {
        let is_newline = token == "\n" || token == "\r\n";
        let joins = !is_newline
            && previous.is_some_and(|previous| {
                previous != "\n" && previous != "\r\n" && !break_allowed_between(previous, token)
            });
        match units.last_mut() {
            Some((_, end)) if joins => *end = start + token.len(),
            _ => units.push((start, start + token.len())),
        }
        previous = Some(token);
    }
    units
        .into_iter()
        .map(|(start, end)| (start, &text[start..end]))
        .collect()
}

/// Line-break opportunity between two adjacent word-boundary tokens.
fn break_allowed_between(previous: &str, next: &str) -> bool {
    let (Some(before), Some(after)) = (previous.chars().last(), next.chars().next()) else {
        return true;
    };
    if is_space(after) || is_closing_punctuation(after) {
        return false;
    }
    if is_space(before) {
        return true;
    }
    if is_ambiguous_quote(after) || is_opening_punctuation(before) || is_ambiguous_quote(before) {
        return false;
    }
    // Paths and "and/or" may break after the slash; scripts written without
    // spaces break between characters.
    before == '/' || is_ideographic(before) || is_ideographic(after)
}

/// Breakable spaces. Not U+2003, which stands in for inline custom emoji, and
/// not no-break spaces.
fn is_space(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\u{3000}')
}

const CLOSING_PUNCTUATION: &[char] = &[
    '!', '?', '.', ',', ';', ':', ')', ']', '}', '%', '”', '’', '»', '›', '…', '‼', '⁇', '、',
    '。', '，', '．', '！', '？', '：', '；', '）', '］', '｝', '」', '』', '】', '〉', '》', '〕',
];

const OPENING_PUNCTUATION: &[char] = &[
    '(', '[', '{', '“', '‘', '«', '‹', '（', '［', '｛', '「', '『', '【', '〈', '《', '〔', '¿',
    '¡',
];

fn is_closing_punctuation(ch: char) -> bool {
    CLOSING_PUNCTUATION.contains(&ch)
}

fn is_opening_punctuation(ch: char) -> bool {
    OPENING_PUNCTUATION.contains(&ch)
}

/// Straight quotes open or close depending on their neighbours.
fn is_ambiguous_quote(ch: char) -> bool {
    matches!(ch, '"' | '\'')
}

fn is_ideographic(ch: char) -> bool {
    matches!(ch,
        '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        | '\u{ac00}'..='\u{d7af}' | '\u{f900}'..='\u{faff}' | '\u{20000}'..='\u{2fa1f}')
}

/// Whether GPUI's paragraph wrapper could put punctuation on a line of its own:
/// it treats "!", "?", ")" and similar as break opportunities even when glued to
/// the previous word. Such paragraphs use the word-based layout instead.
pub(super) fn has_glued_closing_punctuation(text: &str) -> bool {
    let mut previous = '\n';
    text.chars().any(|ch| {
        // GPUI keeps these attached to the word before them already.
        let glued = if ch == '"' {
            !previous.is_whitespace()
        } else {
            is_closing_punctuation(ch) && !matches!(ch, '.' | ',' | ':' | '%') && previous != '\n'
        };
        previous = ch;
        glued
    })
}

impl MixedLayout {
    // Keep GPUI's whole-paragraph bidirectional shaping for these paragraphs;
    // the word-based mixed-size path currently handles left-to-right text only.
    pub(super) fn supports(text: &str) -> bool {
        !text.chars().any(|ch| {
            matches!(ch,
            '\u{0590}'..='\u{08ff}' | '\u{fb1d}'..='\u{fdff}' | '\u{fe70}'..='\u{feff}'
            | '\u{1ee00}'..='\u{1eeff}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        })
    }

    pub(super) fn shape(
        text: &str,
        runs: &[TextRun],
        code_ranges: &[Range<usize>],
        body_size: Pixels,
        line_height: Pixels,
        wrap_width: Pixels,
        window: &mut Window,
    ) -> Self {
        let shape = |range: Range<usize>, run: usize, font_size: Pixels| {
            let mut style = runs[run].clone();
            style.len = range.len();
            window.text_system().shape_line(
                text[range].to_owned().into(),
                font_size,
                &[style],
                None,
            )
        };
        let plan = plan(
            text,
            runs,
            code_ranges,
            body_size,
            line_height,
            wrap_width,
            |range, run, size| {
                let line = shape(range.clone(), run, size);
                let mut glyphs = line
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .peekable();
                text[range.clone()]
                    .grapheme_indices(true)
                    .map(|(offset, grapheme)| {
                        let end = offset + grapheme.len();
                        // x_for_index scans from the first glyph. Walk forward once
                        // instead, so collecting advances is linear for long tokens.
                        while glyphs.peek().is_some_and(|glyph| glyph.index < end) {
                            glyphs.next();
                        }
                        let x = glyphs.peek().map_or(line.width, |glyph| glyph.position.x);
                        (range.start + end, x)
                    })
                    .collect()
            },
        );
        let body_baseline = runs
            .first()
            .map(|run| {
                let mut run = run.clone();
                run.len = 1;
                let line = window
                    .text_system()
                    .shape_line("M".into(), body_size, &[run], None);
                (line.ascent - line.descent) / 2.
            })
            .unwrap_or(px(0.));
        let measured = size(
            plan.width.min(wrap_width),
            line_height * plan.line_starts.len(),
        );
        Self {
            spans: Arc::new(
                plan.spans
                    .into_iter()
                    .map(|span| {
                        let line = shape(span.range.clone(), span.run, span.font_size);
                        let baseline_offset = body_baseline - (line.ascent - line.descent) / 2.;
                        PaintedSpan {
                            span,
                            line,
                            baseline_offset,
                        }
                    })
                    .collect(),
            ),
            line_starts: Arc::new(plan.line_starts),
            runs: Arc::new(runs.to_vec()),
            body_size,
            wrap_width,
            line_height,
            text_len: text.len(),
            measured,
        }
    }

    pub(super) fn matches(
        &self,
        body_size: Pixels,
        line_height: Pixels,
        width: Pixels,
        runs: &[TextRun],
    ) -> bool {
        self.body_size == body_size
            && self.line_height == line_height
            && self.wrap_width == width
            && self.runs.as_slice() == runs
    }

    pub(super) fn measured(&self) -> Size<Pixels> {
        self.measured
    }

    pub(super) fn closest_index(&self, position: Point<Pixels>) -> usize {
        if position.y < px(0.) {
            return 0;
        }
        let line = (f32::from(position.y) / f32::from(self.line_height)) as usize;
        if line >= self.line_starts.len() {
            return self.text_len;
        }
        let mut end = self.line_starts[line];
        for painted in self
            .spans
            .iter()
            .filter(|span| span.span.origin.y == self.line_height * line)
        {
            let span = &painted.span;
            if position.x < span.origin.x + span.width {
                return span.range.start
                    + painted
                        .line
                        .closest_index_for_x((position.x - span.origin.x).max(px(0.)));
            }
            end = span.range.end;
        }
        end
    }

    pub(super) fn position_for_index(&self, index: usize) -> Point<Pixels> {
        if let Some(painted) = self
            .spans
            .iter()
            .find(|span| span.span.range.contains(&index))
        {
            return point(
                painted.span.origin.x + painted.line.x_for_index(index - painted.span.range.start),
                painted.span.origin.y,
            );
        }
        if let Ok(line) = self.line_starts.binary_search(&index) {
            return point(px(0.), self.line_height * line);
        }
        if let Some(painted) = self
            .spans
            .iter()
            .rev()
            .find(|span| span.span.range.end <= index)
        {
            return point(
                painted.span.origin.x + painted.span.width,
                painted.span.origin.y,
            );
        }
        Point::default()
    }

    pub(super) fn bounds_for_range(
        &self,
        range: Range<usize>,
        origin: Point<Pixels>,
    ) -> Vec<Bounds<Pixels>> {
        self.spans
            .iter()
            .filter_map(|painted| {
                let span = &painted.span;
                let start = range.start.max(span.range.start);
                let end = range.end.min(span.range.end);
                if start >= end {
                    return None;
                }
                let x1 = painted.line.x_for_index(start - span.range.start);
                let x2 = painted.line.x_for_index(end - span.range.start);
                Some(Bounds::new(
                    origin + span.origin + point(x1, px(0.)),
                    size(x2 - x1, self.line_height),
                ))
            })
            .collect()
    }

    pub(super) fn paint(
        &self,
        origin: Point<Pixels>,
        backgrounds: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        for painted in self.spans.iter() {
            let position = origin + painted.span.origin + point(px(0.), painted.baseline_offset);
            if backgrounds {
                let _ = painted.line.paint_background(
                    position,
                    self.line_height,
                    gpui_kit::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            } else {
                let _ = painted.line.paint(
                    position,
                    self.line_height,
                    gpui_kit::TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixed_advances(text: &str, range: Range<usize>, size: Pixels) -> Vec<(usize, Pixels)> {
        text[range.clone()]
            .grapheme_indices(true)
            .enumerate()
            .map(|(index, (offset, grapheme))| {
                (
                    range.start + offset + grapheme.len(),
                    size * (index + 1) as f32 / 2.,
                )
            })
            .collect()
    }

    #[test]
    fn long_tokens_are_measured_once_regardless_of_wrapping() {
        for length in [1000, 2000, 4000, 8000] {
            let text = "x".repeat(length);
            let mut measured_bytes = 0;
            let result = plan(
                &text,
                &runs(&[length]),
                std::slice::from_ref(&(0..length)),
                px(14.),
                px(22.),
                px(30.),
                |range, _, size| {
                    measured_bytes += range.len();
                    fixed_advances(&text, range, size)
                },
            );
            assert_eq!(measured_bytes, length);
            assert!(result.line_starts.len() > 100);
            assert_eq!(result.spans.last().unwrap().range.end, length);
        }
    }

    fn runs(lengths: &[usize]) -> Vec<TextRun> {
        lengths
            .iter()
            .map(|len| TextRun {
                len: *len,
                font: gpui_kit::font("Menlo"),
                color: gpui_kit::rgb(0xffffff).into(),
                background_color: None,
                underline: None,
                strikethrough: None,
            })
            .collect()
    }
    #[test]
    fn explicit_newlines_and_graphemes_keep_their_original_indices() {
        let text = "λcode\n\n👨‍👩‍👧‍👦 end";
        let result = plan(
            text,
            &runs(&[text.len()]),
            std::slice::from_ref(&(0..text.len())),
            px(14.),
            px(22.),
            px(200.),
            |range, _, size| fixed_advances(text, range, size),
        );
        assert_eq!(
            result.line_starts,
            vec![0, "λcode\n".len(), "λcode\n\n".len()]
        );
        assert_eq!(
            result
                .spans
                .iter()
                .map(|span| &text[span.range.clone()])
                .collect::<String>(),
            text.replace('\n', "")
        );
        let boundaries = text
            .grapheme_indices(true)
            .map(|(index, _)| index)
            .chain(std::iter::once(text.len()))
            .collect::<Vec<_>>();
        for span in result.spans {
            assert!(boundaries.contains(&span.range.start) && boundaries.contains(&span.range.end));
        }
        assert!(MixedLayout::supports("body λcode 👋"));
        assert!(!MixedLayout::supports("שלום code"));
        assert!(!MixedLayout::supports("\u{202e}code"));
    }

    #[test]
    fn inline_size_changes_only_code_and_wraps_without_losing_utf8() {
        let text = "body λcode very_long_identifier end";
        let start = "body ".len();
        let end = text.len() - " end".len();
        let runs = runs(&[start, end - start, text.len() - end]);
        let result = plan(
            text,
            &runs,
            std::slice::from_ref(&(start..end)),
            px(14.),
            px(22.),
            px(80.),
            |range, _, size| fixed_advances(text, range, size),
        );
        let copied = result
            .spans
            .iter()
            .map(|span| &text[span.range.clone()])
            .collect::<String>();
        assert_eq!(copied, text);
        for span in &result.spans {
            assert!(
                text.is_char_boundary(span.range.start) && text.is_char_boundary(span.range.end)
            );
            assert_eq!(
                span.font_size,
                if span.run == 1 { px(12.) } else { px(14.) }
            );
            assert!(span.origin.x + span.width <= px(80.));
        }
        assert!(result.line_starts.len() > 1);
    }

    fn lines(text: &str, width: f32) -> Vec<String> {
        let result = plan(
            text,
            &runs(&[text.len()]),
            &[],
            px(14.),
            px(22.),
            px(width),
            |range, _, size| fixed_advances(text, range, size),
        );
        let mut starts = result.line_starts.clone();
        starts.push(text.len());
        starts
            .windows(2)
            .map(|line| text[line[0]..line[1]].trim_end().to_string())
            .collect()
    }

    #[test]
    fn closing_punctuation_stays_with_its_word() {
        // fixed_advances: 7px per grapheme at 14px. "before Thursday, please!" is 24
        // graphemes; at 165px "please" fits on the first line but "please!" does not.
        assert_eq!(
            lines("before Thursday, please!", 165.),
            ["before Thursday,", "please!"]
        );
        assert_eq!(
            lines("Copy review, please !", 140.),
            ["Copy review,", "please !"]
        );
        assert_eq!(
            lines("instead of \"Skip\"?", 120.),
            ["instead of", "\"Skip\"?"]
        );
        assert_eq!(lines("see (the docs)", 95.), ["see (the", "docs)"]);
    }

    #[test]
    fn break_units_glue_punctuation_and_trailing_spaces() {
        let units = |text| {
            break_units(text)
                .into_iter()
                .map(|(_, unit)| unit)
                .collect::<Vec<_>>()
        };
        assert_eq!(units("Love it. Maybe!"), ["Love ", "it. ", "Maybe!"]);
        assert_eq!(units("a \"quote\" here"), ["a ", "\"quote\" ", "here"]);
        assert_eq!(units("x-ray and/or"), ["x-ray ", "and/", "or"]);
        assert_eq!(units("one\ntwo"), ["one", "\n", "two"]);
        assert_eq!(units("日本語。"), ["日", "本", "語。"]);
    }

    #[test]
    fn only_paragraphs_gpui_would_strand_need_the_word_layout() {
        assert!(has_glued_closing_punctuation("please!"));
        assert!(has_glued_closing_punctuation("really ?"));
        assert!(has_glued_closing_punctuation("(see docs)"));
        assert!(has_glued_closing_punctuation("say \"hi\""));
        assert!(!has_glued_closing_punctuation(
            "Plain sentence, with a period."
        ));
        assert!(!has_glued_closing_punctuation("it's 100% done: \"ok"));
        assert!(!has_glued_closing_punctuation("!important"));
    }
}
