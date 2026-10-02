//! Text normalization and the positioned text model.
//!
//! [`normalize`] is the canonical normalization used by rule patterns, fingerprints, anchors and
//! the exclude filter: lowercase, accents removed, whitespace runs collapsed to a single space,
//! trimmed.
//!
//! The positioned text model (PDFium chars -> lines -> segments, with a char-index map so that
//! regex matches over normalized text map back to char boxes) belongs to the extraction engine and
//! is built on top of [`fold_char`], which folds one source char independently of its neighbours
//! and therefore keeps the index map trivial to maintain.

use unicode_normalization::char::{decompose_canonical, is_combining_mark};

/// Folds one source char: canonical decomposition, combining marks (accents) dropped, lowercase.
/// Emits zero or more chars (zero for a standalone combining mark). Whitespace is NOT collapsed
/// here; callers handle it.
pub fn fold_char(c: char, mut emit: impl FnMut(char)) {
    decompose_canonical(c, |d| {
        if is_combining_mark(d) {
            return;
        }
        for lower in d.to_lowercase() {
            if !is_combining_mark(lower) {
                emit(lower);
            }
        }
    });
}

/// Lowercase, remove accents, collapse whitespace runs to single spaces and trim.
///
/// `"  Valor Líquido\nda NFS-e "` -> `"valor liquido da nfs-e"`.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            pending_space = !out.is_empty();
            continue;
        }
        fold_char(c, |f| {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            out.push(f);
        });
    }
    out
}

/// [`normalize`] with ALL whitespace removed (used to match profile fingerprints).
pub fn compact(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if !c.is_whitespace() {
            fold_char(c, |f| out.push(f));
        }
    }
    out
}

/// "1 nota", "3 notas": `count` followed by the singular or the plural word.
pub fn count_label(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_accents_case_and_spaces() {
        assert_eq!(
            normalize("  VALOR LÍQUIDO\n da  NFS-e "),
            "valor liquido da nfs-e"
        );
        assert_eq!(normalize("Retenções\u{a0}Ç"), "retencoes c");
        assert_eq!(normalize(""), "");
        // Already decomposed input (base char + combining acute accent).
        assert_eq!(normalize("Li\u{301}quido"), "liquido");
    }

    #[test]
    fn compacts_whitespace_away() {
        assert_eq!(compact("DANFSe v1.0"), "danfsev1.0");
        assert_eq!(compact("Valor Líquido da NFS-e"), "valorliquidodanfs-e");
    }
}
