//! Positioned text model: PDFium chars -> glyphs -> lines -> segments, in display coordinates.
//!
//! Display coordinates are PDF points with the origin at the top-left corner of the page as it is
//! shown (page rotation applied), y growing downward. They match the rendered page image, so the
//! frontend can draw highlights and the user can draw regions in the same space.
//!
//! Every line keeps two texts built from the same glyphs: `raw` (original chars, used to find
//! money and dates) and `norm` (see [`crate::text::normalize`], used to match labels), plus a
//! byte -> glyph map for each, so a regex match maps back to a box on the page.

use crate::model::Rect;
use crate::text::fold_char;
use pdfium_render::prelude::*;
use std::ops::Range;

/// A gap wider than this fraction of the font height separates two words.
const WORD_GAP: f32 = 0.15;
/// A gap wider than this multiple of the font height separates two segments (table columns).
const SEGMENT_GAP: f32 = 1.2;
/// Glyphs whose baselines differ by less than this fraction of the font height share a line.
const BASELINE_TOLERANCE: f32 = 0.35;

/// A visible char with its box in display coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Glyph {
    pub ch: char,
    pub rect: Rect,
    /// Baseline y in display coordinates.
    pub baseline: f32,
}

/// A run of glyphs of one line separated from its neighbours by wide gaps (e.g. a table cell).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Segment {
    pub glyphs: Range<usize>,
    pub raw: Range<usize>,
    pub norm: Range<usize>,
    pub rect: Rect,
}

/// Glyphs sharing a baseline, sorted left to right.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub glyphs: Vec<Glyph>,
    pub segments: Vec<Segment>,
    /// Original text; a single space marks word and segment gaps.
    pub raw: String,
    /// Glyph index of each byte of `raw`.
    pub raw_owner: Vec<usize>,
    /// Normalized text with the same spacing as `raw`.
    pub norm: String,
    /// Glyph index of each byte of `norm`.
    pub norm_owner: Vec<usize>,
    pub rect: Rect,
}

impl Line {
    /// Box of the glyphs behind the byte range `start..end` of `raw`.
    pub fn raw_span_rect(&self, start: usize, end: usize) -> Rect {
        span_rect(&self.glyphs, &self.raw_owner, start, end)
    }

    /// Box of the glyphs behind the byte range `start..end` of `norm`.
    pub fn norm_span_rect(&self, start: usize, end: usize) -> Rect {
        span_rect(&self.glyphs, &self.norm_owner, start, end)
    }
}

/// The text of one page.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct PageText {
    /// Page size in points, as displayed.
    pub width: f32,
    pub height: f32,
    /// Sorted top to bottom, then left to right.
    pub lines: Vec<Line>,
}

impl PageText {
    /// True when the page has at least one letter or digit.
    pub fn has_text(&self) -> bool {
        self.lines
            .iter()
            .any(|line| line.glyphs.iter().any(|g| g.ch.is_alphanumeric()))
    }

    /// Raw text of the whole page, one line per text line.
    pub fn raw_text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&line.raw);
            out.push('\n');
        }
        out
    }
}

/// Maps page space (PDF user space, origin bottom-left, unrotated) to display coordinates.
#[derive(Debug, Clone, Copy)]
struct Frame {
    left: f32,
    bottom: f32,
    /// Unrotated size of the page box.
    width: f32,
    height: f32,
    /// Clockwise quarter turns (0..=3).
    quarter_turns: u8,
}

impl Frame {
    fn of(page: &PdfPage<'_>) -> Frame {
        let quarter_turns = match page.rotation() {
            Ok(PdfPageRenderRotation::Degrees90) => 1,
            Ok(PdfPageRenderRotation::Degrees180) => 2,
            Ok(PdfPageRenderRotation::Degrees270) => 3,
            _ => 0,
        };
        // `bounding()` is the media box intersected with the crop box, in page space: the same box
        // PDFium maps onto the displayed page.
        match page.boundaries().bounding() {
            Ok(boundary) if boundary.bounds.width().value > 0.0 => Frame {
                left: boundary.bounds.left().value,
                bottom: boundary.bounds.bottom().value,
                width: boundary.bounds.width().value,
                height: boundary.bounds.height().value,
                quarter_turns,
            },
            _ => {
                let (w, h) = (page.width().value, page.height().value);
                let (width, height) = if quarter_turns % 2 == 1 {
                    (h, w)
                } else {
                    (w, h)
                };
                Frame {
                    left: 0.0,
                    bottom: 0.0,
                    width,
                    height,
                    quarter_turns,
                }
            }
        }
    }

    fn point(&self, x: f32, y: f32) -> (f32, f32) {
        let ux = x - self.left;
        let uy = y - self.bottom;
        match self.quarter_turns {
            1 => (uy, ux),
            2 => (self.width - ux, uy),
            3 => (self.height - uy, self.width - ux),
            _ => (ux, self.height - uy),
        }
    }

    fn rect(&self, r: &PdfRect) -> Rect {
        let (x1, y1) = self.point(r.left().value, r.bottom().value);
        let (x2, y2) = self.point(r.right().value, r.top().value);
        Rect {
            x: x1.min(x2),
            y: y1.min(y2),
            w: (x2 - x1).abs(),
            h: (y2 - y1).abs(),
        }
    }
}

/// Reads the visible chars of a page and builds its lines.
pub(crate) fn page_text(page: &PdfPage<'_>) -> Result<PageText, PdfiumError> {
    let frame = Frame::of(page);
    let text = page.text()?;
    let mut glyphs = Vec::new();
    for ch in text.chars().iter() {
        let Some(c) = ch.unicode_char() else {
            continue;
        };
        // Spaces and line breaks are rebuilt from the geometry; PDFium's generated chars have no
        // meaningful box.
        if c.is_whitespace() || c.is_control() || ch.is_generated().unwrap_or(false) {
            continue;
        }
        let Ok(bounds) = ch.loose_bounds() else {
            continue;
        };
        let rect = frame.rect(&bounds);
        if !(rect.w.is_finite() && rect.h.is_finite()) || rect.h <= 0.0 {
            continue;
        }
        let baseline = match ch.origin() {
            Ok((x, y)) => frame.point(x.value, y.value).1,
            Err(_) => rect.y + rect.h,
        };
        glyphs.push(Glyph {
            ch: c,
            rect,
            baseline,
        });
    }
    Ok(PageText {
        width: page.width().value,
        height: page.height().value,
        lines: build_lines(glyphs),
    })
}

/// Groups glyphs into lines by baseline and builds segments and texts.
pub(crate) fn build_lines(mut glyphs: Vec<Glyph>) -> Vec<Line> {
    glyphs.sort_by(|a, b| {
        a.baseline
            .total_cmp(&b.baseline)
            .then(a.rect.x.total_cmp(&b.rect.x))
    });

    let mut groups: Vec<Vec<Glyph>> = Vec::new();
    let mut current: Vec<Glyph> = Vec::new();
    let mut current_baseline = 0.0f32;
    let mut current_height = 0.0f32;
    for glyph in glyphs {
        let tolerance = BASELINE_TOLERANCE * glyph.rect.h.max(current_height).max(1.0);
        if current.is_empty() || (glyph.baseline - current_baseline).abs() <= tolerance {
            let n = current.len() as f32;
            current_baseline = (current_baseline * n + glyph.baseline) / (n + 1.0);
            current_height = current_height.max(glyph.rect.h);
            current.push(glyph);
        } else {
            groups.push(std::mem::take(&mut current));
            current_baseline = glyph.baseline;
            current_height = glyph.rect.h;
            current.push(glyph);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let mut lines: Vec<Line> = groups.into_iter().filter_map(make_line).collect();
    lines.sort_by(|a, b| {
        a.rect
            .y
            .total_cmp(&b.rect.y)
            .then(a.rect.x.total_cmp(&b.rect.x))
    });
    lines
}

/// Builds one line from glyphs sharing a baseline.
fn make_line(mut glyphs: Vec<Glyph>) -> Option<Line> {
    glyphs.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));
    // Some generators draw the same text twice with a tiny offset to fake bold.
    glyphs.dedup_by(|next, prev| {
        next.ch == prev.ch && (next.rect.x - prev.rect.x).abs() < 0.3 * prev.rect.w.max(0.5)
    });
    if glyphs.is_empty() {
        return None;
    }

    let mut raw = String::new();
    let mut raw_owner = Vec::new();
    let mut norm = String::new();
    let mut norm_owner = Vec::new();
    let mut segments = Vec::new();
    let mut segment_start = (0usize, 0usize, 0usize);

    for (i, glyph) in glyphs.iter().enumerate() {
        if i > 0 {
            let prev = &glyphs[i - 1];
            let gap = glyph.rect.x - (prev.rect.x + prev.rect.w);
            let height = prev.rect.h.max(glyph.rect.h).max(1.0);
            if gap > SEGMENT_GAP * height {
                segments.push(make_segment(
                    &glyphs,
                    segment_start,
                    (i, raw.len(), norm.len()),
                ));
                push_space(&mut raw, &mut raw_owner, i);
                push_space(&mut norm, &mut norm_owner, i);
                segment_start = (i, raw.len(), norm.len());
            } else if gap > WORD_GAP * height {
                push_space(&mut raw, &mut raw_owner, i);
                push_space(&mut norm, &mut norm_owner, i);
            }
        }
        raw.push(glyph.ch);
        raw_owner.extend(std::iter::repeat_n(i, glyph.ch.len_utf8()));
        fold_char(glyph.ch, |folded| {
            norm.push(folded);
            norm_owner.extend(std::iter::repeat_n(i, folded.len_utf8()));
        });
    }
    segments.push(make_segment(
        &glyphs,
        segment_start,
        (glyphs.len(), raw.len(), norm.len()),
    ));

    let rect = union_all(glyphs.iter().map(|g| g.rect));
    Some(Line {
        glyphs,
        segments,
        raw,
        raw_owner,
        norm,
        norm_owner,
        rect,
    })
}

fn push_space(text: &mut String, owner: &mut Vec<usize>, glyph: usize) {
    if !text.is_empty() && !text.ends_with(' ') {
        text.push(' ');
        owner.push(glyph);
    }
}

fn make_segment(
    glyphs: &[Glyph],
    (glyph_start, raw_start, norm_start): (usize, usize, usize),
    (glyph_end, raw_end, norm_end): (usize, usize, usize),
) -> Segment {
    Segment {
        glyphs: glyph_start..glyph_end,
        raw: raw_start..raw_end,
        norm: norm_start..norm_end,
        rect: union_all(glyphs[glyph_start..glyph_end].iter().map(|g| g.rect)),
    }
}

fn span_rect(glyphs: &[Glyph], owner: &[usize], start: usize, end: usize) -> Rect {
    if start >= end || end > owner.len() {
        return Rect::default();
    }
    let (first, last) = (owner[start], owner[end - 1]);
    union_all(
        glyphs[first.min(last)..=first.max(last)]
            .iter()
            .map(|g| g.rect),
    )
}

/// Smallest rectangle containing all the given ones (default rect when empty).
pub(crate) fn union_all(rects: impl IntoIterator<Item = Rect>) -> Rect {
    let mut iter = rects.into_iter();
    let Some(first) = iter.next() else {
        return Rect::default();
    };
    let (mut x1, mut y1, mut x2, mut y2) = (first.x, first.y, first.x + first.w, first.y + first.h);
    for r in iter {
        x1 = x1.min(r.x);
        y1 = y1.min(r.y);
        x2 = x2.max(r.x + r.w);
        y2 = y2.max(r.y + r.h);
    }
    Rect {
        x: x1,
        y: y1,
        w: x2 - x1,
        h: y2 - y1,
    }
}

/// Geometry helpers on [`Rect`].
pub(crate) trait RectExt {
    fn right(&self) -> f32;
    fn bottom(&self) -> f32;
    fn center(&self) -> (f32, f32);
    fn area(&self) -> f32;
    fn contains_point(&self, x: f32, y: f32) -> bool;
    /// Overlap area with another rectangle.
    fn overlap_area(&self, other: &Rect) -> f32;
    /// Length of the horizontal overlap with another rectangle.
    fn overlap_x(&self, other: &Rect) -> f32;
    /// Length of the vertical overlap with another rectangle.
    fn overlap_y(&self, other: &Rect) -> f32;
}

impl RectExt for Rect {
    fn right(&self) -> f32 {
        self.x + self.w
    }

    fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn center(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    fn area(&self) -> f32 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.bottom()
    }

    fn overlap_area(&self, other: &Rect) -> f32 {
        self.overlap_x(other) * self.overlap_y(other)
    }

    fn overlap_x(&self, other: &Rect) -> f32 {
        (self.right().min(other.right()) - self.x.max(other.x)).max(0.0)
    }

    fn overlap_y(&self, other: &Rect) -> f32 {
        (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0.0)
    }
}

/// Glyphs for `text` starting at `x`, top `top`, with a fixed advance (tests).
#[cfg(test)]
pub(crate) fn word(text: &str, x: f32, top: f32, size: f32) -> Vec<Glyph> {
    let advance = size * 0.5;
    text.chars()
        .enumerate()
        .filter(|(_, c)| !c.is_whitespace())
        .map(|(i, ch)| Glyph {
            ch,
            rect: Rect {
                x: x + i as f32 * advance,
                y: top,
                w: advance,
                h: size,
            },
            baseline: top + size * 0.8,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_and_segments() {
        let mut glyphs = word("Total das Retenções", 12.0, 510.0, 6.0);
        glyphs.extend(word("VALOR LÍQUIDO DA NFS-e", 156.5, 510.0, 6.0));
        glyphs.extend(word("R$ 1.234,56", 156.5, 517.0, 8.0));
        let lines = build_lines(glyphs);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].segments.len(), 2);
        assert_eq!(lines[0].raw, "Total das Retenções VALOR LÍQUIDO DA NFS-e");
        assert_eq!(lines[0].norm, "total das retencoes valor liquido da nfs-e");
        assert_eq!(lines[1].raw, "R$ 1.234,56");

        // A normalized match maps back to the glyph boxes of the second column.
        let line = &lines[0];
        let start = line.norm.find("valor").unwrap();
        let rect = line.norm_span_rect(start, line.norm.len());
        assert!((rect.x - 156.5).abs() < 0.01);
    }

    #[test]
    fn keeps_glued_words_together() {
        let lines = build_lines(word("ValorLíquidodaNFS-e", 439.4, 605.2, 7.0));
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].norm, "valorliquidodanfs-e");
        assert_eq!(lines[0].segments.len(), 1);
    }
}
