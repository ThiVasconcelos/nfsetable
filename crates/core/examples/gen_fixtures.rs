//! Generates the synthetic PDF fixtures used by the tests (crates/core/tests/fixtures).
//!
//! cargo run -p nfsetable-core --example gen_fixtures
//!
//! Every name, CNPJ and amount here is made up. The DANFSe layouts reproduce only the positions
//! of the real documents (A4, points, top-left origin), which is what the extraction relies on.

use nfsetable_core::{dev_pdfium_dir, Engine};
use pdfium_render::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

const PAGE_HEIGHT: f32 = 842.0;

/// A text drawn at `x` with its top at `top` (points, top-left origin), turned `angle` degrees
/// counter-clockwise around the start of its baseline.
struct Text {
    text: &'static str,
    x: f32,
    top: f32,
    size: f32,
    bold: bool,
    angle: f32,
}

const fn t(text: &'static str, x: f32, top: f32, size: f32) -> Text {
    Text {
        text,
        x,
        top,
        size,
        bold: false,
        angle: 0.0,
    }
}

const fn b(text: &'static str, x: f32, top: f32, size: f32) -> Text {
    Text {
        text,
        x,
        top,
        size,
        bold: true,
        angle: 0.0,
    }
}

/// The watermark of a cancelled or substituted DANFSe (NT 008, §§ 2.5.1 and 2.5.2): 60 pt at 45°,
/// across the middle of the page, as the Receita Federal's generator draws it (as text).
fn watermark(text: &'static str) -> Text {
    Text {
        text,
        x: 165.0,
        top: 560.0,
        size: 60.0,
        bold: false,
        angle: 45.0,
    }
}

fn danfse_v1(net_value: &'static str) -> Vec<Text> {
    vec![
        b("Prefeitura de Cidade Exemplo", 40.0, 20.0, 10.0),
        b("DANFSe v1.0", 40.0, 34.0, 9.0),
        t("Documento Auxiliar da NFS-e", 40.0, 46.0, 8.0),
        t("Chave de Acesso da NFS-e", 40.0, 70.0, 6.0),
        t(
            "00000000000000000000000000000000000000000000000000",
            40.0,
            78.0,
            7.0,
        ),
        t("Número da NFS-e", 14.2, 100.0, 6.0),
        t("Competência da NFS-e", 160.0, 100.0, 6.0),
        t("Data e Hora da emissão da NFS-e", 300.0, 100.0, 6.0),
        t("12", 14.2, 108.0, 7.0),
        t("05/03/2026", 160.0, 108.0, 7.0),
        t("05/03/2026 10:15:00", 300.0, 108.0, 7.0),
        b("EMITENTE DA NFS-e", 14.2, 130.0, 7.0),
        t("Nome / Nome Empresarial", 14.2, 142.0, 6.0),
        t("CNPJ / CPF / NIF", 300.0, 142.0, 6.0),
        t("FULANO DE TAL SERVICOS DIGITAIS", 14.2, 150.0, 7.0),
        t("00.000.000/0001-91", 300.0, 150.0, 7.0),
        b("TOMADOR DO SERVIÇO", 14.2, 175.0, 7.0),
        t("EMPRESA EXEMPLO LTDA", 14.2, 187.0, 7.0),
        b("SERVIÇO PRESTADO", 14.2, 215.0, 7.0),
        t(
            "01.01.01 - Análise e desenvolvimento de sistemas",
            14.2,
            227.0,
            7.0,
        ),
        t("Descrição do Serviço", 14.2, 245.0, 6.0),
        t(
            "Desenvolvimento de software sob encomenda.",
            14.2,
            253.0,
            7.0,
        ),
        t("Valor do Serviço", 14.2, 471.4, 7.0),
        t("R$3.300,00", 14.2, 479.5, 8.0),
        t("Valor do Serviço", 14.2, 583.9, 7.0),
        t("Desconto Condicionado", 155.9, 583.9, 7.0),
        t("Desconto Incondicionado", 297.6, 583.9, 7.0),
        t("ISSQN Retido", 439.4, 583.9, 7.0),
        t("R$3.300,00", 14.2, 592.0, 8.0),
        t("R$", 155.9, 592.0, 8.0),
        t("R$", 297.6, 592.0, 8.0),
        t("-", 439.4, 592.0, 8.0),
        t("IRRF, CP, CSLL - Retidos", 14.2, 605.2, 7.0),
        t("PIS/COFINS Retidos", 297.6, 605.2, 7.0),
        // Glued on purpose: some DANFSe files have no spaces in their labels.
        t("ValorLíquidodaNFS-e", 439.4, 605.2, 7.0),
        t("R$89,02", 14.2, 613.2, 8.0),
        t("-", 297.6, 613.2, 8.0),
        t(net_value, 439.4, 613.2, 8.0),
        b("TOTAIS APROXIMADOS DOS TRIBUTOS", 14.2, 627.0, 7.0),
    ]
}

fn danfse_v2(net_value: &'static str) -> Vec<Text> {
    vec![
        b("DANFSe v2.0", 40.0, 20.0, 9.0),
        t("Município: Cidade Exemplo - UF", 300.0, 20.0, 7.0),
        t("Documento Auxiliar da NFS-e", 40.0, 32.0, 8.0),
        t("CHAVE DE ACESSO DA NFS-e", 40.0, 56.0, 6.0),
        t(
            "11111111111111111111111111111111111111111111111111",
            40.0,
            64.0,
            7.0,
        ),
        t("NÚMERO DA NFS-e", 11.9, 90.0, 6.0),
        t("COMPETÊNCIA DA NFS-e", 156.5, 90.0, 6.0),
        t("34", 11.9, 98.0, 7.0),
        t("10/04/2026", 156.5, 98.0, 7.0),
        b("PRESTADOR / FORNECEDOR", 11.9, 120.0, 7.0),
        t("FULANO DE TAL SERVICOS DIGITAIS", 11.9, 132.0, 7.0),
        b("TOMADOR / ADQUIRENTE", 11.9, 160.0, 7.0),
        t("EMPRESA EXEMPLO LTDA", 11.9, 172.0, 7.0),
        t("R$0,00", 11.9, 440.1, 7.0),
        t("Alíq. Efetiva Municipal - IBS", 11.9, 452.3, 6.0),
        t("Valor Apurado Municipal - IBS", 156.5, 452.3, 6.0),
        t("Valor Apurado Estadual - IBS", 445.6, 452.3, 6.0),
        b("VALOR TOTAL DA NFS-e", 11.9, 480.0, 7.0),
        t("VALOR DA OPERAÇÃO/SERVIÇO", 156.5, 490.0, 6.0),
        t("Desconto Incondicionado", 301.0, 490.0, 6.0),
        t("Desconto Condicionado", 445.6, 490.0, 6.0),
        // Trap: the operation value sits right above the net value label.
        t("R$1.300,00", 156.5, 497.9, 7.0),
        t("-", 301.0, 497.9, 7.0),
        t("-", 445.6, 497.9, 7.0),
        t("Total das Retenções (ISSQN/Federais)", 11.9, 510.0, 6.0),
        b("VALOR LÍQUIDO DA NFS-e", 156.5, 510.0, 6.0),
        t("Total do IBS/CBS", 301.0, 510.0, 6.0),
        // Trap: a second "net value" label that must be rejected.
        b("VALOR LÍQUIDO DA NFS-e + IBS/CBS", 445.6, 510.0, 6.0),
        t("R$65,44", 11.9, 516.9, 7.0),
        t(net_value, 156.5, 516.9, 7.0),
        t("R$12,34", 301.0, 516.9, 7.0),
        t("R$1.246,90", 445.6, 516.9, 7.0),
        b("INFORMAÇÕES COMPLEMENTARES", 11.9, 530.0, 7.0),
    ]
}

/// One centimetre in points.
const CM: f32 = 72.0 / 2.54;

/// x of the text of each column of the NT 008 grid (column edge plus the 0,08 cm inset).
const NT_COLUMNS: [f32; 4] = [10.77, 155.62, 300.19, 445.04];

/// A text of the Receita Federal's DANFSe generator placed by its top in cm. The generator puts
/// the baseline 0,908 × size below the top (Helvetica ascent plus its adjustment); `write_pdf` puts
/// it 0,8 × size below, hence the extra 0,108 × size.
fn nt(text: &'static str, x: f32, top_cm: f32, size: f32, bold: bool) -> Text {
    Text {
        text,
        x,
        top: top_cm * CM + 0.108 * size,
        size,
        bold,
        angle: 0.0,
    }
}

/// A field of the generator: a 6 pt bold label and its 7 pt value 0,30 cm below ('-' when empty).
fn field(
    out: &mut Vec<Text>,
    column: usize,
    top_cm: f32,
    label: &'static str,
    value: &'static str,
) {
    out.push(nt(label, NT_COLUMNS[column], top_cm, 6.0, true));
    out.push(nt(value, NT_COLUMNS[column], top_cm + 0.30, 7.0, false));
}

/// A field of the access rows: 7 pt bold label, 7 pt value 0,30 cm below.
fn access_field(
    out: &mut Vec<Text>,
    column: usize,
    top_cm: f32,
    label: &'static str,
    value: &'static str,
) {
    out.push(nt(label, NT_COLUMNS[column], top_cm, 7.0, true));
    out.push(nt(value, NT_COLUMNS[column], top_cm + 0.30, 7.0, false));
}

/// A section title: 7 pt bold, 0,07 cm below the top of the section.
fn section(out: &mut Vec<Text>, top_cm: f32, title: &'static str) {
    out.push(nt(title, NT_COLUMNS[0], top_cm + 0.07, 7.0, true));
}

/// A suppressed block: its message centred in a 0,40 cm (party) or 0,32 cm (ISSQN) band.
fn suppressed(out: &mut Vec<Text>, top_cm: f32, height_cm: f32, message: &'static str) {
    out.push(nt(
        message,
        150.0,
        top_cm + (height_cm - 7.5 / CM) / 2.0,
        7.5,
        false,
    ));
}

/// Variants of the DANFSe of NT SE/CGNFS-e 008 v1.02, as the Receita Federal's open-source
/// generator lays it out (labels, columns and rows of its renderer).
#[derive(Clone, Copy, PartialEq)]
enum Nt008 {
    /// ME/EPP in 2026, every block present, ISS withheld.
    Me2026,
    /// MEI: no recipient nor intermediary (suppressed blocks), no IBS/CBS.
    Mei,
    /// ME/EPP with a 2027 competence: no PIS/COFINS row, and the net value + IBS/CBS differs from
    /// the net value.
    Me2027,
}

fn danfse_nt008(variant: Nt008) -> Vec<Text> {
    use Nt008::*;
    let mei = variant == Mei;
    let mut out = vec![
        nt("DANFSe v2.0", 238.0, 0.35, 9.0, true),
        nt("Documento Auxiliar da NFS-e", 200.0, 0.65, 9.0, true),
        nt("Município: Cidade Exemplo - UF", 442.8, 0.45, 8.0, false),
        nt(
            "Ambiente Gerador: Sefin Nacional NFS-e",
            442.8,
            0.97,
            6.0,
            false,
        ),
        nt("Tipo de Ambiente: Produção", 442.8, 1.22, 6.0, false),
    ];
    let o = &mut out;
    access_field(
        o,
        0,
        1.48,
        "CHAVE DE ACESSO DA NFS-E",
        "33333333333333333333333333333333333333333333333333",
    );
    let (number, competence, issued) = match variant {
        Me2026 => ("41", "10/04/2026", "10/04/2026 10:15:00"),
        Mei => ("7", "15/05/2026", "18/05/2026 09:30:00"),
        Me2027 => ("58", "10/03/2027", "02/04/2027 16:05:00"),
    };
    access_field(o, 0, 2.27, "NÚMERO DA NFS-E", number);
    access_field(o, 1, 2.27, "COMPETÊNCIA DA NFS-E", competence);
    access_field(o, 2, 2.27, "DATA E HORA DA EMISSÃO DA NFS-E", issued);
    access_field(o, 0, 2.96, "NÚMERO DA DPS", number);
    access_field(o, 1, 2.96, "SÉRIE DA DPS", "900");
    access_field(o, 2, 2.96, "DATA E HORA DA EMISSÃO DA DPS", issued);
    // Identification row: 7 pt label 0,06 cm below the top, value at 0,36 cm.
    for (column, label, value) in [
        (0, "EMITENTE DA NFS-e", "Prestador do Serviço"),
        (1, "SITUAÇÃO DA NFS-e", "NFS-e Gerada"),
        (2, "FINALIDADE", "NFS-e Regular"),
    ] {
        o.push(nt(label, NT_COLUMNS[column], 3.71, 7.0, true));
        o.push(nt(value, NT_COLUMNS[column], 4.01, 7.0, false));
    }

    // Provider (with the Simples Nacional row) and taker.
    section(o, 4.34, "PRESTADOR / FORNECEDOR");
    field(o, 1, 4.34, "CNPJ / CPF / NIF", "00.000.000/0001-91");
    field(o, 2, 4.34, "Indicador Municipal (Inscrição)", "-");
    field(o, 3, 4.34, "Telefone", "-");
    field(
        o,
        0,
        4.98,
        "Nome / Nome Empresarial",
        "FULANO DE TAL SERVICOS DIGITAIS",
    );
    field(o, 2, 4.98, "Município / Sigla UF", "Cidade Exemplo - UF");
    field(o, 3, 4.98, "Código IBGE / CEP", "0000000 / 00000-000");
    field(o, 0, 5.62, "Endereço", "Rua Exemplo, 100");
    field(o, 2, 5.62, "E-mail", "-");
    let (simples, regime) = if mei {
        ("2 - Optante - MEI", "-")
    } else {
        (
            "3 - Optante - ME/EPP",
            "1 - Regime de apuração dos tributos federais e municipal pelo SN",
        )
    };
    field(
        o,
        0,
        6.26,
        "Simples Nacional na Data de Competência",
        simples,
    );
    field(o, 2, 6.26, "Regime de Apuração Tributária pelo SN", regime);
    section(o, 6.92, "TOMADOR / ADQUIRENTE");
    field(o, 1, 6.92, "CNPJ / CPF / NIF", "00.000.000/0001-91");
    field(o, 2, 6.92, "Indicador Municipal (Inscrição)", "-");
    field(o, 3, 6.92, "Telefone", "-");
    field(
        o,
        0,
        7.56,
        "Nome / Nome Empresarial",
        "EMPRESA EXEMPLO LTDA",
    );
    field(o, 2, 7.56, "Município / Sigla UF", "Cidade Exemplo - UF");
    field(o, 3, 7.56, "Código IBGE / CEP", "0000000 / 00000-000");
    field(o, 0, 8.20, "Endereço", "Avenida Exemplo, 200");
    field(o, 2, 8.20, "E-mail", "-");

    // Recipient and intermediary: full blocks, or suppressed bands that pull the rest up.
    let service_top = if mei {
        suppressed(
            o,
            8.86,
            0.40,
            "DESTINATÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e",
        );
        suppressed(
            o,
            9.26,
            0.40,
            "INTERMEDIÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e",
        );
        9.66
    } else {
        section(o, 8.86, "DESTINATÁRIO DA OPERAÇÃO");
        field(o, 1, 8.86, "CNPJ / CPF / NIF", "-");
        field(o, 3, 8.86, "Telefone", "-");
        field(o, 0, 9.50, "Nome / Nome Empresarial", "-");
        field(o, 2, 9.50, "Município / Sigla UF", "-");
        field(o, 3, 9.50, "Código IBGE / CEP", "-");
        field(o, 0, 10.14, "Endereço", "-");
        field(o, 2, 10.14, "E-mail", "-");
        section(o, 10.80, "INTERMEDIÁRIO DA OPERAÇÃO");
        field(o, 1, 10.80, "CNPJ / CPF / NIF", "-");
        field(o, 2, 10.80, "Indicador Municipal (Inscrição)", "-");
        field(o, 3, 10.80, "Telefone", "-");
        field(o, 0, 11.44, "Nome / Nome Empresarial", "-");
        field(o, 2, 11.44, "Município / Sigla UF", "-");
        field(o, 3, 11.44, "Código IBGE / CEP", "-");
        field(o, 0, 12.08, "Endereço", "-");
        field(o, 2, 12.08, "E-mail", "-");
        12.74
    };

    // Service.
    let s = service_top;
    section(o, s, "SERVIÇO PRESTADO");
    field(
        o,
        1,
        s,
        "Código de Tributação Nacional / Municipal",
        "01.01.01 / -",
    );
    field(o, 2, s, "Código da NBS", "1.1502.10.00");
    field(
        o,
        3,
        s,
        "Local da Prestação / Sigla UF / País",
        "Cidade Exemplo - UF",
    );
    o.push(nt(
        "01.01.01 - Análise e desenvolvimento de sistemas.",
        NT_COLUMNS[0],
        s + 0.65,
        7.0,
        false,
    ));
    o.push(nt(
        "Descrição do Serviço",
        NT_COLUMNS[0],
        s + 1.05,
        6.0,
        true,
    ));
    o.push(nt(
        "Desenvolvimento de software sob encomenda.",
        NT_COLUMNS[0],
        s + 1.35,
        7.0,
        false,
    ));

    // Municipal (ISSQN), 2,59 cm high.
    let m = s + 1.69;
    section(o, m, "TRIBUTAÇÃO MUNICIPAL (ISSQN)");
    field(
        o,
        1,
        m,
        "Tipo de Tributação do ISSQN",
        "Operação Tributável",
    );
    field(
        o,
        2,
        m,
        "Município / Sigla UF / País de Incidência do ISSQN",
        "Cidade Exemplo - UF",
    );
    field(
        o,
        0,
        m + 0.65,
        "Regime Especial de Tributação do ISSQN",
        "Nenhum",
    );
    field(o, 1, m + 0.65, "Tipo de Imunidade do ISSQN", "-");
    field(o, 2, m + 0.65, "Suspensão da Exigibilidade do ISSQN", "Não");
    field(o, 3, m + 0.65, "Número Processo Suspensão", "-");
    field(o, 0, m + 1.30, "Benefício Municipal", "-");
    field(o, 1, m + 1.30, "Cálculo do BM", "-");
    field(o, 2, m + 1.30, "Total Deduções/Reduções", "-");
    field(o, 3, m + 1.30, "Desconto Incondicionado", "-");
    let (base, issqn, withheld) = if mei {
        ("-", "-", "Não Retido")
    } else {
        ("R$ 1.300,00", "R$ 26,00", "Retido pelo Tomador")
    };
    field(o, 0, m + 1.95, "BC ISSQN", base);
    field(
        o,
        1,
        m + 1.95,
        "Alíquota Aplicada",
        if mei { "-" } else { "2,00%" },
    );
    field(o, 2, m + 1.95, "Retenção do ISSQN", withheld);
    field(o, 3, m + 1.95, "ISSQN Apurado", issqn);

    // Federal (except CBS); the PIS/COFINS row is gone from 2027.
    let f = m + 2.59;
    section(o, f, "TRIBUTAÇÃO FEDERAL (EXCETO CBS)");
    field(o, 1, f, "IRRF", "-");
    field(o, 2, f, "Contribuição Previdenciária - Retida", "-");
    field(o, 3, f, "Contribuições Sociais - Retidas", "-");
    if variant != Me2027 {
        field(o, 0, f + 0.65, "PIS - Débito Apuração Própria", "-");
        field(o, 1, f + 0.65, "COFINS - Débito Apuração Própria", "-");
        field(o, 2, f + 0.65, "Descrição Contrib. Sociais - Retidas", "-");
    }

    // IBS / CBS: every field present, '-' when empty (always for the MEI).
    let i = f + 1.30;
    let dash = |value: &'static str| if mei { "-" } else { value };
    section(o, i, "TRIBUTAÇÃO IBS / CBS");
    field(o, 1, i, "CST / cClassTrib", dash("000 / 000001"));
    field(
        o,
        2,
        i,
        "Indicador de Operação / Código IBGE Incidência / Município Incidência / Sigla UF",
        dash("100301 / 0000000 / Cidade Exemplo / UF"),
    );
    field(
        o,
        0,
        i + 0.64,
        "Exclusões e Reduções da Base de Cálculo",
        dash("R$ 26,00"),
    );
    field(
        o,
        1,
        i + 0.64,
        "Base de Cálculo Após Exclusões e Reduções",
        dash("R$ 1.274,00"),
    );
    field(
        o,
        2,
        i + 0.64,
        "Red. Alíquota IBS / Red. Alíquota CBS",
        dash("0,00% / 0,00%"),
    );
    field(
        o,
        3,
        i + 0.64,
        "Alíquota IBS UF / IBS Mun",
        dash("0,10% / 0,00%"),
    );
    field(
        o,
        0,
        i + 1.29,
        "Alíq. Efetiva Municipal - IBS",
        dash("0,00%"),
    );
    field(o, 1, i + 1.29, "Valor Apurado Municipal - IBS", "-");
    field(
        o,
        2,
        i + 1.29,
        "Alíq. Efetiva Estadual - IBS",
        dash("0,10%"),
    );
    field(
        o,
        3,
        i + 1.29,
        "Valor Apurado Estadual - IBS",
        dash("R$ 1,27"),
    );
    field(o, 0, i + 1.94, "Valor Total Apurado - IBS", dash("R$ 1,27"));
    field(o, 1, i + 1.94, "Alíquota - CBS", dash("0,90%"));
    field(o, 2, i + 1.94, "Alíquota Efetiva - CBS", dash("0,90%"));
    field(
        o,
        3,
        i + 1.94,
        "Valor Total Apurado - CBS",
        dash("R$ 11,47"),
    );

    // Totals. In 2026 the net value + IBS/CBS equals the net value; from 2027 it adds them.
    let t = i + 2.58;
    let (service, retained, net, ibs_cbs, net_plus) = match variant {
        Me2026 => (
            "R$ 1.300,00",
            "R$ 26,00",
            "R$ 1.274,00",
            "R$ 12,74",
            "R$ 1.274,00",
        ),
        Mei => ("R$ 800,00", "-", "R$ 800,00", "-", "R$ 800,00"),
        Me2027 => (
            "R$ 1.300,00",
            "R$ 26,00",
            "R$ 1.274,00",
            "R$ 12,74",
            "R$ 1.286,74",
        ),
    };
    section(o, t, "VALOR TOTAL DA NFS-E");
    field(o, 1, t, "VALOR DA OPERAÇÃO / SERVIÇO", service);
    field(o, 2, t, "Desconto Incondicionado", "-");
    field(o, 3, t, "Desconto Condicionado", "-");
    field(
        o,
        0,
        t + 0.69,
        "Total das Retenções (ISSQN / Federais)",
        retained,
    );
    field(o, 1, t + 0.69, "VALOR LÍQUIDO DA NFS-e", net);
    field(o, 2, t + 0.69, "Total do IBS/CBS", ibs_cbs);
    field(o, 3, t + 0.69, "VALOR LÍQUIDO DA NFS-e + IBS/CBS", net_plus);

    let c = t + 1.37;
    section(o, c, "INFORMAÇÕES COMPLEMENTARES");
    o.push(nt(
        "Totais aproximados dos tributos: R$ 0,00",
        NT_COLUMNS[0],
        c + 0.41,
        6.5,
        false,
    ));
    out
}

fn legacy_municipal() -> Vec<Text> {
    vec![
        b("PREFEITURA MUNICIPAL DE CIDADE EXEMPLO", 40.0, 30.0, 11.0),
        t(
            "NOTA FISCAL DE SERVIÇOS ELETRÔNICA - NFS-e",
            40.0,
            46.0,
            9.0,
        ),
        t("Número da Nota: 000123", 420.0, 30.0, 8.0),
        t("Data de Emissão: 15/04/2026", 420.0, 44.0, 8.0),
        t(
            "Prestador: FULANO DE TAL SERVICOS DIGITAIS",
            40.0,
            80.0,
            8.0,
        ),
        t("Tomador: EMPRESA EXEMPLO LTDA", 40.0, 94.0, 8.0),
        b("Discriminação dos serviços", 40.0, 120.0, 8.0),
        t("Consultoria em tecnologia da informação.", 40.0, 134.0, 8.0),
        t("Valor dos Serviços: R$ 2.650,00", 40.0, 700.0, 9.0),
        t("Deduções: R$ 150,00", 40.0, 716.0, 9.0),
        b("Valor Total da Nota: R$ 2.500,00", 40.0, 740.0, 10.0),
    ]
}

fn unusual(shift: f32, amount: &'static str) -> Vec<Text> {
    vec![
        b("RECIBO DE PRESTAÇÃO DE SERVIÇOS", 40.0, 40.0, 12.0),
        t("Emitido por: FULANO DE TAL", 40.0, 60.0, 8.0),
        t("Referência: desenvolvimento de software", 40.0, 90.0, 8.0),
        b("Montante a pagar", 300.0, 400.0 + shift, 8.0),
        t(amount, 300.0, 410.0 + shift, 9.0),
        t(
            "Observações: pagamento via transferência.",
            40.0,
            460.0 + shift,
            8.0,
        ),
        t(
            "Taxa de serviço incluída: R$ 50,00",
            40.0,
            480.0 + shift,
            8.0,
        ),
    ]
}

/// How the page is shown.
#[derive(Clone, Copy, PartialEq)]
enum Orientation {
    /// Plain portrait page.
    Upright,
    /// Landscape page as generators usually make them: the page has /Rotate 90 and its content
    /// is drawn rotated so that it reads upright on screen.
    Landscape,
    /// A page with /Rotate 90 whose content was not rotated: text shows sideways on screen.
    Sideways,
}

fn write_pdf(
    pdfium: &Pdfium,
    path: &Path,
    texts: &[Text],
    orientation: Orientation,
) -> Result<(), PdfiumError> {
    let mut document = pdfium.create_new_pdf()?;
    let regular = document.fonts_mut().helvetica();
    let bold = document.fonts_mut().helvetica_bold();
    {
        let mut page = document
            .pages_mut()
            .create_page_at_end(PdfPagePaperSize::a4())?;
        for text in texts {
            let font = if text.bold { bold } else { regular };
            if orientation == Orientation::Landscape {
                // Shown rotated 90 degrees clockwise, so draw the text rotated 90 degrees
                // counter-clockwise: page x = display y (baseline), page y = display x.
                let mut object = page.objects_mut().create_text_object(
                    PdfPoints::ZERO,
                    PdfPoints::ZERO,
                    text.text,
                    font,
                    PdfPoints::new(text.size),
                )?;
                let baseline = text.top + 0.8 * text.size;
                object.transform(0.0, 1.0, -1.0, 0.0, baseline, text.x)?;
            } else if text.angle != 0.0 {
                let mut object = page.objects_mut().create_text_object(
                    PdfPoints::ZERO,
                    PdfPoints::ZERO,
                    text.text,
                    font,
                    PdfPoints::new(text.size),
                )?;
                let (sin, cos) = text.angle.to_radians().sin_cos();
                let baseline = PAGE_HEIGHT - text.top - 0.8 * text.size;
                object.transform(cos, sin, -sin, cos, text.x, baseline)?;
            } else {
                // PDFium places text by its baseline, measured from the bottom of the page.
                let baseline = PAGE_HEIGHT - text.top - 0.8 * text.size;
                page.objects_mut().create_text_object(
                    PdfPoints::new(text.x),
                    PdfPoints::new(baseline),
                    text.text,
                    font,
                    PdfPoints::new(text.size),
                )?;
            }
        }
        if orientation != Orientation::Upright {
            page.set_rotation(PdfPageRenderRotation::Degrees90);
        }
    }
    document.save_to_file(path)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine = Engine::new(&[dev_pdfium_dir()])?;
    let _pdfium = engine.lock();
    let pdfium = engine.pdfium();

    let dir = root.join("tests/fixtures");
    fs::create_dir_all(dir.join("sub"))?;

    use Orientation::*;
    write_pdf(
        pdfium,
        &dir.join("danfse-v1.pdf"),
        &danfse_v1("R$3.210,98"),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("danfse-v2.pdf"),
        &danfse_v2("R$1.234,56"),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("danfse-nt008.pdf"),
        &danfse_nt008(Nt008::Me2026),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("danfse-nt008-mei.pdf"),
        &danfse_nt008(Nt008::Mei),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("danfse-nt008-2027.pdf"),
        &danfse_nt008(Nt008::Me2027),
        Upright,
    )?;
    let mut cancelled = danfse_nt008(Nt008::Me2026);
    cancelled.push(watermark("CANCELADA"));
    write_pdf(
        pdfium,
        &dir.join("danfse-nt008-watermark.pdf"),
        &cancelled,
        Upright,
    )?;
    let mut substituted = danfse_nt008(Nt008::Mei);
    substituted.push(watermark("SUBSTITUÍDA"));
    write_pdf(
        pdfium,
        &dir.join("danfse-nt008-watermark-sub.pdf"),
        &substituted,
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("legacy-municipal.pdf"),
        &legacy_municipal(),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("unusual.pdf"),
        &unusual(0.0, "R$ 845,10"),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("unusual-shifted.pdf"),
        &unusual(40.0, "R$ 912,45"),
        Upright,
    )?;
    write_pdf(pdfium, &dir.join("blank.pdf"), &[], Upright)?;
    write_pdf(
        pdfium,
        &dir.join("nota-cancelada.pdf"),
        &danfse_v1("R$999,99"),
        Upright,
    )?;
    write_pdf(
        pdfium,
        &dir.join("rotated.pdf"),
        &danfse_v2("R$555,55"),
        Landscape,
    )?;
    write_pdf(
        pdfium,
        &dir.join("sideways.pdf"),
        &danfse_v2("R$444,44"),
        Sideways,
    )?;
    write_pdf(
        pdfium,
        &dir.join("sub/danfse-v2-sub.pdf"),
        &danfse_v2("R$777,77"),
        Upright,
    )?;
    fs::copy(dir.join("danfse-v1.pdf"), dir.join("danfse-v1-copia.pdf"))?;
    fs::write(
        dir.join("corrupt.pdf"),
        b"%PDF-1.7\nthis is not really a pdf\n",
    )?;

    println!("Fixtures written to {}", dir.display());
    Ok(())
}
