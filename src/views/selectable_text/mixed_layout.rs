//! Layout for paragraphs with smaller inline-code runs. GPUI's built-in text
//! layout accepts one font size per paragraph; keep sizing, painting and hit
//! testing together here so code can be 12px without shrinking the body text.
use gpui::{App, Bounds, Pixels, Point, ShapedLine, Size, TextRun, Window, point, px, size};
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
    mut measure: impl FnMut(Range<usize>, usize, Pixels) -> Pixels,
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

    for (word_start, word) in text.split_word_bound_indices() {
        if word == "\n" || word == "\r\n" {
            result.line_starts.push(word_start + word.len());
            x = px(0.);
            y += line_height;
            continue;
        }
        let word_end = word_start + word.len();
        let pieces = styled
            .iter()
            .filter_map(|(range, run, font_size)| {
                let range = range.start.max(word_start)..range.end.min(word_end);
                (range.start < range.end).then_some((range, *run, *font_size))
            })
            .collect::<Vec<_>>();
        let total = pieces
            .iter()
            .map(|(range, run, font_size)| measure(range.clone(), *run, *font_size))
            .fold(px(0.), |sum, value| sum + value);
        if x > px(0.) && x + total > width {
            result.line_starts.push(word_start);
            x = px(0.);
            y += line_height;
        }
        for (range, run, font_size) in pieces {
            let mut start = range.start;
            while start < range.end {
                let full_width = measure(start..range.end, run, font_size);
                let (end, measured) = if full_width <= width - x {
                    (range.end, full_width)
                } else {
                    // Split an overlong token at grapheme boundaries, keeping UTF-8,
                    // selection and keyboard navigation in the original byte space.
                    let ends = text[start..range.end]
                        .grapheme_indices(true)
                        .map(|(index, grapheme)| start + index + grapheme.len())
                        .collect::<Vec<_>>();
                    let mut low = 0;
                    let mut high = ends.len();
                    while low < high {
                        let mid = (low + high) / 2;
                        if measure(start..ends[mid], run, font_size) <= width - x {
                            low = mid + 1;
                        } else {
                            high = mid;
                        }
                    }
                    let end = ends[low.saturating_sub(1)];
                    (end, measure(start..end, run, font_size))
                };
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
                if start < range.end {
                    result.line_starts.push(start);
                    x = px(0.);
                    y += line_height;
                }
            }
        }
    }
    result
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
            |range, run, size| shape(range, run, size).width,
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
                let _ = painted
                    .line
                    .paint_background(position, self.line_height, window, cx);
            } else {
                let _ = painted.line.paint(position, self.line_height, window, cx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn runs(lengths: &[usize]) -> Vec<TextRun> {
        lengths
            .iter()
            .map(|len| TextRun {
                len: *len,
                font: gpui::font("Menlo"),
                color: gpui::rgb(0xffffff).into(),
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
            &[0..text.len()],
            px(14.),
            px(22.),
            px(200.),
            |range, _, size| size * text[range].graphemes(true).count() as f32 / 2.,
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
            &[start..end],
            px(14.),
            px(22.),
            px(80.),
            |range, _, size| size * text[range].chars().count() as f32 / 2.,
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
}
