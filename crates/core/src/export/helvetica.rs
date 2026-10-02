//! Metrics of the standard Helvetica fonts used by the PDF report (`PdfFonts::helvetica()` and
//! `helvetica_bold()`), which PDFium writes with WinAnsiEncoding and without embedding.
//!
//! PDFium's glyph API finds no glyphs in these fonts, so the advance widths come from the Adobe
//! font metrics (AFM) of the standard 14 fonts, which PDF viewers use for them. A unit test checks
//! them against PDFium: they all match except "±", "÷", "µ", "·" and "¯", which PDFium's built-in
//! Helvetica replacement draws a little wider or narrower (the AFM value is kept for those).

use unicode_normalization::char::decompose_compatible;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Font {
    Regular,
    Bold,
}

/// Unicode chars of the WinAnsiEncoding codes 0x80..=0x9F; the other codes of the encoding are
/// the Unicode chars with the same value (0x20..=0x7E and 0xA0..=0xFF).
const WINANSI_HIGH: [(char, u8); 27] = [
    ('\u{20AC}', 0x80), // €
    ('\u{201A}', 0x82), // ‚
    ('\u{0192}', 0x83), // ƒ
    ('\u{201E}', 0x84), // „
    ('\u{2026}', 0x85), // …
    ('\u{2020}', 0x86), // †
    ('\u{2021}', 0x87), // ‡
    ('\u{02C6}', 0x88), // ˆ
    ('\u{2030}', 0x89), // ‰
    ('\u{0160}', 0x8A), // Š
    ('\u{2039}', 0x8B), // ‹
    ('\u{0152}', 0x8C), // Œ
    ('\u{017D}', 0x8E), // Ž
    ('\u{2018}', 0x91), // ‘
    ('\u{2019}', 0x92), // ’
    ('\u{201C}', 0x93), // “
    ('\u{201D}', 0x94), // ”
    ('\u{2022}', 0x95), // •
    ('\u{2013}', 0x96), // –
    ('\u{2014}', 0x97), // —
    ('\u{02DC}', 0x98), // ˜
    ('\u{2122}', 0x99), // ™
    ('\u{0161}', 0x9A), // š
    ('\u{203A}', 0x9B), // ›
    ('\u{0153}', 0x9C), // œ
    ('\u{017E}', 0x9E), // ž
    ('\u{0178}', 0x9F), // Ÿ
];

/// Advance widths (1/1000 em) of Helvetica for the WinAnsiEncoding codes 0x20..=0xFF; 0 marks
/// codes without a char.
#[rustfmt::skip]
const REGULAR: [u16; 224] = [
    // 0x20
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278,
    // 0x30
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556,
    // 0x40
    1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778,
    // 0x50
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556,
    // 0x60
    333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, 556, 556,
    // 0x70
    556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584, 0,
    // 0x80
    556, 0, 222, 556, 333, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    // 0x90
    0, 222, 222, 333, 333, 350, 556, 1000, 333, 1000, 500, 333, 944, 0, 500, 667,
    // 0xA0
    278, 333, 556, 556, 556, 556, 260, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    // 0xB0
    400, 584, 333, 333, 333, 556, 537, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    // 0xC0
    667, 667, 667, 667, 667, 667, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    // 0xD0
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    // 0xE0
    556, 556, 556, 556, 556, 556, 889, 500, 556, 556, 556, 556, 278, 278, 278, 278,
    // 0xF0
    556, 556, 556, 556, 556, 556, 556, 584, 611, 556, 556, 556, 556, 500, 556, 500,
];

/// Advance widths (1/1000 em) of Helvetica-Bold for the WinAnsiEncoding codes 0x20..=0xFF.
#[rustfmt::skip]
const BOLD: [u16; 224] = [
    // 0x20
    278, 333, 474, 556, 556, 889, 722, 238, 333, 333, 389, 584, 278, 333, 278, 278,
    // 0x30
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, 333, 333, 584, 584, 584, 611,
    // 0x40
    975, 722, 722, 722, 722, 667, 611, 778, 722, 278, 556, 722, 611, 833, 722, 778,
    // 0x50
    667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, 333, 278, 333, 584, 556,
    // 0x60
    333, 556, 611, 556, 611, 556, 333, 611, 611, 278, 278, 556, 278, 889, 611, 611,
    // 0x70
    611, 611, 389, 556, 333, 611, 556, 778, 556, 556, 500, 389, 280, 389, 584, 0,
    // 0x80
    556, 0, 278, 556, 500, 1000, 556, 556, 333, 1000, 667, 333, 1000, 0, 611, 0,
    // 0x90
    0, 278, 278, 500, 500, 350, 556, 1000, 333, 1000, 556, 333, 944, 0, 500, 667,
    // 0xA0
    278, 333, 556, 556, 556, 556, 280, 556, 333, 737, 370, 556, 584, 333, 737, 333,
    // 0xB0
    400, 584, 333, 333, 333, 611, 556, 278, 333, 333, 365, 556, 834, 834, 834, 611,
    // 0xC0
    722, 722, 722, 722, 722, 722, 1000, 722, 667, 667, 667, 667, 278, 278, 278, 278,
    // 0xD0
    722, 722, 778, 778, 778, 778, 778, 584, 778, 722, 722, 722, 722, 667, 667, 611,
    // 0xE0
    556, 556, 556, 556, 556, 556, 889, 556, 556, 556, 556, 556, 278, 278, 278, 278,
    // 0xF0
    611, 611, 611, 611, 611, 611, 611, 584, 611, 611, 611, 611, 611, 556, 611, 556,
];

/// Width used for a char the tables do not know (never happens after [`sanitize`]).
const FALLBACK_WIDTH: u16 = 556;

/// WinAnsiEncoding code of `c`, if PDFium can write it with the standard fonts.
///
/// The no-break space and the soft hyphen are left out: PDFium maps their codes to plain space
/// and hyphen, finds no code for U+00A0 / U+00AD and would write "ÿ" instead.
pub(super) fn winansi_code(c: char) -> Option<u8> {
    match c {
        ' '..='~' | '\u{A1}'..='\u{AC}' | '\u{AE}'..='\u{FF}' => u8::try_from(c).ok(),
        _ => WINANSI_HIGH
            .iter()
            .find(|&&(high, _)| high == c)
            .map(|&(_, code)| code),
    }
}

/// Advance width of `c`, in 1/1000 em.
fn char_units(c: char, font: Font) -> u16 {
    let table = match font {
        Font::Regular => &REGULAR,
        Font::Bold => &BOLD,
    };
    winansi_code(c)
        .and_then(|code| table.get(usize::from(code).checked_sub(0x20)?))
        .copied()
        .filter(|&width| width > 0)
        .unwrap_or(FALLBACK_WIDTH)
}

/// Width of `c` drawn at `size` points, in points.
pub(super) fn char_width(c: char, font: Font, size: f32) -> f32 {
    f32::from(char_units(c, font)) * size / 1000.0
}

/// Width of `text` drawn at `size` points, in points (no kerning: PDFium applies none).
pub(super) fn text_width(text: &str, font: Font, size: f32) -> f32 {
    let units: u32 = text.chars().map(|c| u32::from(char_units(c, font))).sum();
    units as f32 * size / 1000.0
}

/// `text` limited to what the standard fonts can show: control chars become spaces, invisible
/// formatting chars are dropped and other chars outside WinAnsiEncoding become their
/// compatibility decomposition when it fits (e.g. "ő" → "o", "ﬁ" → "fi") or "?".
pub(super) fn sanitize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_control() {
            out.push(' ');
        } else if is_invisible(c) {
            // Dropped.
        } else if winansi_code(c).is_some() {
            out.push(c);
        } else {
            let before = out.len();
            decompose_compatible(c, |part| {
                if winansi_code(part).is_some() {
                    out.push(part);
                }
            });
            if out.len() == before {
                out.push('?');
            }
        }
    }
    out
}

/// Chars that are not shown on screen (soft hyphen, zero-width and direction marks, BOM...).
fn is_invisible(c: char) -> bool {
    matches!(
        c,
        '\u{AD}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}'
            | '\u{FEFF}'
    )
}

/// `text` cut at the end with "…" so that it fits in `max_width` points.
pub(super) fn fit_end(text: &str, font: Font, size: f32, max_width: f32) -> String {
    if text_width(text, font, size) <= max_width {
        return text.to_string();
    }
    let room = max_width - text_width("…", font, size);
    let mut width = 0.0;
    let mut end = 0;
    for (i, c) in text.char_indices() {
        width += char_width(c, font, size);
        if width > room {
            break;
        }
        end = i + c.len_utf8();
    }
    format!("{}…", text[..end].trim_end())
}

/// `text` cut in the middle with "…" so that it fits in `max_width` points. Keeps the start and
/// the end of file names, where the note number and the extension usually are.
pub(super) fn fit_middle(text: &str, font: Font, size: f32, max_width: f32) -> String {
    if text_width(text, font, size) <= max_width {
        return text.to_string();
    }
    let room = (max_width - text_width("…", font, size)).max(0.0);
    let chars: Vec<char> = text.chars().collect();
    // The end takes up to 40% of the room, the start the rest.
    let mut tail_width = 0.0;
    let mut tail_start = chars.len();
    while tail_start > 0 {
        let width = char_width(chars[tail_start - 1], font, size);
        if tail_width + width > room * 0.4 {
            break;
        }
        tail_width += width;
        tail_start -= 1;
    }
    let mut head_width = tail_width;
    let mut head_end = 0;
    while head_end < tail_start {
        let width = char_width(chars[head_end], font, size);
        if head_width + width > room {
            break;
        }
        head_width += width;
        head_end += 1;
    }
    let head: String = chars[..head_end].iter().collect();
    let tail: String = chars[tail_start..].iter().collect();
    format!("{}…{}", head.trim_end(), tail.trim_start())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{dev_pdfium_dir, Engine};
    use pdfium_render::prelude::*;

    #[test]
    fn maps_chars_to_winansi() {
        assert_eq!(winansi_code('A'), Some(0x41));
        assert_eq!(winansi_code('ç'), Some(0xE7));
        assert_eq!(winansi_code('Ã'), Some(0xC3));
        assert_eq!(winansi_code('€'), Some(0x80));
        assert_eq!(winansi_code('…'), Some(0x85));
        assert_eq!(winansi_code('—'), Some(0x97));
        assert_eq!(winansi_code('\u{7F}'), None);
        assert_eq!(winansi_code('\u{85}'), None);
        assert_eq!(winansi_code('\u{A0}'), None);
        assert_eq!(winansi_code('\u{AD}'), None);
        assert_eq!(winansi_code('ł'), None);
        assert_eq!(winansi_code('中'), None);
    }

    #[test]
    fn sanitizes_text_for_the_standard_fonts() {
        assert_eq!(sanitize("Serviço de manutenção"), "Serviço de manutenção");
        assert_eq!(sanitize("a\tb\nc"), "a b c");
        assert_eq!(sanitize("Győr ﬁnal"), "Gyor final");
        assert_eq!(sanitize("nota\u{200B}\u{AD}1"), "nota1");
        assert_eq!(sanitize("R$\u{A0}10\u{202F}mil"), "R$ 10 mil");
        assert_eq!(sanitize("Łódź 中"), "?ódz ?");
        assert_eq!(sanitize("😀.pdf"), "?.pdf");
    }

    #[test]
    fn measures_text() {
        // "Relatório": R 722, e 556, l 222, a 556, t 278, ó 556, r 333, i 222, o 556.
        assert!((text_width("Relatório", Font::Regular, 10.0) - 40.01).abs() < 0.001);
        assert!(text_width("TOTAL", Font::Bold, 8.0) > text_width("TOTAL", Font::Regular, 8.0));
        assert_eq!(text_width("", Font::Regular, 8.0), 0.0);
    }

    #[test]
    fn cuts_text_to_fit() {
        let name = "NFS-e 000123 EMPRESA EXEMPLO LTDA serviços de março de 2026.pdf";
        let short = fit_end(name, Font::Regular, 8.0, 100.0);
        assert!(short.ends_with('…'), "{short}");
        assert!(text_width(&short, Font::Regular, 8.0) <= 100.0);
        assert!(name.starts_with(short.trim_end_matches('…')));

        let middle = fit_middle(name, Font::Regular, 8.0, 120.0);
        assert!(text_width(&middle, Font::Regular, 8.0) <= 120.0);
        let (head, tail) = middle.split_once('…').expect("ellipsis");
        assert!(name.starts_with(head), "{middle}");
        assert!(name.ends_with(tail) && tail.ends_with(".pdf"), "{middle}");

        assert_eq!(fit_end("curto.pdf", Font::Regular, 8.0, 100.0), "curto.pdf");
        assert_eq!(fit_middle("curto.pdf", Font::Bold, 8.0, 100.0), "curto.pdf");
        assert_eq!(fit_end("abc", Font::Regular, 8.0, 1.0), "…");
        assert_eq!(fit_middle("abc", Font::Regular, 8.0, 1.0), "…");
    }

    /// The widths of the tables match the advances PDFium uses when it lays out the text. The
    /// advance of `c` is the growth of the bounds of "HH" when `c` is put between the two "H".
    #[test]
    fn widths_match_pdfium() {
        // Drawn by PDFium with other advances (see the module documentation).
        const PDFIUM_DIFFERS: [char; 5] = ['±', '÷', 'µ', '·', '¯'];
        let engine =
            Engine::new(&[dev_pdfium_dir()]).expect("PDFium not found: run scripts/fetch-pdfium");
        let _pdfium = engine.lock();
        let mut document = engine.pdfium().create_new_pdf().unwrap();
        let fonts = [
            (Font::Regular, document.fonts_mut().helvetica()),
            (Font::Bold, document.fonts_mut().helvetica_bold()),
        ];
        let size = 100.0;
        let bounds_width = |text: &str, font: PdfFontToken| {
            PdfPageTextObject::new(&document, text, font, PdfPoints::new(size))
                .unwrap()
                .width()
                .unwrap()
                .value
        };
        let chars = (' '..='~')
            .chain('\u{A0}'..='\u{FF}')
            .chain(WINANSI_HIGH.iter().map(|&(c, _)| c))
            .filter(|&c| winansi_code(c).is_some() && !PDFIUM_DIFFERS.contains(&c));
        let mut mismatches = Vec::new();
        for (font, token) in fonts {
            let base = bounds_width("HH", token);
            for c in chars.clone() {
                let advance = (bounds_width(&format!("H{c}H"), token) - base) * 1000.0 / size;
                let expected = f32::from(char_units(c, font));
                if (advance - expected).abs() > 1.0 {
                    mismatches.push(format!("{font:?} {c:?}: {expected} x PDFium {advance:.1}"));
                }
            }
        }
        assert!(mismatches.is_empty(), "{mismatches:#?}");
    }
}
