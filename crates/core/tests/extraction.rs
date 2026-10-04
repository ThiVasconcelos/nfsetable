//! Extraction over the synthetic fixtures (regenerate them with
//! `cargo run -p nfsetable-core --example gen_fixtures`). Needs PDFium: run scripts/fetch-pdfium first.

use nfsetable_core::{
    builtin_fields, builtin_profiles, DocKind, DocResult, FieldKind, Origin, Profile, Rect, Rule,
    Status, COMPETENCE_FIELD, NET_VALUE_FIELD, SERVICE_VALUE_FIELD,
};
use std::collections::BTreeMap;

mod common;
use common::{engine, fixture};

fn extract_with(name: &str, profiles: &[Profile]) -> DocResult {
    engine().extract(&fixture(name), profiles, &builtin_fields())
}

fn extract(name: &str) -> DocResult {
    extract_with(name, &builtin_profiles())
}

fn cents(doc: &DocResult) -> Option<i64> {
    doc.fields.get(NET_VALUE_FIELD).and_then(|v| v.cents)
}

fn service_cents(doc: &DocResult) -> Option<i64> {
    doc.fields.get(SERVICE_VALUE_FIELD).and_then(|v| v.cents)
}

fn competence(doc: &DocResult) -> Option<&str> {
    doc.fields
        .get(COMPETENCE_FIELD)
        .and_then(|v| v.date.as_deref())
}

fn origin(doc: &DocResult) -> Option<&Origin> {
    doc.fields.get(NET_VALUE_FIELD).map(|v| &v.origin)
}

fn danfse_origin() -> Origin {
    Origin::Profile {
        id: "danfse-national".into(),
        name: "DANFSe (padrão nacional)".into(),
    }
}

#[test]
fn danfse_v1_with_glued_label() {
    let doc = extract("danfse-v1.pdf");
    assert_eq!(doc.status, Status::Ok, "{doc:?}");
    assert_eq!(cents(&doc), Some(321_098));
    assert_eq!(origin(&doc), Some(&danfse_origin()));
    assert_eq!(doc.doc_type, "NFS-e");
    assert_eq!(doc.page_count, 1);
    let bbox = doc.fields[NET_VALUE_FIELD].bbox;
    assert!((bbox.x - 439.4).abs() < 2.0, "{bbox:?}");
    assert!(bbox.y > 605.0 && bbox.y < 625.0, "{bbox:?}");
    // Gross value and competence for the taxes.
    assert_eq!(service_cents(&doc), Some(330_000));
    assert_eq!(competence(&doc), Some("2026-03-05"));
}

#[test]
fn danfse_v2_ignores_operation_value_and_ibs_cbs_column() {
    let doc = extract("danfse-v2.pdf");
    assert_eq!(doc.status, Status::Ok, "{doc:?}");
    assert_eq!(cents(&doc), Some(123_456));
    assert_eq!(origin(&doc), Some(&danfse_origin()));
    let bbox = doc.fields[NET_VALUE_FIELD].bbox;
    assert!((bbox.x - 156.5).abs() < 2.0, "{bbox:?}");
    assert!(bbox.y > 510.0, "{bbox:?}");
    assert_eq!(service_cents(&doc), Some(130_000));
    assert_eq!(competence(&doc), Some("2026-04-10"));
}

#[test]
fn legacy_layout_uses_the_heuristic() {
    let doc = extract("legacy-municipal.pdf");
    assert_eq!(doc.status, Status::Ok, "{doc:?}");
    assert_eq!(cents(&doc), Some(250_000));
    assert_eq!(origin(&doc), Some(&Origin::Auto));
    assert_eq!(doc.doc_type, "PDF");
    // The heuristic also finds the gross value and a date to use as competence.
    assert_eq!(service_cents(&doc), Some(265_000));
    assert_eq!(competence(&doc), Some("2026-04-15"));
}

#[test]
fn landscape_page_boxes_are_in_display_coordinates() {
    let doc = extract("rotated.pdf");
    assert_eq!(doc.status, Status::Ok, "{doc:?}");
    assert_eq!(cents(&doc), Some(55_555));
    let bbox = doc.fields[NET_VALUE_FIELD].bbox;
    // Same place as in the upright v2 layout, on a page displayed as 842 x 595.
    assert!((bbox.x - 156.5).abs() < 2.0, "{bbox:?}");
    assert!(bbox.y > 510.0 && bbox.y < 530.0, "{bbox:?}");

    let page = engine()
        .render_page(&fixture("rotated.pdf"), 0, 400)
        .unwrap();
    assert_eq!(
        (page.width_pt.round(), page.height_pt.round()),
        (842.0, 595.0)
    );
}

#[test]
fn sideways_text_does_not_break_extraction() {
    let doc = extract("sideways.pdf");
    assert_eq!(doc.status, Status::NotFound, "{doc:?}");
}

#[test]
fn blank_and_corrupt_files_report_their_status() {
    let blank = extract("blank.pdf");
    assert_eq!(blank.status, Status::NoText);
    assert!(blank.message.is_some());

    let corrupt = extract("corrupt.pdf");
    assert_eq!(corrupt.status, Status::Error);
    assert!(corrupt.message.unwrap().contains("PDF"));

    let missing = extract("does-not-exist.pdf");
    assert_eq!(missing.status, Status::Error);
}

/// The whole "teach the value position" flow: not found, draw a region, test it on a file whose
/// block moved down, save it as a profile.
#[test]
fn region_rule_follows_the_anchor_and_can_be_saved() {
    let doc = extract("unusual.pdf");
    assert_eq!(doc.status, Status::NotFound);
    assert!(!doc.fields.contains_key(NET_VALUE_FIELD));

    // A loose rectangle around "R$ 845,10" (top 410, size 9).
    let drawn = Rect {
        x: 294.0,
        y: 404.0,
        w: 70.0,
        h: 20.0,
    };
    let read = engine()
        .read_region(&fixture("unusual.pdf"), 0, drawn, FieldKind::Money)
        .unwrap();
    assert!(read.text.contains("845,10"), "{read:?}");
    assert_eq!(read.value.as_ref().and_then(|v| v.cents), Some(84_510));
    let Rule::Region { anchor, .. } = &read.suggested_rule else {
        panic!("expected a region rule: {read:?}");
    };
    assert_eq!(
        anchor.as_ref().map(|a| a.text.as_str()),
        Some("montante a pagar")
    );

    let field = &builtin_fields()[0];
    let test = |name: &str, rule: &Rule| {
        let result = engine().test_rule(&fixture(name), field, rule);
        assert!(result.error.is_none(), "{result:?}");
        result.value.and_then(|v| v.cents)
    };
    assert_eq!(test("unusual.pdf", &read.suggested_rule), Some(84_510));
    assert_eq!(
        test("unusual-shifted.pdf", &read.suggested_rule),
        Some(91_245)
    );

    // Without the anchor the fixed position misses the moved block.
    let mut fixed = read.suggested_rule.clone();
    if let Rule::Region { anchor, .. } = &mut fixed {
        *anchor = None;
    }
    assert_eq!(test("unusual-shifted.pdf", &fixed), None);

    // Saved as a user profile, the rule is used automatically; built-ins still win on DANFSe.
    let mut fields = BTreeMap::new();
    fields.insert(
        NET_VALUE_FIELD.to_string(),
        vec![read.suggested_rule.clone()],
    );
    let mut profiles = builtin_profiles();
    profiles.push(Profile {
        id: "recibos-1".into(),
        name: "Recibos".into(),
        builtin: false,
        fingerprint: Vec::new(),
        name_patterns: Vec::new(),
        doc_type: None,
        kind: None,
        fields,
    });
    let shifted = extract_with("unusual-shifted.pdf", &profiles);
    assert_eq!(shifted.status, Status::Ok);
    assert_eq!(cents(&shifted), Some(91_245));
    assert_eq!(
        origin(&shifted),
        Some(&Origin::Profile {
            id: "recibos-1".into(),
            name: "Recibos".into()
        })
    );
    assert_eq!(
        origin(&extract_with("danfse-v1.pdf", &profiles)),
        Some(&danfse_origin())
    );
}

#[test]
fn test_rule_reports_unreadable_files() {
    let rule = Rule::Region {
        rect: Rect {
            x: 0.5,
            y: 0.5,
            w: 0.1,
            h: 0.02,
        },
        page: 0,
        anchor: None,
    };
    let result = engine().test_rule(&fixture("corrupt.pdf"), &builtin_fields()[0], &rule);
    assert!(result.value.is_none());
    assert!(result.error.is_some());
}

#[test]
fn read_region_rejects_missing_pages() {
    let rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
    };
    let err = engine()
        .read_region(&fixture("danfse-v1.pdf"), 3, rect, FieldKind::Money)
        .unwrap_err();
    assert!(err.to_string().contains("página 4"), "{err}");

    // The last possible index is an error too, not an overflow.
    let err = engine()
        .read_region(&fixture("danfse-v1.pdf"), u32::MAX, rect, FieldKind::Money)
        .unwrap_err();
    assert!(err.to_string().contains("página 4294967296"), "{err}");
}

#[test]
fn render_page_returns_a_png_data_url() {
    let page = engine()
        .render_page(&fixture("danfse-v2.pdf"), 0, 600)
        .unwrap();
    assert!(page.data_url.starts_with("data:image/png;base64,"));
    assert!(page.data_url.len() > 1_000);
    assert_eq!(
        (page.width_pt.round(), page.height_pt.round()),
        (595.0, 842.0)
    );
    assert_eq!(page.page_count, 1);
}

/// A user profile that only classifies: matched by file name and/or text, it sets the type and
/// the kind, while the values keep coming from the built-in rules.
fn classifier(name: &str, patterns: &[&str], fingerprint: &[&str], doc_type: &str) -> Profile {
    Profile {
        id: format!("{}-1", name.to_lowercase()),
        name: name.into(),
        builtin: false,
        fingerprint: fingerprint.iter().map(|s| s.to_string()).collect(),
        name_patterns: patterns.iter().map(|s| s.to_string()).collect(),
        doc_type: Some(doc_type.into()),
        kind: Some(DocKind::Expense),
        fields: BTreeMap::new(),
    }
}

#[test]
fn documents_are_revenue_by_default() {
    let v1 = extract("danfse-v1.pdf");
    assert_eq!((v1.doc_type.as_str(), v1.kind), ("NFS-e", DocKind::Revenue));
    let legacy = extract("legacy-municipal.pdf");
    assert_eq!(
        (legacy.doc_type.as_str(), legacy.kind),
        ("PDF", DocKind::Revenue)
    );
}

#[test]
fn user_profiles_classify_by_file_name_and_text() {
    let mut profiles = builtin_profiles();
    // By file name, with a wildcard: the municipal note becomes a rent expense.
    profiles.push(classifier("Aluguel", &["LEGACY-*"], &[], "Aluguel"));
    // By text and file name: the DANFSe v1 (e.g. issued by the accountant) becomes an expense,
    // although the built-in DANFSe profile also matches it.
    profiles.push(classifier("Contador", &["v1"], &["danfse"], "Contador"));

    let legacy = extract_with("legacy-municipal.pdf", &profiles);
    assert_eq!(legacy.doc_type, "Aluguel");
    assert_eq!(legacy.kind, DocKind::Expense);
    assert_eq!(cents(&legacy), Some(250_000));

    let v1 = extract_with("danfse-v1.pdf", &profiles);
    assert_eq!(v1.doc_type, "Contador");
    assert_eq!(v1.kind, DocKind::Expense);
    // The value still comes from the built-in DANFSe rules.
    assert_eq!(cents(&v1), Some(321_098));
    assert_eq!(origin(&v1), Some(&danfse_origin()));

    // Not matched by any classifier: unchanged.
    let v2 = extract_with("danfse-v2.pdf", &profiles);
    assert_eq!((v2.doc_type.as_str(), v2.kind), ("NFS-e", DocKind::Revenue));
}
