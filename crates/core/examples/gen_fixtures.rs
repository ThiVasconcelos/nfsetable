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

/// A text drawn at `x` with its top at `top` (points, top-left origin).
struct Text {
    text: &'static str,
    x: f32,
    top: f32,
    size: f32,
    bold: bool,
}

const fn t(text: &'static str, x: f32, top: f32, size: f32) -> Text {
    Text {
        text,
        x,
        top,
        size,
        bold: false,
    }
}

const fn b(text: &'static str, x: f32, top: f32, size: f32) -> Text {
    Text {
        text,
        x,
        top,
        size,
        bold: true,
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
