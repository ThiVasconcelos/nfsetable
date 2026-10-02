//! SQL scripts that create the `notas` table and insert the rows, for PostgreSQL and for
//! MySQL/MariaDB.

use super::{competence, kind, one_line, truncate_chars, Summary};
use crate::model::{DocKind, ExportRow};
use crate::parse::format_brl;
use crate::parse::format_decimal_with;

/// Rows per `INSERT` statement.
const BATCH_SIZE: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Dialect {
    Postgresql,
    Mysql,
}

const POSTGRESQL_PREAMBLE: &str = "\
-- Script para PostgreSQL.

SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;

BEGIN;

CREATE TABLE IF NOT EXISTS notas (
    id SERIAL PRIMARY KEY,
    arquivo TEXT NOT NULL,
    caminho TEXT NOT NULL,
    tipo TEXT NOT NULL,
    natureza TEXT NOT NULL,
    competencia DATE,
    valor NUMERIC(14,2),
    status TEXT NOT NULL,
    origem TEXT NOT NULL
);
";

const MYSQL_PREAMBLE: &str = "\
-- Script para MySQL / MariaDB. Assume o sql_mode padrão, em que a barra invertida é caractere de
-- escape nos textos (sem NO_BACKSLASH_ESCAPES).

SET NAMES utf8mb4;
START TRANSACTION;

CREATE TABLE IF NOT EXISTS `notas` (
    `id` INT AUTO_INCREMENT PRIMARY KEY,
    `arquivo` VARCHAR(512) NOT NULL,
    `caminho` TEXT NOT NULL,
    `tipo` VARCHAR(100) NOT NULL,
    `natureza` VARCHAR(10) NOT NULL,
    `competencia` DATE NULL,
    `valor` DECIMAL(14,2) NULL,
    `status` VARCHAR(40) NOT NULL,
    `origem` VARCHAR(200) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
";

const POSTGRESQL_INSERT: &str = concat!(
    "INSERT INTO notas (arquivo, caminho, tipo, natureza, competencia, valor, status, origem) ",
    "VALUES\n"
);

const MYSQL_INSERT: &str = concat!(
    "INSERT INTO `notas` (`arquivo`, `caminho`, `tipo`, `natureza`, `competencia`, `valor`, ",
    "`status`, `origem`) VALUES\n"
);

/// Maximum length, in characters, of the MySQL `VARCHAR` columns (longer texts are cut so that
/// strict mode accepts them).
const MYSQL_FILE_CHARS: usize = 512;
const MYSQL_TYPE_CHARS: usize = 100;
const MYSQL_STATUS_CHARS: usize = 40;
const MYSQL_ORIGIN_CHARS: usize = 200;

/// The whole script: header comments (with the company, when given, already on one line), a
/// transaction with `CREATE TABLE IF NOT EXISTS notas` and the rows in `INSERT` statements of up
/// to 500 rows. Natureza becomes `'receita'` or `'despesa'`, competence `'yyyy-mm-01'`, amounts
/// `1234.56`; unknown values are `NULL`.
pub(super) fn to_string(
    dialect: Dialect,
    rows: &[ExportRow],
    generated_at: &str,
    company: Option<&str>,
) -> String {
    let summary = Summary::new(rows);
    let totals = &summary.totals;
    let mut out = String::with_capacity(2048 + rows.len() * 192);
    out.push_str(&format!(
        "-- Exportado pelo nfsetable em {}\n",
        one_line(generated_at)
    ));
    if let Some(company) = company {
        out.push_str(&format!("-- Empresa: {}\n", one_line(company)));
    }
    out.push_str(&format!("-- {}\n", summary.count_text()));
    out.push_str(&format!(
        "-- Receitas: {}\n-- Despesas: {}\n-- Saldo (receitas - despesas): {}\n",
        format_brl(totals.revenue.cents),
        format_brl(totals.expense.cents),
        format_brl(totals.balance())
    ));
    out.push_str(
        "-- Notas sem valor ou fora do total ficam com valor NULL e não entram nas somas.\n\
         -- Estimativa: confira com os documentos originais.\n",
    );
    out.push_str(match dialect {
        Dialect::Postgresql => POSTGRESQL_PREAMBLE,
        Dialect::Mysql => MYSQL_PREAMBLE,
    });
    let insert = match dialect {
        Dialect::Postgresql => POSTGRESQL_INSERT,
        Dialect::Mysql => MYSQL_INSERT,
    };
    for batch in rows.chunks(BATCH_SIZE) {
        out.push('\n');
        out.push_str(insert);
        for (i, row) in batch.iter().enumerate() {
            out.push_str("    (");
            push_row(&mut out, dialect, row);
            out.push_str(if i + 1 == batch.len() { ");\n" } else { "),\n" });
        }
    }
    out.push_str("\nCOMMIT;\n");
    out
}

fn push_row(out: &mut String, dialect: Dialect, row: &ExportRow) {
    let limit = |chars: usize| match dialect {
        Dialect::Postgresql => usize::MAX,
        Dialect::Mysql => chars,
    };
    push_text(
        out,
        dialect,
        truncate_chars(&row.file, limit(MYSQL_FILE_CHARS)),
    );
    out.push_str(", ");
    push_text(out, dialect, &row.path);
    out.push_str(", ");
    push_text(
        out,
        dialect,
        truncate_chars(&row.doc_type, limit(MYSQL_TYPE_CHARS)),
    );
    out.push_str(", ");
    out.push_str(match kind(row) {
        DocKind::Revenue => "'receita'",
        DocKind::Expense => "'despesa'",
    });
    out.push_str(", ");
    match competence(row) {
        Some(month) => {
            out.push('\'');
            out.push_str(&format!("{}-01", month.iso()));
            out.push('\'');
        }
        None => out.push_str("NULL"),
    }
    out.push_str(", ");
    match row.cents {
        Some(cents) => out.push_str(&format_decimal_with(cents, '.')),
        None => out.push_str("NULL"),
    }
    out.push_str(", ");
    push_text(
        out,
        dialect,
        truncate_chars(&row.status, limit(MYSQL_STATUS_CHARS)),
    );
    out.push_str(", ");
    push_text(
        out,
        dialect,
        truncate_chars(&row.origin, limit(MYSQL_ORIGIN_CHARS)),
    );
}

/// Appends `text` as a string literal.
///
/// PostgreSQL (`standard_conforming_strings` on, set by the script): quotes are doubled and
/// backslashes stay literal; NUL, which `TEXT` cannot hold, is removed.
///
/// MySQL (default `sql_mode`): backslash, NUL, line feed, carriage return and Ctrl-Z are
/// backslash escaped and quotes are doubled (valid in every `sql_mode`).
fn push_text(out: &mut String, dialect: Dialect, text: &str) {
    out.push('\'');
    for c in text.chars() {
        match (dialect, c) {
            (_, '\'') => out.push_str("''"),
            (Dialect::Postgresql, '\0') => {}
            (Dialect::Mysql, '\\') => out.push_str("\\\\"),
            (Dialect::Mysql, '\0') => out.push_str("\\0"),
            (Dialect::Mysql, '\n') => out.push_str("\\n"),
            (Dialect::Mysql, '\r') => out.push_str("\\r"),
            (Dialect::Mysql, '\u{1a}') => out.push_str("\\Z"),
            _ => out.push(c),
        }
    }
    out.push('\'');
}

/// The first `max` characters of `text`.
#[cfg(test)]
mod tests {
    use super::*;

    fn literal(dialect: Dialect, text: &str) -> String {
        let mut out = String::new();
        push_text(&mut out, dialect, text);
        out
    }

    #[test]
    fn escapes_postgresql_literals() {
        assert_eq!(
            literal(Dialect::Postgresql, "C:\\Notas\\d'água.pdf"),
            "'C:\\Notas\\d''água.pdf'"
        );
        assert_eq!(literal(Dialect::Postgresql, "a\0b\nc"), "'ab\nc'");
    }

    #[test]
    fn escapes_mysql_literals() {
        assert_eq!(
            literal(Dialect::Mysql, "C:\\Notas\\d'água.pdf"),
            "'C:\\\\Notas\\\\d''água.pdf'"
        );
        assert_eq!(
            literal(Dialect::Mysql, "a\0b\nc\rd\u{1a}e"),
            "'a\\0b\\nc\\rd\\Ze'"
        );
    }

    #[test]
    fn truncates_by_characters() {
        assert_eq!(truncate_chars("ação", 2), "aç");
        assert_eq!(truncate_chars("ação", 4), "ação");
        assert_eq!(truncate_chars("ação", 10), "ação");
        assert_eq!(truncate_chars("", 0), "");
    }
}
