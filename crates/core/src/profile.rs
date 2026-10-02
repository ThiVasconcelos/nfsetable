//! Built-in fields and profiles, fingerprint matching and profile validation.

use crate::error::CoreError;
use crate::model::{FieldDef, FieldKind, Profile, Rule};
use crate::regex_cache::compiled;
use crate::text::{compact, normalize};
use sha2::{Digest, Sha256};

/// Id of the built-in net value field ("Valor líquido", required).
pub const NET_VALUE_FIELD: &str = "net_value";
/// Id of the built-in gross value field ("Valor do serviço"), the revenue used by the taxes.
pub const SERVICE_VALUE_FIELD: &str = "service_value";
/// Id of the built-in competence field ("Competência"), the month used by the taxes.
pub const COMPETENCE_FIELD: &str = "competence";

/// Built-in profiles embedded at compile time from `crates/core/profiles/*.json` (file name, content).
const BUILTIN_PROFILE_SOURCES: &[(&str, &str)] = &[(
    "danfse-national.json",
    include_str!("../profiles/danfse-national.json"),
)];

/// Built-in field definitions: the net value (required) plus the gross value and the competence
/// used by the tax calculations (optional).
pub fn builtin_fields() -> Vec<FieldDef> {
    vec![
        FieldDef {
            id: NET_VALUE_FIELD.to_string(),
            label: "Valor líquido".to_string(),
            kind: FieldKind::Money,
            required: true,
        },
        FieldDef {
            id: SERVICE_VALUE_FIELD.to_string(),
            label: "Valor do serviço".to_string(),
            kind: FieldKind::Money,
            required: false,
        },
        FieldDef {
            id: COMPETENCE_FIELD.to_string(),
            label: "Competência".to_string(),
            kind: FieldKind::Date,
            required: false,
        },
    ]
}

/// SHA-256 of `profiles` as JSON: it changes whenever any profile changes, so results read with
/// other profiles can be told apart.
pub fn profiles_fingerprint(profiles: &[Profile]) -> String {
    let json = serde_json::to_vec(profiles).unwrap_or_default();
    format!("{:x}", Sha256::digest(&json))
}

/// Built-in extraction profiles (always `builtin: true`).
///
/// Panics only if an embedded JSON file is invalid, which the test suite rules out.
pub fn builtin_profiles() -> Vec<Profile> {
    BUILTIN_PROFILE_SOURCES
        .iter()
        .map(|(file, json)| {
            let mut profile: Profile = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("invalid built-in profile {file}: {e}"));
            profile.builtin = true;
            profile
        })
        .collect()
}

/// True when every fingerprint entry appears in `compact_text`, the document text normalized
/// with all whitespace removed (see [`crate::text::compact`]). An empty fingerprint matches any
/// document.
pub fn fingerprint_matches(profile: &Profile, compact_text: &str) -> bool {
    profile
        .fingerprint
        .iter()
        .map(|entry| compact(entry))
        .all(|entry| compact_text.contains(&entry))
}

/// True when at least one file name pattern of the profile matches `file_name`, or when the
/// profile has no patterns. See [`Profile::name_patterns`].
pub fn name_matches(profile: &Profile, file_name: &str) -> bool {
    if profile.name_patterns.is_empty() {
        return true;
    }
    let name = normalize(file_name);
    profile
        .name_patterns
        .iter()
        .any(|pattern| pattern_matches(pattern, &name))
}

/// Whether the profile applies to a document: fingerprint and file name patterns.
pub fn profile_matches(profile: &Profile, compact_text: &str, file_name: &str) -> bool {
    fingerprint_matches(profile, compact_text) && name_matches(profile, file_name)
}

/// `normalized_name` must already be normalized; the pattern is normalized here.
fn pattern_matches(pattern: &str, normalized_name: &str) -> bool {
    let pattern = normalize(pattern);
    if pattern.is_empty() {
        return false;
    }
    if pattern.contains(['*', '?']) {
        glob_matches(&pattern, normalized_name)
    } else {
        normalized_name.contains(&pattern)
    }
}

/// Whole-name match where `*` is any run of characters and `?` exactly one; everything else is
/// literal. Backtracks only to the last `*`, so it runs in linear time per pattern.
fn glob_matches(pattern: &str, name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let (mut p, mut n) = (0, 0);
    // After the last `*`: where the pattern resumes and how much of the name it has taken.
    let mut star: Option<(usize, usize)> = None;
    while n < name.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == name[n]) {
            p += 1;
            n += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p + 1, n));
            p += 1;
        } else if let Some((resume, taken)) = star {
            star = Some((resume, taken + 1));
            p = resume;
            n = taken + 1;
        } else {
            return false;
        }
    }
    pattern[p..].iter().all(|&c| c == '*')
}

/// Checks that a profile can be used: non-empty name and valid regular expressions.
pub fn validate_profile(profile: &Profile) -> Result<(), CoreError> {
    if profile.name.trim().is_empty() {
        return Err(CoreError::InvalidRule(
            "o perfil precisa de um nome".to_string(),
        ));
    }
    if profile
        .name_patterns
        .iter()
        .any(|p| normalize(p).is_empty())
    {
        return Err(CoreError::InvalidRule(
            "padrão de nome de arquivo vazio".to_string(),
        ));
    }
    if profile
        .doc_type
        .as_deref()
        .is_some_and(|t| t.trim().is_empty())
    {
        return Err(CoreError::InvalidRule("tipo vazio".to_string()));
    }
    for rules in profile.fields.values() {
        for rule in rules {
            validate_rule(rule)?;
        }
    }
    Ok(())
}

/// Checks that a rule can be used (valid regular expressions, sane geometry).
pub fn validate_rule(rule: &Rule) -> Result<(), CoreError> {
    match rule {
        Rule::Anchor {
            pattern,
            exclude_suffix,
            max_distance,
            ..
        } => {
            compiled(pattern)?;
            if let Some(suffix) = exclude_suffix {
                compiled(suffix)?;
            }
            if !(max_distance.is_finite() && *max_distance > 0.0) {
                return Err(CoreError::InvalidRule(
                    "a distância máxima deve ser positiva".to_string(),
                ));
            }
        }
        Rule::Region { rect, .. } => {
            let values = [rect.x, rect.y, rect.w, rect.h];
            if values.iter().any(|v| !v.is_finite()) || rect.w <= 0.0 || rect.h <= 0.0 {
                return Err(CoreError::InvalidRule("região inválida".to_string()));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Direction;

    #[test]
    fn fingerprint_follows_every_profile_change() {
        let profiles = builtin_profiles();
        let base = profiles_fingerprint(&profiles);
        assert_eq!(base.len(), 64);
        assert_eq!(profiles_fingerprint(&builtin_profiles()), base);
        let mut changed = profiles.clone();
        changed[0].doc_type = Some("Outro".to_string());
        assert_ne!(profiles_fingerprint(&changed), base);
        assert_ne!(profiles_fingerprint(&[]), base);
    }

    #[test]
    fn builtin_profile_is_valid() {
        let profiles = builtin_profiles();
        assert_eq!(profiles.len(), 1);
        let danfse = &profiles[0];
        assert_eq!(danfse.id, "danfse-national");
        assert!(danfse.builtin);
        assert_eq!(danfse.fingerprint, vec!["danfse".to_string()]);
        let rules = &danfse.fields[NET_VALUE_FIELD];
        assert_eq!(rules.len(), 2);
        assert!(matches!(
            &rules[0],
            Rule::Anchor { direction: Direction::Below, max_distance, exclude_suffix: Some(_), .. }
                if *max_distance == 24.0
        ));
        assert!(matches!(
            &rules[1],
            Rule::Anchor { direction: Direction::Right, max_distance, .. } if *max_distance == 250.0
        ));
        validate_profile(danfse).unwrap();
    }

    #[test]
    fn name_patterns_match_like_a_file_search() {
        let mut profile = builtin_profiles().remove(0);
        assert!(name_matches(&profile, "qualquer.pdf"));
        profile.name_patterns = vec!["Conta de LUZ".into(), "internet-*-2026.pdf".into()];
        // Substring, case and accents ignored.
        assert!(name_matches(&profile, "2026-03 conta de luz (março).pdf"));
        assert!(name_matches(&profile, "Internet-Março-2026.PDF"));
        assert!(!name_matches(&profile, "internet-marco-2025.pdf"));
        assert!(!name_matches(&profile, "aluguel.pdf"));
        // `?` is one character; the rest of the pattern is literal (no regex injection).
        profile.name_patterns = vec!["nota-??.pdf".into(), "a+b".into()];
        assert!(name_matches(&profile, "nota-07.pdf"));
        assert!(!name_matches(&profile, "nota-7.pdf"));
        assert!(name_matches(&profile, "recibo a+b.pdf"));
        assert!(!name_matches(&profile, "recibo aab.pdf"));
    }

    #[test]
    fn glob_patterns() {
        let accepted = [
            ("nota-*.pdf", "nota-1.pdf"),
            ("nota-*.pdf", "nota-.pdf"),
            ("*luz*", "conta de luz (marco).pdf"),
            ("a?c", "abc"),
            ("**x", "x"),
            ("*", ""),
        ];
        for (pattern, name) in accepted {
            assert!(glob_matches(pattern, name), "{pattern} should match {name}");
        }
        let rejected = [
            ("nota-*.pdf", "nota.pdf"),
            ("a?c", "ac"),
            ("a?c", "abbc"),
            ("*.pdf", "arquivo.pdfx"),
            ("x*", "yx"),
            ("nota-??.pdf", "nota-7.pdf"),
        ];
        for (pattern, name) in rejected {
            assert!(
                !glob_matches(pattern, name),
                "{pattern} should not match {name}"
            );
        }
    }

    #[test]
    fn fingerprint_uses_compact_text() {
        let mut profile = builtin_profiles().remove(0);
        assert!(fingerprint_matches(
            &profile,
            &compact("DANFSe v2.0 Documento Auxiliar")
        ));
        assert!(!fingerprint_matches(
            &profile,
            &compact("Nota Fiscal de Serviço")
        ));
        profile.fingerprint = vec!["Documento Auxiliar".to_string()];
        assert!(fingerprint_matches(
            &profile,
            &compact("DOCUMENTO   AUXILIAR da NFS-e")
        ));
        profile.fingerprint.clear();
        assert!(fingerprint_matches(&profile, ""));
    }
}
