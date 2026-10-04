//! Extraction rules: anchor labels, regions (optionally anchored), the automatic heuristic and the
//! order in which profiles are tried.

use crate::engine::{page_count, Engine};
use crate::error::CoreError;
use crate::layout::{build_lines, page_text, union_all, Line, PageText, RectExt};
use crate::model::{
    DocKind, DocResult, FieldDef, FieldKind, FieldValue, Origin, Profile, Rect, RegionAnchor,
    RegionRead, Rule, RuleTest, Status,
};
use crate::parse::{find_dates, find_money};
use crate::profile::{
    profile_matches, validate_rule, COMPETENCE_FIELD, NET_VALUE_FIELD, SERVICE_VALUE_FIELD,
};
use crate::regex_cache::compiled;
use crate::text::compact;
use pdfium_render::prelude::PdfPageIndex;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Pages searched per document.
const MAX_PAGES: u32 = 5;

/// Label patterns of the automatic heuristic (normalized text), per built-in field, in priority
/// order, each with an optional exclude suffix.
const AUTO_NET_VALUE_LABELS: &[(&str, Option<&str>)] = &[
    (r"valor\s*liquido\s*da\s*nfs-?e", Some(r"^\s*\+")),
    (r"valor\s*liquido\s*da\s*nota", None),
    (r"valor\s*liquido", Some(r"^\s*\+")),
    (r"valor\s*total\s*da\s*nfs-?e", None),
    (r"valor\s*total\s*da\s*nota", None),
    (r"valor\s*total\s*dos\s*servicos", None),
    (r"valor\s*dos\s*servicos", None),
    (r"valor\s*do\s*servico", None),
    (r"valor\s*total", None),
];
const AUTO_SERVICE_VALUE_LABELS: &[(&str, Option<&str>)] = &[
    (r"valor\s*total\s*dos\s*servicos", None),
    (r"valor\s*dos\s*servicos", None),
    (r"valor\s*do\s*servico", None),
    (r"valor\s*da\s*operacao", None),
    (r"valor\s*bruto", None),
];
const AUTO_COMPETENCE_LABELS: &[(&str, Option<&str>)] = &[
    (r"competencia", None),
    (r"data\s*(?:e\s*hora\s*)?d[ae]\s*emissao", None),
    (r"emissao", None),
    (r"emitida\s*em", None),
];
const AUTO_RIGHT_DISTANCE: f32 = 250.0;
const AUTO_BELOW_DISTANCE: f32 = 30.0;

/// How far above a region an anchor may be, and how far to the left.
const ANCHOR_ABOVE_DISTANCE: f32 = 40.0;
const ANCHOR_LEFT_DISTANCE: f32 = 250.0;
/// Horizontal distances count less than vertical ones when choosing an anchor.
const ANCHOR_LEFT_WEIGHT: f32 = 0.25;
const ANCHOR_MAX_CHARS: usize = 60;
/// Padding added around a value found in a drawn region before turning it into a rule.
const RULE_PADDING: f32 = 2.0;

const MSG_NO_TEXT: &str =
    "O PDF não tem texto selecionável (provavelmente é uma imagem escaneada).";
const MSG_NOT_FOUND: &str =
    "Valor não encontrado. Marque o valor na página para ensinar onde ele fica.";

/// A candidate value found on a page.
#[derive(Debug, Clone)]
struct Token {
    rect: Rect,
    raw: String,
    cents: Option<i64>,
    date: Option<String>,
    text: Option<String>,
}

impl Token {
    /// A text value: what was found is also the value.
    fn text(rect: Rect, text: String) -> Token {
        Token {
            rect,
            raw: text.clone(),
            cents: None,
            date: None,
            text: Some(text),
        }
    }

    fn into_value(self, origin: Origin, page: u32) -> FieldValue {
        FieldValue {
            raw: self.raw,
            cents: self.cents,
            date: self.date,
            text: self.text,
            origin,
            page,
            bbox: self.rect,
        }
    }
}

/// Which pages to read.
enum Pages {
    /// The first `n` pages.
    First(u32),
    /// A single 0-based page.
    Only(u32),
}

struct LoadedDoc {
    /// (0-based page index, text) of the pages read.
    pages: Vec<(u32, PageText)>,
    page_count: u32,
}

fn load(engine: &Engine, path: &Path, which: Pages) -> Result<LoadedDoc, CoreError> {
    let document = engine.open(path)?;
    let pages = document.pages();
    let count = page_count(pages);
    let range = match which {
        Pages::First(n) => 0..count.min(n),
        Pages::Only(page) => page..(page + 1).min(count),
    };
    let mut texts = Vec::with_capacity(range.len());
    for index in range {
        let page = pages.get(index as PdfPageIndex)?;
        // A page whose text cannot be read counts as a page without text.
        texts.push((index, page_text(&page).unwrap_or_default()));
    }
    Ok(LoadedDoc {
        pages: texts,
        page_count: count,
    })
}

/// See [`Engine::extract`].
pub(crate) fn extract_document(
    engine: &Engine,
    path: &Path,
    profiles: &[Profile],
    fields: &[FieldDef],
) -> DocResult {
    let mut result = DocResult {
        path: path.to_string_lossy().into_owned(),
        status: Status::Error,
        message: None,
        doc_type: "PDF".to_string(),
        kind: DocKind::Revenue,
        fields: BTreeMap::new(),
        page_count: 0,
    };
    let doc = match load(engine, path, Pages::First(MAX_PAGES)) {
        Ok(doc) => doc,
        Err(err) => {
            result.message = Some(err.to_string());
            return result;
        }
    };
    result.page_count = doc.page_count;

    let all_text: String = doc.pages.iter().map(|(_, page)| page.raw_text()).collect();
    let compact_text = compact(&all_text);
    let file_name = crate::scan::file_name(path);
    let builtin: Vec<&Profile> = profiles
        .iter()
        .filter(|p| p.builtin && profile_matches(p, &compact_text, &file_name))
        .collect();
    let user: Vec<&Profile> = profiles
        .iter()
        .filter(|p| !p.builtin && profile_matches(p, &compact_text, &file_name))
        .collect();
    // Classification: the user's profiles first (so they can say e.g. "the DANFSe issued by my
    // accountant is an expense"), then the built-in ones.
    let classifiers = || user.iter().chain(builtin.iter());
    if let Some(doc_type) = classifiers().find_map(|p| p.doc_type.clone()) {
        result.doc_type = doc_type;
    } else if !builtin.is_empty() {
        result.doc_type = "NFS-e".to_string();
    }
    if let Some(kind) = classifiers().find_map(|p| p.kind) {
        result.kind = kind;
    }

    if !doc.pages.iter().any(|(_, page)| page.has_text()) {
        result.status = Status::NoText;
        result.message = Some(MSG_NO_TEXT.to_string());
        return result;
    }

    for field in fields {
        let from_profiles = builtin.iter().chain(user.iter()).find_map(|profile| {
            let rules = profile.fields.get(&field.id)?;
            rules.iter().find_map(|rule| {
                apply_rule(&doc.pages, rule, field.kind).map(|(page, token)| {
                    let origin = Origin::Profile {
                        id: profile.id.clone(),
                        name: profile.name.clone(),
                    };
                    token.into_value(origin, page)
                })
            })
        });
        let found = from_profiles.or_else(|| {
            let labels = auto_labels(&field.id)?;
            doc.pages.iter().find_map(|(index, page)| {
                apply_auto(page, field.kind, labels)
                    .map(|token| token.into_value(Origin::Auto, *index))
            })
        });
        if let Some(value) = found {
            result.fields.insert(field.id.clone(), value);
        }
    }

    // Only required fields decide the status; optional ones are a bonus.
    let missing_required = fields
        .iter()
        .any(|f| f.required && !result.fields.contains_key(&f.id));
    if missing_required {
        result.status = Status::NotFound;
        result.message = Some(MSG_NOT_FOUND.to_string());
    } else {
        result.status = Status::Ok;
    }
    result
}

/// See [`Engine::read_region`].
pub(crate) fn read_region(
    engine: &Engine,
    path: &Path,
    page: u32,
    rect: Rect,
    kind: FieldKind,
) -> Result<RegionRead, CoreError> {
    let doc = load(engine, path, Pages::Only(page))?;
    let Some((_, text)) = doc.pages.into_iter().next() else {
        return Err(CoreError::PageOutOfRange {
            page,
            count: doc.page_count,
        });
    };
    let rect = positive(rect);
    let (inside, _) = inside_text(&text, &rect);
    let token = read_rect(&text, &rect, kind);

    // The rule targets the value itself (padded) when one was found, so a loosely drawn
    // rectangle does not catch neighbouring values on other files.
    let target = token
        .as_ref()
        .map(|t| pad(&t.rect, RULE_PADDING))
        .unwrap_or(rect);
    let (w, h) = (text.width.max(1.0), text.height.max(1.0));
    let suggested_rule = Rule::Region {
        rect: Rect {
            x: target.x / w,
            y: target.y / h,
            w: target.w / w,
            h: target.h / h,
        },
        page,
        anchor: derive_anchor(&text, &target),
    };

    Ok(RegionRead {
        text: inside,
        value: token.map(|t| t.into_value(Origin::Region, page)),
        suggested_rule,
    })
}

/// See [`Engine::test_rule`].
pub(crate) fn test_rule(engine: &Engine, path: &Path, field: &FieldDef, rule: &Rule) -> RuleTest {
    let mut result = RuleTest {
        path: path.to_string_lossy().into_owned(),
        value: None,
        error: None,
    };
    if let Err(err) = validate_rule(rule) {
        result.error = Some(err.to_string());
        return result;
    }
    let which = match rule {
        Rule::Region { page, .. } => Pages::Only(*page),
        Rule::Anchor { .. } => Pages::First(MAX_PAGES),
    };
    match load(engine, path, which) {
        Ok(doc) => {
            result.value = apply_rule(&doc.pages, rule, field.kind)
                .map(|(page, token)| token.into_value(Origin::Region, page));
        }
        Err(err) => result.error = Some(err.to_string()),
    }
    result
}

/// Applies one rule to the loaded pages; returns the page index and the value found.
fn apply_rule(pages: &[(u32, PageText)], rule: &Rule, kind: FieldKind) -> Option<(u32, Token)> {
    match rule {
        Rule::Anchor {
            pattern,
            exclude_suffix,
            direction,
            max_distance,
        } => {
            let pattern = compiled(pattern).ok()?;
            let exclude = match exclude_suffix {
                Some(suffix) => Some(compiled(suffix).ok()?),
                None => None,
            };
            pages.iter().find_map(|(index, page)| {
                apply_anchor(
                    page,
                    kind,
                    &pattern,
                    exclude.as_deref(),
                    *direction,
                    *max_distance,
                )
                .map(|token| (*index, token))
            })
        }
        Rule::Region { rect, page, anchor } => {
            let (index, text) = pages.iter().find(|(index, _)| index == page)?;
            apply_region(text, rect, anchor.as_ref(), kind).map(|token| (*index, token))
        }
    }
}

/// Finds every occurrence of the label and returns the nearest value in `direction`.
fn apply_anchor(
    page: &PageText,
    kind: FieldKind,
    pattern: &Regex,
    exclude: Option<&Regex>,
    direction: crate::model::Direction,
    max_distance: f32,
) -> Option<Token> {
    let tokens = tokens(page, kind);
    nearest_to_label(page, pattern, exclude, |line, label, end| match direction {
        crate::model::Direction::Below => pick_below(label, &tokens, max_distance),
        crate::model::Direction::Right if kind == FieldKind::Text => {
            text_after(line, end, label, max_distance)
        }
        crate::model::Direction::Right => pick_right(label, &tokens, max_distance),
    })
}

/// The nearest value over every occurrence of the label on the page: `pick` turns one occurrence
/// (its line, its box and where it ends in the normalized text) into a value and its distance.
/// The first of equally near values wins.
fn nearest_to_label(
    page: &PageText,
    pattern: &Regex,
    exclude: Option<&Regex>,
    mut pick: impl FnMut(&Line, &Rect, usize) -> Option<(Token, f32)>,
) -> Option<Token> {
    page.lines
        .iter()
        .flat_map(|line| {
            label_matches(line, pattern, exclude)
                .into_iter()
                .map(move |(label, end)| (line, label, end))
        })
        .filter_map(|(line, label, end)| pick(line, &label, end))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(token, _)| token)
}

/// Compiled heuristic labels of a built-in field, `None` for fields without a heuristic.
fn auto_labels(field_id: &str) -> Option<&'static [(Regex, Option<Regex>)]> {
    type Labels = Vec<(Regex, Option<Regex>)>;
    fn compile(table: &[(&str, Option<&str>)]) -> Labels {
        table
            .iter()
            .map(|(pattern, suffix)| {
                (
                    Regex::new(pattern).expect("valid auto label"),
                    suffix.map(|s| Regex::new(s).expect("valid auto suffix")),
                )
            })
            .collect()
    }
    static NET_VALUE: OnceLock<Labels> = OnceLock::new();
    static SERVICE_VALUE: OnceLock<Labels> = OnceLock::new();
    static COMPETENCE: OnceLock<Labels> = OnceLock::new();
    let labels = match field_id {
        NET_VALUE_FIELD => NET_VALUE.get_or_init(|| compile(AUTO_NET_VALUE_LABELS)),
        SERVICE_VALUE_FIELD => SERVICE_VALUE.get_or_init(|| compile(AUTO_SERVICE_VALUE_LABELS)),
        COMPETENCE_FIELD => COMPETENCE.get_or_init(|| compile(AUTO_COMPETENCE_LABELS)),
        _ => return None,
    };
    Some(labels.as_slice())
}

/// The automatic heuristic: known labels in priority order, value of `kind` to the right on the
/// same line, else below.
fn apply_auto(
    page: &PageText,
    kind: FieldKind,
    labels: &[(Regex, Option<Regex>)],
) -> Option<Token> {
    let tokens = tokens(page, kind);
    if tokens.is_empty() {
        return None;
    }
    labels.iter().find_map(|(pattern, exclude)| {
        nearest_to_label(page, pattern, exclude.as_ref(), |_, label, _| {
            pick_right(label, &tokens, AUTO_RIGHT_DISTANCE)
                .or_else(|| pick_below(label, &tokens, AUTO_BELOW_DISTANCE))
        })
    })
}

/// Reads a region rule: at the anchor when it is found (nearest occurrence to the original
/// position first), otherwise at the relative rectangle.
fn apply_region(
    page: &PageText,
    relative: &Rect,
    anchor: Option<&RegionAnchor>,
    kind: FieldKind,
) -> Option<Token> {
    let absolute = Rect {
        x: relative.x * page.width,
        y: relative.y * page.height,
        w: relative.w * page.width,
        h: relative.h * page.height,
    };
    if let Some(anchor) = anchor {
        if let Some(pattern) = anchor_regex(&anchor.text) {
            let mut candidates: Vec<Rect> = page
                .lines
                .iter()
                .flat_map(|line| label_matches(line, &pattern, None))
                .map(|(found, _)| Rect {
                    x: found.x + anchor.dx,
                    y: found.y + anchor.dy,
                    w: anchor.w,
                    h: anchor.h,
                })
                .collect();
            candidates.sort_by(|a, b| {
                distance(a.x, a.y, absolute.x, absolute.y)
                    .total_cmp(&distance(b.x, b.y, absolute.x, absolute.y))
            });
            if let Some(token) = candidates
                .iter()
                .find_map(|rect| read_rect(page, rect, kind))
            {
                return Some(token);
            }
        }
    }
    read_rect(page, &absolute, kind)
}

/// Label occurrences on a line: (box of the label, byte end of the match in `line.norm`).
fn label_matches(line: &Line, pattern: &Regex, exclude: Option<&Regex>) -> Vec<(Rect, usize)> {
    pattern
        .find_iter(&line.norm)
        .filter(|m| m.start() < m.end())
        .filter(|m| exclude.is_none_or(|ex| !ex.is_match(&line.norm[m.end()..])))
        .map(|m| (line.norm_span_rect(m.start(), m.end()), m.end()))
        .collect()
}

/// Candidate values of a kind on a page.
fn tokens(page: &PageText, kind: FieldKind) -> Vec<Token> {
    let mut out = Vec::new();
    for line in &page.lines {
        match kind {
            FieldKind::Money => {
                for m in find_money(&line.raw) {
                    out.push(Token {
                        rect: line.raw_span_rect(m.start, m.end),
                        raw: line.raw[m.start..m.end].trim().to_string(),
                        cents: Some(m.cents),
                        date: None,
                        text: None,
                    });
                }
            }
            FieldKind::Date => {
                for d in find_dates(&line.raw) {
                    out.push(Token {
                        rect: line.raw_span_rect(d.start, d.end),
                        raw: line.raw[d.start..d.end].to_string(),
                        cents: None,
                        date: Some(d.iso),
                        text: None,
                    });
                }
            }
            FieldKind::Text => {
                for segment in &line.segments {
                    let raw = line.raw[segment.raw.clone()].trim();
                    if !raw.is_empty() {
                        out.push(Token::text(segment.rect, raw.to_string()));
                    }
                }
            }
        }
    }
    out
}

/// Nearest token below the label, aligned with it; distance = vertical gap.
fn pick_below(label: &Rect, tokens: &[Token], max_distance: f32) -> Option<(Token, f32)> {
    tokens
        .iter()
        .filter(|t| t.rect.y >= label.bottom() - 2.0 && t.rect.y <= label.bottom() + max_distance)
        .filter(|t| t.rect.overlap_x(label) > 0.0 || (t.rect.x - label.x).abs() <= 8.0)
        .map(|t| {
            let vertical = (t.rect.y - label.bottom()).max(0.0);
            // Tokens of the same row: prefer the one aligned with the label.
            (t, vertical + 0.01 * (t.rect.x - label.x).abs())
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, d)| (t.clone(), d))
}

/// Nearest token to the right of the label on the same line; distance = horizontal gap.
fn pick_right(label: &Rect, tokens: &[Token], max_distance: f32) -> Option<(Token, f32)> {
    tokens
        .iter()
        .filter(|t| t.rect.overlap_y(label) >= 0.5 * t.rect.h.min(label.h))
        .filter(|t| t.rect.x >= label.right() - 1.0 && t.rect.x - label.right() <= max_distance)
        .map(|t| (t, (t.rect.x - label.right()).max(0.0)))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(t, d)| (t.clone(), d))
}

/// Text fields: the rest of the label's segment after the label, else the next segment.
fn text_after(
    line: &Line,
    norm_end: usize,
    label: &Rect,
    max_distance: f32,
) -> Option<(Token, f32)> {
    let last_glyph = *line.norm_owner.get(norm_end.checked_sub(1)?)?;
    let start = line
        .raw_owner
        .iter()
        .position(|&owner| owner > last_glyph)?;
    let segment_end = line
        .segments
        .iter()
        .find(|s| s.raw.contains(&start))
        .map(|s| s.raw.end)
        .unwrap_or(line.raw.len());
    let trim = |s: &str| {
        s.trim_matches(|c: char| c.is_whitespace() || c == ':' || c == '-')
            .to_string()
    };

    let rest = trim(&line.raw[start..segment_end]);
    let (text, rect) = if rest.is_empty() {
        let next = line.segments.iter().find(|s| s.raw.start >= segment_end)?;
        (trim(&line.raw[next.raw.clone()]), next.rect)
    } else {
        (rest, line.raw_span_rect(start, segment_end))
    };
    let gap = (rect.x - label.right()).max(0.0);
    (!text.is_empty() && gap <= max_distance).then(|| (Token::text(rect, text), gap))
}

/// The value of `kind` best covered by `rect`: tokens whose center is inside, or that overlap it
/// by at least 30% of their own area (so values that grew a digit are still read).
fn read_rect(page: &PageText, rect: &Rect, kind: FieldKind) -> Option<Token> {
    if kind == FieldKind::Text {
        let (text, bbox) = inside_text(page, rect);
        return (!text.is_empty()).then(|| Token::text(bbox, text));
    }
    let (rcx, rcy) = rect.center();
    tokens(page, kind)
        .into_iter()
        .filter_map(|token| {
            let overlap = token.rect.overlap_area(rect);
            if overlap <= 0.0 {
                return None;
            }
            let ratio = overlap / token.rect.area().max(1e-3);
            let (cx, cy) = token.rect.center();
            let inside = rect.contains_point(cx, cy);
            (inside || ratio >= 0.3).then(|| {
                let d = distance(cx, cy, rcx, rcy);
                (token, ratio.min(1.0), d)
            })
        })
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.2.total_cmp(&a.2)))
        .map(|(token, _, _)| token)
}

/// Text of the glyphs whose center lies inside `rect` (lines joined by a space), and its box.
fn inside_text(page: &PageText, rect: &Rect) -> (String, Rect) {
    let glyphs: Vec<_> = page
        .lines
        .iter()
        .flat_map(|line| line.glyphs.iter())
        .filter(|g| {
            let (cx, cy) = g.rect.center();
            rect.contains_point(cx, cy)
        })
        .copied()
        .collect();
    let bbox = union_all(glyphs.iter().map(|g| g.rect));
    let lines = build_lines(glyphs);
    let text = lines
        .iter()
        .map(|line| line.raw.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    (text, bbox)
}

/// Chooses the text that best anchors `rect`: the nearest text segment above it, or the text
/// just to its left on the same line.
fn derive_anchor(page: &PageText, rect: &Rect) -> Option<RegionAnchor> {
    let mut best: Option<(f32, Rect, String)> = None;
    let mut consider = |score: f32, found: Rect, text: String| {
        if best.as_ref().is_none_or(|(s, _, _)| score < *s) {
            best = Some((score, found, text));
        }
    };

    let widened = Rect {
        x: rect.x - 20.0,
        w: rect.w + 40.0,
        ..*rect
    };
    for line in &page.lines {
        for segment in &line.segments {
            // Loose glyph boxes and the padding of the rule rectangle make a label right above
            // the value overlap it slightly: accept that, as long as the label sits mostly above.
            let gap = rect.y - segment.rect.bottom();
            let (_, center_y) = segment.rect.center();
            if center_y >= rect.y
                || gap < -0.5 * segment.rect.h
                || gap > ANCHOR_ABOVE_DISTANCE
                || segment.rect.overlap_x(&widened) <= 0.0
            {
                continue;
            }
            if let Some(text) = anchor_text(&line.norm[segment.norm.clone()]) {
                consider(gap.max(0.0), segment.rect, text);
            }
        }
    }

    for line in &page.lines {
        if line.rect.overlap_y(rect) < 0.5 * line.rect.h.min(rect.h) {
            continue;
        }
        let Some(last) = line
            .glyphs
            .iter()
            .rposition(|g| g.rect.right() <= rect.x + 1.0)
        else {
            continue;
        };
        let Some(segment) = line.segments.iter().find(|s| s.glyphs.contains(&last)) else {
            continue;
        };
        let first = segment.glyphs.start;
        let run = union_all(line.glyphs[first..=last].iter().map(|g| g.rect));
        let gap = rect.x - run.right();
        if gap > ANCHOR_LEFT_DISTANCE {
            continue;
        }
        let start = line.norm_owner.iter().position(|&o| o >= first);
        let end = line
            .norm_owner
            .iter()
            .rposition(|&o| o <= last)
            .map(|e| e + 1);
        if let (Some(start), Some(end)) = (start, end) {
            if let Some(text) = anchor_text(&line.norm[start..end]) {
                consider(gap.max(0.0) * ANCHOR_LEFT_WEIGHT, run, text);
            }
        }
    }

    best.map(|(_, found, text)| RegionAnchor {
        text,
        dx: rect.x - found.x,
        dy: rect.y - found.y,
        w: rect.w,
        h: rect.h,
    })
}

/// Normalized text usable as an anchor: at least two letters, cut at a word boundary.
fn anchor_text(norm: &str) -> Option<String> {
    let text = norm.trim();
    if text.chars().filter(|c| c.is_alphabetic()).count() < 2 {
        return None;
    }
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() && out.chars().count() + 1 + word.chars().count() > ANCHOR_MAX_CHARS {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    Some(out)
}

/// Literal search for an anchor text that also matches glued or differently spaced words.
fn anchor_regex(text: &str) -> Option<Arc<Regex>> {
    let words: Vec<String> = text.split_whitespace().map(regex::escape).collect();
    if words.is_empty() {
        return None;
    }
    compiled(&words.join(r"\s*")).ok()
}

fn positive(rect: Rect) -> Rect {
    let (x1, x2) = (rect.x.min(rect.x + rect.w), rect.x.max(rect.x + rect.w));
    let (y1, y2) = (rect.y.min(rect.y + rect.h), rect.y.max(rect.y + rect.h));
    Rect {
        x: x1,
        y: y1,
        w: x2 - x1,
        h: y2 - y1,
    }
}

fn pad(rect: &Rect, by: f32) -> Rect {
    Rect {
        x: rect.x - by,
        y: rect.y - by,
        w: rect.w + 2.0 * by,
        h: rect.h + 2.0 * by,
    }
}

fn distance(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Direction;

    use crate::layout::word;

    fn page(words: &[(&str, f32, f32)]) -> PageText {
        let glyphs = words
            .iter()
            .flat_map(|(text, x, top)| word(text, *x, *top, 6.0))
            .collect();
        PageText {
            width: 595.0,
            height: 842.0,
            lines: build_lines(glyphs),
        }
    }

    fn danfse_rule(direction: Direction, max_distance: f32) -> Rule {
        Rule::Anchor {
            pattern: r"valor\s*liquido\s*da\s*nfs-?e".into(),
            exclude_suffix: Some(r"^\s*\+".into()),
            direction,
            max_distance,
        }
    }

    #[test]
    fn danfse_v2_traps_are_avoided() {
        let page = page(&[
            ("VALOR DA OPERAÇÃO", 156.5, 491.0),
            ("R$ 1.300,00", 156.5, 498.0),
            ("Total das Retenções", 11.9, 510.0),
            ("VALOR LÍQUIDO DA NFS-e", 156.5, 510.0),
            ("Total do IBS/CBS", 301.0, 510.0),
            ("VALOR LÍQUIDO DA NFS-e + IBS/CBS", 445.6, 510.0),
            ("R$ 65,44", 11.9, 517.0),
            ("R$ 1.234,56", 156.5, 517.0),
            ("R$ 12,34", 301.0, 517.0),
            ("R$ 1.246,90", 445.6, 517.0),
        ]);
        let pages = vec![(0, page)];
        let (_, token) = apply_rule(
            &pages,
            &danfse_rule(Direction::Below, 24.0),
            FieldKind::Money,
        )
        .expect("value below the label");
        assert_eq!(token.cents, Some(123_456));
        assert!(apply_rule(
            &pages,
            &danfse_rule(Direction::Right, 250.0),
            FieldKind::Money
        )
        .is_none());
        let labels = auto_labels(NET_VALUE_FIELD).unwrap();
        assert_eq!(
            apply_auto(&pages[0].1, FieldKind::Money, labels)
                .unwrap()
                .cents,
            Some(123_456)
        );
    }

    #[test]
    fn auto_reads_value_to_the_right() {
        let page = page(&[
            ("Valor Total da Nota:", 40.0, 700.0),
            ("R$ 2.500,00", 110.0, 700.0),
            ("R$ 9,99", 110.0, 740.0),
        ]);
        let labels = auto_labels(NET_VALUE_FIELD).unwrap();
        assert_eq!(
            apply_auto(&page, FieldKind::Money, labels).unwrap().cents,
            Some(250_000)
        );
    }

    #[test]
    fn region_follows_its_anchor() {
        let original = page(&[
            ("Montante a pagar", 300.0, 400.0),
            ("R$ 845,10", 300.0, 408.0),
        ]);
        let shifted = page(&[
            ("Montante a pagar", 300.0, 440.0),
            ("R$ 912,45", 300.0, 448.0),
        ]);

        let value_rect = tokens(&original, FieldKind::Money)[0].rect;
        let target = pad(&value_rect, RULE_PADDING);
        let anchor = derive_anchor(&original, &target).expect("anchor above");
        assert_eq!(anchor.text, "montante a pagar");
        let relative = Rect {
            x: target.x / 595.0,
            y: target.y / 842.0,
            w: target.w / 595.0,
            h: target.h / 842.0,
        };
        let read = |page: &PageText, anchor: Option<&RegionAnchor>| {
            apply_region(page, &relative, anchor, FieldKind::Money).and_then(|t| t.cents)
        };
        assert_eq!(read(&original, Some(&anchor)), Some(84_510));
        assert_eq!(read(&shifted, Some(&anchor)), Some(91_245));
        assert_eq!(read(&shifted, None), None);
    }

    #[test]
    fn anchor_to_the_left_is_used_on_forms() {
        let page = page(&[
            ("Montante a pagar:", 40.0, 300.0),
            ("R$ 845,10", 100.0, 300.0),
        ]);
        let value_rect = tokens(&page, FieldKind::Money)[0].rect;
        let anchor = derive_anchor(&page, &value_rect).expect("anchor to the left");
        assert_eq!(anchor.text, "montante a pagar:");
        assert!(anchor.dx > 0.0 && anchor.dy.abs() < 1.0);
    }

    #[test]
    fn grown_values_are_still_read() {
        let page = page(&[("R$ 10.845,10", 300.0, 400.0)]);
        // A rectangle drawn around a shorter "R$ 845,10" at the same place.
        let rect = Rect {
            x: 298.0,
            y: 398.0,
            w: 30.0,
            h: 10.0,
        };
        assert_eq!(
            read_rect(&page, &rect, FieldKind::Money).unwrap().cents,
            Some(1_084_510)
        );
    }

    #[test]
    fn text_fields_read_after_the_label() {
        let page = page(&[("Tomador: EMPRESA EXEMPLO LTDA", 40.0, 300.0)]);
        let token = apply_anchor(
            &page,
            FieldKind::Text,
            &Regex::new("tomador").unwrap(),
            None,
            Direction::Right,
            250.0,
        )
        .unwrap();
        assert_eq!(token.text.as_deref(), Some("EMPRESA EXEMPLO LTDA"));
    }

    #[test]
    fn anchor_text_is_limited() {
        assert_eq!(anchor_text("r$ 1.234,56"), None);
        assert_eq!(anchor_text("  valor total "), Some("valor total".into()));
        let long = "palavra ".repeat(20);
        assert!(anchor_text(&long).unwrap().chars().count() <= ANCHOR_MAX_CHARS);
    }
}
