//! Scan, CSV export and the JSON shape of the contract. None of these need PDFium.

use nfsetable_core::{
    builtin_profiles, export_csv, scan, CsvRow, DocKind, DocResult, FieldValue, Origin, Rect,
    RegionAnchor, Rule, ScanOptions, Source, Status,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

mod common;
use common::fixtures;

fn names(paths: impl IntoIterator<Item = String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|p| {
            Path::new(&p)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn scan_filters_sorts_and_finds_duplicates() {
    let result = scan(&ScanOptions {
        sources: vec![Source {
            path: fixtures().to_string_lossy().into_owned(),
            recursive: false,
        }],
        exclude: vec!["CANCELADA".into()],
    });
    let files = names(result.files.iter().map(|f| f.path.clone()));
    assert_eq!(
        files,
        [
            "blank.pdf",
            "corrupt.pdf",
            "danfse-v1.pdf",
            "danfse-v1-copia.pdf",
            "danfse-v2.pdf",
            "legacy-municipal.pdf",
            "rotated.pdf",
            "sideways.pdf",
            "unusual.pdf",
            "unusual-shifted.pdf",
        ]
    );
    assert_eq!(names(result.excluded.clone()), ["nota-cancelada.pdf"]);

    let copy = result
        .files
        .iter()
        .find(|f| f.name == "danfse-v1-copia.pdf")
        .unwrap();
    let original = result
        .files
        .iter()
        .find(|f| f.name == "danfse-v1.pdf")
        .unwrap();
    assert_eq!(copy.duplicate_of.as_deref(), Some(original.path.as_str()));
    assert_eq!(copy.hash, original.hash);
    assert_eq!(original.duplicate_of, None);
    assert_eq!(
        result
            .files
            .iter()
            .filter(|f| f.duplicate_of.is_some())
            .count(),
        1
    );

    assert_eq!(result.sources.len(), 1);
    assert!(result.sources[0].is_dir && result.sources[0].exists);
    assert_eq!(result.sources[0].file_count, 10);
}

#[test]
fn scan_recurses_accepts_files_and_reports_missing_sources() {
    let dir = fixtures();
    let result = scan(&ScanOptions {
        sources: vec![
            Source {
                path: dir.to_string_lossy().into_owned(),
                recursive: true,
            },
            // Already covered by the folder: listed once.
            Source {
                path: dir.join("danfse-v2.pdf").to_string_lossy().into_owned(),
                recursive: false,
            },
            Source {
                path: dir.join("missing-folder").to_string_lossy().into_owned(),
                recursive: true,
            },
        ],
        // Accent- and case-insensitive.
        exclude: vec!["CANCELÁDA".into()],
    });
    assert_eq!(result.files.len(), 11);
    assert!(result.files.iter().any(|f| f.name == "danfse-v2-sub.pdf"));
    assert_eq!(
        result
            .files
            .iter()
            .filter(|f| f.name == "danfse-v2.pdf")
            .count(),
        1
    );
    assert_eq!(result.sources[1].file_count, 1);
    assert!(!result.sources[1].is_dir);
    assert!(!result.sources[2].exists);
    assert_eq!(result.sources[2].file_count, 0);
}

#[test]
fn csv_is_excel_friendly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notas.csv");
    let rows = vec![
        CsvRow {
            file: "nota; com separador.pdf".into(),
            path: "C:/Notas/nota; com separador.pdf".into(),
            doc_type: "NFS-e".into(),
            kind: Some(DocKind::Revenue),
            competence: Some("2026-03".into()),
            cents: Some(123_456),
            status: "OK".into(),
            origin: "DANFSe (padrão nacional)".into(),
        },
        CsvRow {
            file: "recibo.pdf".into(),
            path: "C:/Notas/recibo.pdf".into(),
            doc_type: "PDF".into(),
            kind: None,
            competence: None,
            cents: None,
            status: "Não encontrado".into(),
            origin: String::new(),
        },
        CsvRow {
            file: "ajuste \"final\".pdf".into(),
            path: "C:/Notas/ajuste \"final\".pdf".into(),
            doc_type: "PDF".into(),
            kind: None,
            competence: Some("2026-04".into()),
            cents: Some(-1_050),
            status: "OK".into(),
            origin: "Manual".into(),
        },
    ];
    export_csv(&rows, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"\xEF\xBB\xBF"));
    let text = String::from_utf8(bytes[3..].to_vec()).unwrap();
    let lines: Vec<&str> = text.split("\r\n").collect();
    assert_eq!(
        lines,
        [
            "Arquivo;Caminho;Tipo;Natureza;Competência;Valor;Status;Origem",
            "\"nota; com separador.pdf\";\"C:/Notas/nota; com separador.pdf\";NFS-e;Receita;03/2026;1234,56;OK;DANFSe (padrão nacional)",
            "recibo.pdf;C:/Notas/recibo.pdf;PDF;Receita;;;Não encontrado;",
            "\"ajuste \"\"final\"\".pdf\";\"C:/Notas/ajuste \"\"final\"\".pdf\";PDF;Receita;04/2026;-10,50;OK;Manual",
            "TOTAL;;;;;1224,06;;",
            "",
        ]
    );
}

/// The frontend (app/src/lib/types.ts) relies on this exact JSON shape.
#[test]
fn json_shape_matches_the_frontend_types() {
    let rule = Rule::Region {
        rect: Rect {
            x: 0.1,
            y: 0.2,
            w: 0.3,
            h: 0.04,
        },
        page: 0,
        anchor: Some(RegionAnchor {
            text: "montante a pagar".into(),
            dx: 0.0,
            dy: 10.0,
            w: 80.0,
            h: 12.0,
        }),
    };
    let value = serde_json::to_value(&rule).unwrap();
    assert_eq!(value["type"], "region");
    assert_eq!(value["anchor"]["text"], "montante a pagar");

    let anchor = &builtin_profiles()[0].fields["net_value"][0];
    let value = serde_json::to_value(anchor).unwrap();
    assert_eq!(value["type"], "anchor");
    assert_eq!(value["excludeSuffix"], "^\\s*\\+");
    assert_eq!(value["direction"], "below");
    assert_eq!(value["maxDistance"], 24.0);

    let mut fields = BTreeMap::new();
    fields.insert(
        "net_value".to_string(),
        FieldValue {
            raw: "R$ 1.234,56".into(),
            cents: Some(123_456),
            date: None,
            text: None,
            origin: Origin::Profile {
                id: "danfse-national".into(),
                name: "DANFSe (padrão nacional)".into(),
            },
            page: 0,
            bbox: Rect::default(),
        },
    );
    let doc = DocResult {
        path: "C:/Notas/a.pdf".into(),
        status: Status::NotFound,
        message: None,
        doc_type: "NFS-e".into(),
        kind: DocKind::Revenue,
        fields,
        page_count: 1,
    };
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(value["status"], "notFound");
    assert_eq!(value["docType"], "NFS-e");
    assert_eq!(value["kind"], "revenue");
    assert_eq!(value["pageCount"], 1);
    assert_eq!(value["message"], serde_json::Value::Null);
    assert_eq!(
        value["fields"]["net_value"]["origin"],
        json!({ "type": "profile", "id": "danfse-national", "name": "DANFSe (padrão nacional)" })
    );
    assert_eq!(
        serde_json::to_value(Origin::Auto).unwrap(),
        json!({ "type": "auto" })
    );

    // Optional keys may be omitted by the frontend.
    let parsed: Rule = serde_json::from_value(json!({
        "type": "region", "rect": { "x": 0.1, "y": 0.1, "w": 0.1, "h": 0.1 }, "page": 0
    }))
    .unwrap();
    assert!(matches!(parsed, Rule::Region { anchor: None, .. }));
}
