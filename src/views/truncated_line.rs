//! One line of styled text that ends in "…" when it does not fit its width.
//!
//! GPUI's own `text_ellipsis` truncates during measurement and keeps the result
//! of the first measurement it sees. Inside virtualized lists that first pass can
//! use a throwaway width, leaving a few characters behind. This element lays out
//! at the full available width and truncates while painting, so it always uses
//! its final bounds.
use gpui_kit::{
    App, Bounds, Element, ElementId, FontWeight, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextRun, Window, relative,
};

const ELLIPSIS: &str = "…";

/// A run of text with its own color and weight.
pub struct Segment {
    pub text: String,
    pub color: Hsla,
    pub weight: FontWeight,
}

pub struct TruncatedLine {
    segments: Vec<Segment>,
}

impl TruncatedLine {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self { segments }
    }
}

impl IntoElement for TruncatedLine {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TruncatedLine {
    type RequestLayoutState = ();
    type PrepaintState = Option<(ShapedLine, Pixels)>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let text_style = window.text_style();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let mut text = String::new();
        let mut runs = Vec::new();
        for segment in &self.segments {
            if segment.text.is_empty() {
                continue;
            }
            let mut run = text_style.to_run(segment.text.len());
            run.color = segment.color;
            run.font.weight = segment.weight;
            text.push_str(&segment.text);
            runs.push(run);
        }
        if text.is_empty() {
            return None;
        }
        let shape = |text: &str, runs: &[TextRun], window: &mut Window| {
            window.text_system().shape_line(
                SharedString::from(text.to_owned()),
                font_size,
                runs,
                None,
            )
        };
        let line = shape(&text, &runs, window);
        if line.width <= bounds.size.width {
            return Some((line, window.line_height()));
        }
        let ellipsis_run = {
            let mut run = runs.last().cloned()?;
            run.len = ELLIPSIS.len();
            run
        };
        let ellipsis_width = shape(ELLIPSIS, &[ellipsis_run], window).width;
        let cut = line
            .index_for_x((bounds.size.width - ellipsis_width).max(Pixels::ZERO))
            .unwrap_or(text.len());
        let kept = text[..cut].trim_end().len();
        let (text, runs) = truncate_runs(&text, &runs, kept);
        Some((shape(&text, &runs, window), window.line_height()))
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some((line, line_height)) = prepaint.as_ref() {
            let _ = line.paint(
                bounds.origin,
                *line_height,
                gpui_kit::TextAlign::Left,
                None,
                window,
                cx,
            );
        }
    }
}

/// Keeps the first `kept` bytes of `text` and appends "…" in the style of the
/// last kept run.
fn truncate_runs(text: &str, runs: &[TextRun], kept: usize) -> (String, Vec<TextRun>) {
    let mut out = Vec::new();
    let mut remaining = kept;
    for run in runs {
        if remaining == 0 {
            break;
        }
        let mut run = run.clone();
        run.len = run.len.min(remaining);
        remaining -= run.len;
        out.push(run);
    }
    match out.last_mut() {
        Some(last) => last.len += ELLIPSIS.len(),
        None => {
            if let Some(first) = runs.first() {
                let mut run = first.clone();
                run.len = ELLIPSIS.len();
                out.push(run);
            }
        }
    }
    (format!("{}{ELLIPSIS}", &text[..kept]), out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(len: usize, weight: FontWeight) -> TextRun {
        TextRun {
            len,
            font: gpui_kit::Font {
                weight,
                ..gpui_kit::font(".SystemUIFont")
            },
            color: gpui_kit::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn truncation_keeps_run_styles_and_appends_one_ellipsis() {
        let text = "Alice Johnson Final mocks";
        let runs = [run(13, FontWeight::SEMIBOLD), run(12, FontWeight::NORMAL)];
        let (short, short_runs) = truncate_runs(text, &runs, "Alice Johnson Fin".len());
        assert_eq!(short, "Alice Johnson Fin…");
        assert_eq!(
            short_runs.iter().map(|run| run.len).sum::<usize>(),
            short.len()
        );
        assert_eq!(short_runs[0].font.weight, FontWeight::SEMIBOLD);
        assert_eq!(short_runs[1].len, " Fin…".len());

        let (inside_first, first_runs) = truncate_runs(text, &runs, 3);
        assert_eq!(inside_first, "Ali…");
        assert_eq!(first_runs.len(), 1);
        assert_eq!(first_runs[0].len, inside_first.len());

        let (empty, empty_runs) = truncate_runs(text, &runs, 0);
        assert_eq!(empty, "…");
        assert_eq!(empty_runs[0].len, ELLIPSIS.len());
    }
}
