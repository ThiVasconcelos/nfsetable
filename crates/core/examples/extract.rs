//! Command line reader: lists the net value of each NFS-e PDF and prints the total.
//!
//! cargo run -p nfsetable-core --example extract -- <folder-or-file>... [--exclude <text>]... [--recursive]

use nfsetable_core::parse::format_brl;
use nfsetable_core::{
    builtin_fields, builtin_profiles, dev_pdfium_dir, scan, DocResult, Engine, ScanOptions, Source,
    Status, COMPETENCE_FIELD, NET_VALUE_FIELD,
};
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "Uso: extract <pasta-ou-arquivo>... [--exclude <texto>]... [--recursive]

  --exclude, -e   ignora arquivos cujo nome contém o texto (pode repetir)
  --recursive, -r inclui subpastas";

fn main() -> ExitCode {
    let mut paths = Vec::new();
    let mut exclude = Vec::new();
    let mut recursive = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--exclude" | "-e" => match args.next() {
                Some(value) => exclude.push(value),
                None => {
                    eprintln!("--exclude precisa de um valor\n\n{USAGE}");
                    return ExitCode::from(2);
                }
            },
            "--recursive" | "-r" => recursive = true,
            "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => paths.push(arg),
        }
    }
    if paths.is_empty() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let engine = match Engine::new(&[dev_pdfium_dir()]) {
        Ok(engine) => engine,
        Err(err) => {
            eprintln!("{err}\nRode scripts/fetch-pdfium.sh (ou scripts/fetch-pdfium.ps1) antes.");
            return ExitCode::FAILURE;
        }
    };

    let sources = paths
        .into_iter()
        .map(|path| Source { path, recursive })
        .collect();
    let found = scan(&ScanOptions { sources, exclude });
    let profiles = builtin_profiles();
    let fields = builtin_fields();

    let mut total: i64 = 0;
    let mut read = 0usize;
    let mut missing = 0usize;
    for file in &found.files {
        if let Some(first) = &file.duplicate_of {
            println!(
                "{:>16}  {:>7}  {}  (cópia de {first}, não somada)",
                "-", "", file.name
            );
            continue;
        }
        let doc = engine.extract(Path::new(&file.path), &profiles, &fields);
        match doc
            .fields
            .get(NET_VALUE_FIELD)
            .and_then(|value| value.cents)
        {
            Some(cents) if doc.status == Status::Ok => {
                total += cents;
                read += 1;
                println!(
                    "{:>16}  {:>7}  {}",
                    format_brl(cents),
                    competence(&doc),
                    file.name
                );
            }
            _ => {
                missing += 1;
                let reason = doc.message.clone().unwrap_or_default();
                println!(
                    "{:>16}  {:>7}  {}  ({reason})",
                    "?",
                    competence(&doc),
                    file.name
                );
            }
        }
    }
    for path in &found.excluded {
        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        println!("{:>16}  {:>7}  {name}  (ignorado pelo filtro)", "-", "");
    }

    println!("\nTotal: {} em {read} nota(s)", format_brl(total));
    if missing > 0 {
        println!("Sem valor: {missing} arquivo(s)");
    }
    ExitCode::SUCCESS
}

/// Competence month as "MM/AAAA", or empty when the note does not show one.
fn competence(doc: &DocResult) -> String {
    doc.fields
        .get(COMPETENCE_FIELD)
        .and_then(|value| value.date.as_deref())
        .and_then(|iso| Some(format!("{}/{}", iso.get(5..7)?, iso.get(0..4)?)))
        .unwrap_or_default()
}
