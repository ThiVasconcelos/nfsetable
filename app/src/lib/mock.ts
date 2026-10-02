// Synthetic backend used when the UI runs in a normal browser (npm run dev, screenshots).
//
// EVERYTHING HERE IS FICTITIOUS: company names, people, CNPJ/CPF numbers, cities and amounts are
// made up for demonstration. The documents are described as text boxes in PDF points (top-left
// origin, A4 = 595 x 842), and that single model drives the SVG page renders, read_region,
// test_rule and the extraction with saved profiles, so the whole "fix a failed file" flow can be
// exercised end to end without Tauri. The tax report is a simplified computation (./mock-tax.ts).

import type { Backend, DragDropState, Unlisten } from './api'
import type {
  AppInfo,
  DocKind,
  DocResult,
  ExportFormat,
  ExportRequest,
  ExportRow,
  FieldValue,
  Origin,
  Profile,
  Progress,
  Rect,
  RegionAnchor,
  RegionRead,
  RegionRule,
  RenderedPage,
  Rule,
  RuleTest,
  ScanOptions,
  ScanResult,
  ScannedFile,
  SourceInfo,
  TaxCatalog,
  TaxInput,
  TaxReport,
} from './types'
import {
  baseName,
  compactText,
  formatBRLPlain,
  formatMonth,
  isPdfPath,
  normalizeText,
  pathKey,
  slugify,
  trimTrailingSeparators,
} from './format'
import { EXPORT_FORMATS } from './labels'
import { MOCK_TAX_CATALOG, mockTaxReport } from './mock-tax'
import { nameMatches } from './rules'
import { clearAll } from './storage'

const PAGE_W = 595
const PAGE_H = 842
const FONT = "Helvetica, Arial, 'Liberation Sans', sans-serif"

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms))
const jitter = (base: number, spread: number) => base + Math.round(Math.random() * spread)

// ---------------------------------------------------------------- page model

interface TextItem {
  text: string
  x: number
  y: number
  w: number
  h: number
  size: number
  bold: boolean
  color: string
}

type Shape =
  | { kind: 'rect'; x: number; y: number; w: number; h: number; fill?: string; stroke?: string; r?: number }
  | { kind: 'line'; x1: number; y1: number; x2: number; y2: number; stroke?: string; width?: number }
  | { kind: 'qr'; x: number; y: number; size: number; seed: number }
  | { kind: 'smudge'; x: number; y: number; w: number; h: number }

interface MockPage {
  items: TextItem[]
  shapes: Shape[]
  scanned?: boolean
}

/** Approximate Helvetica advance widths (em units). */
function charWidth(c: string): number {
  if (c >= '0' && c <= '9') return 0.556
  if (c === ' ') return 0.278
  if ('.,:;'.includes(c)) return 0.278
  if ("iljI!|'".includes(c)) return 0.24
  if ('ft()[]/-'.includes(c)) return 0.333
  if (c === 'm' || c === 'w') return 0.8
  if (c === 'M' || c === 'W') return 0.86
  if (c === '$') return 0.556
  if (c === '%') return 0.889
  if (c !== c.toLowerCase()) return 0.68
  return 0.52
}

function measure(text: string, size: number, bold: boolean): number {
  let em = 0
  for (const c of text) em += charWidth(c)
  return Math.round(em * size * (bold ? 1.06 : 1) * 10) / 10
}

class PageBuilder {
  items: TextItem[] = []
  shapes: Shape[] = []

  text(x: number, y: number, text: string, size = 8, opts: { bold?: boolean; color?: string } = {}): TextItem {
    const bold = opts.bold ?? false
    const item: TextItem = {
      text,
      x,
      y,
      w: measure(text, size, bold),
      h: Math.round(size * 1.15 * 10) / 10,
      size,
      bold,
      color: opts.color ?? '#1d1d1f',
    }
    this.items.push(item)
    return item
  }

  /** Small grey field label. */
  label(x: number, y: number, text: string): TextItem {
    return this.text(x, y, text, 6, { color: '#5b5b60' })
  }

  /** Text placed right after another item on the same line. */
  after(prev: TextItem, text: string, size = prev.size, opts: { bold?: boolean; color?: string } = {}): TextItem {
    return this.text(prev.x + prev.w + 4, prev.y, text, size, opts)
  }

  box(x: number, y: number, w: number, h: number, opts: { fill?: string; stroke?: string; r?: number } = {}) {
    this.shapes.push({ kind: 'rect', x, y, w, h, stroke: '#b9bcc4', ...opts })
  }

  section(y: number, h: number, title: string) {
    this.box(20, y, 555, h)
    this.text(28, y + 5, title, 7, { bold: true, color: '#2b2f3a' })
  }

  line(x1: number, y1: number, x2: number, y2: number, stroke = '#b9bcc4', width = 0.7) {
    this.shapes.push({ kind: 'line', x1, y1, x2, y2, stroke, width })
  }

  footer(index: number, count: number, left: string) {
    this.text(20, 812, left, 6.5, { color: '#77777c' })
    this.text(522, 812, `Página ${index + 1} de ${count}`, 6.5, { color: '#77777c' })
  }

  build(scanned = false): MockPage {
    return { items: this.items, shapes: this.shapes, scanned }
  }
}


function isMoneyText(text: string): boolean {
  return /^-?\s*R\$\s*-?\s*\d{1,3}(\.\d{3})*,\d{2}$|^-?\s*R\$\s*-?\s*\d+,\d{2}$/.test(text.trim())
}

function parseMoney(text: string): number | null {
  const m = /(-)?\s*R?\$?\s*(-)?\s*(\d{1,3}(?:\.\d{3})+|\d+),(\d{2})/.exec(text)
  if (!m) return null
  const cents = Number(m[3].replaceAll('.', '')) * 100 + Number(m[4])
  return m[1] || m[2] ? -cents : cents
}

// ---------------------------------------------------------------- synthetic parties

interface Party {
  name: string
  doc: string
  city: string
  address: string
  email: string
}

const PROVIDER: Party = {
  name: 'EMPRESA EXEMPLO LTDA',
  doc: '00.000.000/0001-91',
  city: 'Cidade Exemplo - UF',
  address: 'Rua das Flores, 100 - Centro',
  email: 'contato@empresa-exemplo.example',
}

const client = (name: string, doc: string): Party => ({
  name,
  doc,
  city: 'Cidade Exemplo - UF',
  address: 'Avenida Principal, 2000 - Sala 5',
  email: 'financeiro@cliente.example',
})

const ALFA = client('CLIENTE ALFA SERVIÇOS LTDA', '11.111.111/0001-11')
const BETA = client('CLIENTE BETA COMÉRCIO LTDA', '22.222.222/0001-22')
const GAMA = client('CLIENTE GAMA TECNOLOGIA S.A.', '33.333.333/0001-33')
const DELTA = client('CLIENTE DELTA CONSULTORIA ME', '44.444.444/0001-44')
const EPSILON = client('CLIENTE ÉPSILON EDUCAÇÃO LTDA', '55.555.555/0001-55')
const ZETA = client('CLIENTE ZETA LOGÍSTICA LTDA', '66.666.666/0001-66')
const FULANO = client('Fulano de Tal', '000.000.000-00')
/** Suppliers of the expense documents (the company is the one who pays). */
const CONTADOR: Party = {
  name: 'CONTABILIDADE EXEMPLO LTDA',
  doc: '77.777.777/0001-77',
  city: 'Cidade Exemplo - UF',
  address: 'Rua dos Contadores, 45 - Sala 3',
  email: 'atendimento@contabilidade.example',
}

const SERVICE_LINES = [
  'Serviços de desenvolvimento e manutenção de sistemas sob demanda,',
  'incluindo suporte técnico remoto e reuniões de acompanhamento.',
  'Período de referência conforme competência informada.',
]

// ---------------------------------------------------------------- layouts

interface Built {
  pages: MockPage[]
  value: TextItem | null
  valuePage: number
  /** Gross value ("Valor do serviço"), when the layout has one. */
  service?: TextItem
  /** Date the competence heuristics would read (ISO), and where it is printed. */
  competence?: { item: TextItem; date: string }
}

/** "07/2026" -> "2026-07-01"; "12/07/2026 ..." -> "2026-07-12". */
function isoDate(text: string): string {
  const full = /(\d{2})\/(\d{2})\/(\d{4})/.exec(text)
  if (full) return `${full[3]}-${full[2]}-${full[1]}`
  const month = /(\d{2})\/(\d{4})/.exec(text)
  return month ? `${month[2]}-${month[1]}-01` : ''
}

/**
 * DANFSe-like layout ("padrão nacional"); v1 = until 2025, v2 = 2026 with the IBS/CBS block.
 * `competence` is "MM/AAAA" (printed as 01/MM/AAAA); `retained` is withheld from the gross value, so
 * the net value is `cents` and the service value `cents + retained`.
 */
function danfse(o: {
  version: 1 | 2
  number: string
  issued: string
  competence: string
  client: Party
  /** Who issued the note (default: the company; e.g. the accountant for an expense). */
  emitter?: Party
  cents: number
  retained?: number
  pages?: number
  seed: number
}): Built {
  const retained = o.retained ?? 0
  const serviceCents = o.cents + retained
  const count = o.pages ?? 1
  const p = new PageBuilder()

  p.box(20, 20, 555, 62)
  p.box(28, 28, 46, 46, { fill: '#e9edf7', stroke: '#c9d1e6', r: 4 })
  p.text(36, 45, 'NFS-e', 9, { bold: true, color: '#34437a' })
  p.text(84, 28, `DANFSe v${o.version}.0`, 13, { bold: true })
  p.text(84, 46, 'Documento Auxiliar da NFS-e', 8)
  p.text(84, 59, 'Nota Fiscal de Serviço eletrônica', 7, { color: '#5b5b60' })
  p.label(300, 30, 'Número da NFS-e')
  p.text(300, 38, o.number, 8)
  p.label(300, 54, 'Competência da NFS-e')
  const competence = p.text(300, 62, `01/${o.competence}`, 8)
  p.label(404, 30, 'Data e Hora da emissão')
  p.text(404, 38, o.issued, 8)
  p.label(404, 54, 'Situação')
  p.text(404, 62, 'Normal', 8)
  p.shapes.push({ kind: 'qr', x: 521, y: 26, size: 50, seed: o.seed })

  const party = (y: number, title: string, who: Party, withAddress: boolean) => {
    p.section(y, 62, title)
    p.label(28, y + 18, 'Nome / Nome Empresarial')
    p.text(28, y + 26, who.name, 8)
    p.label(330, y + 18, 'CNPJ / CPF / NIF')
    p.text(330, y + 26, who.doc, 8)
    p.label(28, y + 40, withAddress ? 'Endereço' : 'E-mail')
    p.text(28, y + 48, withAddress ? who.address : who.email, 8)
    p.label(330, y + 40, 'Município')
    p.text(330, y + 48, who.city, 8)
  }
  party(90, 'EMITENTE DA NFS-e', o.emitter ?? PROVIDER, true)
  party(160, 'TOMADOR DO SERVIÇO', o.client, false)

  p.section(230, 92, 'SERVIÇO PRESTADO')
  p.label(28, 248, 'Código de Tributação Nacional')
  p.text(28, 256, '01.07.01 - Suporte técnico em informática', 8)
  p.label(28, 272, 'Descrição do Serviço')
  SERVICE_LINES.forEach((line, i) => p.text(28, 281 + i * 11, line, 7.5))

  p.section(330, 54, 'TRIBUTAÇÃO MUNICIPAL')
  p.label(28, 348, 'Tributação do ISSQN')
  p.text(28, 356, 'Operação Tributável', 8)
  p.label(170, 348, 'Município de Incidência do ISSQN')
  p.text(170, 356, PROVIDER.city, 8)
  p.label(330, 348, 'Regime Especial de Tributação')
  p.text(330, 356, 'Nenhum', 8)
  p.label(470, 348, 'Optante pelo Simples Nacional')
  p.text(470, 356, 'Sim', 8)

  let valuesY = 392
  if (o.version === 2) {
    p.section(392, 54, 'IBS / CBS')
    p.label(28, 410, 'Situação Tributária')
    p.text(28, 418, '000 - Tributação integral', 8)
    p.label(200, 410, 'Alíquota IBS')
    p.text(200, 418, '0,10%', 8)
    p.label(330, 410, 'Alíquota CBS')
    p.text(330, 418, '0,90%', 8)
    valuesY = 454
  }

  p.section(valuesY, 60, 'VALOR TOTAL DA NFS-e')
  const labelsY = valuesY + 20
  const amountsY = valuesY + 29
  let value: TextItem
  let service: TextItem
  if (o.version === 2) {
    p.text(28, labelsY, 'Valor do Serviço', 6, { color: '#5b5b60' })
    p.text(150, labelsY, 'Total das Retenções (ISSQN/Federais)', 6, { color: '#5b5b60' })
    p.text(318, labelsY, 'VALOR LÍQUIDO DA NFS-e', 6, { bold: true, color: '#2b2f3a' })
    p.text(440, labelsY, 'VALOR LÍQUIDO DA NFS-e + IBS/CBS', 6, { bold: true, color: '#2b2f3a' })
    service = p.text(28, amountsY, formatBRLPlain(serviceCents), 8)
    p.text(150, amountsY, retained ? formatBRLPlain(retained) : '-', 8)
    value = p.text(318, amountsY, formatBRLPlain(o.cents), 8, { bold: true })
    p.text(440, amountsY, formatBRLPlain(0), 8)
  } else {
    p.text(28, labelsY, 'Valor do Serviço', 6, { color: '#5b5b60' })
    p.text(150, labelsY, 'Desconto Incondicionado', 6, { color: '#5b5b60' })
    p.text(300, labelsY, 'Total Retenções', 6, { color: '#5b5b60' })
    p.text(439.4, labelsY, 'Valor Líquido da NFS-e', 6, { bold: true, color: '#2b2f3a' })
    service = p.text(28, amountsY, formatBRLPlain(serviceCents), 8)
    p.text(150, amountsY, formatBRLPlain(0), 8)
    p.text(300, amountsY, formatBRLPlain(retained), 8)
    value = p.text(439.4, amountsY, formatBRLPlain(o.cents), 8, { bold: true })
  }

  const infoY = valuesY + 68
  p.section(infoY, 80, 'INFORMAÇÕES COMPLEMENTARES')
  ;[
    'Documento emitido por ME ou EPP optante pelo Simples Nacional.',
    'Não gera direito a crédito fiscal de IPI.',
    'Documento fictício gerado para demonstração do nfsetable.',
  ].forEach((line, i) => p.text(28, infoY + 20 + i * 11, line, 7.5))
  p.footer(0, count, 'DANFSe de demonstração · dados fictícios')

  const pages = [p.build()]
  for (let i = 1; i < count; i++) {
    const c = new PageBuilder()
    c.text(20, 24, `DANFSe v${o.version}.0 · continuação`, 10, { bold: true })
    c.text(20, 40, `NFS-e ${o.number}`, 8, { color: '#5b5b60' })
    c.section(60, 300, 'INFORMAÇÕES COMPLEMENTARES (continuação)')
    for (let l = 0; l < 20; l++) {
      c.text(28, 80 + l * 13, `Item ${l + 1}: atividade de acompanhamento técnico registrada no período.`, 7.5)
    }
    c.footer(i, count, 'DANFSe de demonstração · dados fictícios')
    pages.push(c.build())
  }
  return { pages, value, valuePage: 0, service, competence: { item: competence, date: isoDate(competence.text) } }
}

/**
 * Legacy municipal layout: "Valor Total da Nota: R$ ..." on one line (found by the heuristic). The
 * issue date gives the competence, unless it is printed in words (`longDate`), which no heuristic
 * reads.
 */
function municipal(o: { number: string; issued: string; client: Party; cents: number; longDate?: string }): Built {
  const p = new PageBuilder()
  p.box(20, 20, 555, 74)
  p.box(30, 30, 46, 46, { fill: '#eef0e8', stroke: '#c8ccb9', r: 23 })
  p.text(40, 48, 'BRASÃO', 6.5, { bold: true, color: '#5d6150' })
  p.text(86, 30, 'PREFEITURA MUNICIPAL DE CIDADE EXEMPLO', 11, { bold: true })
  p.text(86, 47, 'Secretaria Municipal de Finanças', 8)
  p.text(86, 62, 'NOTA FISCAL DE SERVIÇOS ELETRÔNICA - NFS-e', 9, { bold: true, color: '#2b2f3a' })
  p.label(452, 30, 'Número da Nota')
  p.text(452, 38, o.number, 11, { bold: true })
  p.label(452, 58, o.longDate ? 'Emissão' : 'Data e Hora de Emissão')
  const issued = p.text(452, 66, o.longDate ?? o.issued, 8)

  const block = (y: number, title: string, who: Party) => {
    p.section(y, 58, title)
    p.text(28, y + 18, `Razão Social: ${who.name}`, 8)
    p.text(28, y + 30, `CNPJ/CPF: ${who.doc}`, 8)
    p.text(300, y + 30, 'Inscrição Municipal: 000.000-0', 8)
    p.text(28, y + 42, `Endereço: ${who.address} - ${who.city}`, 8)
  }
  block(102, 'PRESTADOR DE SERVIÇOS', PROVIDER)
  block(168, 'TOMADOR DE SERVIÇOS', o.client)

  p.section(234, 250, 'DISCRIMINAÇÃO DOS SERVIÇOS')
  SERVICE_LINES.forEach((line, i) => p.text(28, 254 + i * 12, line, 8))

  p.box(20, 492, 555, 36, { fill: '#f4f4f1' })
  const label = p.text(28, 504, 'Valor Total da Nota:', 10, { bold: true })
  const value = p.after(label, formatBRLPlain(o.cents), 10, { bold: true })

  p.section(536, 80, 'OUTRAS INFORMAÇÕES')
  p.text(28, 556, 'Código de verificação: AB12-CD34', 8)
  p.text(28, 568, 'Documento fictício gerado para demonstração do nfsetable.', 8)
  p.footer(0, 1, 'Prefeitura de Cidade Exemplo · dados fictícios')
  const competence = o.longDate ? undefined : { item: issued, date: isoDate(issued.text) }
  return { pages: [p.build()], value, valuePage: 0, competence }
}

/** Unusual invoice: the value sits next to "Montante a pagar" (unknown label -> notFound). */
function unusualInvoice(o: { number: string; issued: string; client: Party; cents: number; shift: number }): Built {
  const p = new PageBuilder()
  p.text(40, 40, 'FATURA DE SERVIÇOS', 18, { bold: true })
  p.text(478, 46, `Nº ${o.number}`, 11, { bold: true })
  const issued = p.text(40, 68, `Emitida em ${o.issued} · Vencimento em 30 dias`, 8, { color: '#5b5b60' })
  p.line(40, 88, 555, 88)
  p.text(40, 102, 'PRESTADOR', 7, { bold: true, color: '#5b5b60' })
  p.text(40, 114, PROVIDER.name, 9)
  p.text(40, 127, `CNPJ ${PROVIDER.doc}`, 8)
  p.text(40, 139, PROVIDER.city, 8)
  p.text(320, 102, 'CLIENTE', 7, { bold: true, color: '#5b5b60' })
  p.text(320, 114, o.client.name, 9)
  p.text(320, 127, o.client.doc, 8)
  p.text(320, 139, o.client.city, 8)

  p.box(40, 180, 515, 18, { fill: '#eef1f6', stroke: '#eef1f6' })
  p.text(48, 185, 'Descrição', 8, { bold: true })
  p.text(420, 185, 'Horas', 8, { bold: true })
  p.text(480, 185, 'Período', 8, { bold: true })
  ;[
    ['Desenvolvimento de software sob demanda', '40'],
    ['Reuniões de acompanhamento', '6'],
    ['Suporte e manutenção', '10'],
  ].forEach(([desc, hours], i) => {
    const y = 206 + i * 16
    p.text(48, y, desc, 8)
    p.text(420, y, hours, 8)
    p.text(480, y, '07/2026', 8)
  })
  p.line(40, 256, 555, 256)

  const y = 592 + o.shift
  p.box(300, y, 255, 34, { fill: '#f4f6fa', stroke: '#dfe3ec', r: 3 })
  p.text(312, y + 12, 'Montante a pagar', 10, { bold: true })
  const value = p.text(455, y + 11, formatBRLPlain(o.cents), 11, { bold: true })

  p.text(40, 668, 'Observações', 7, { bold: true, color: '#5b5b60' })
  p.text(40, 680, 'Pagamento por transferência até o vencimento.', 8)
  p.text(40, 692, 'Documento fictício gerado para demonstração do nfsetable.', 8)
  p.footer(0, 1, 'Fatura de demonstração · dados fictícios')
  return { pages: [p.build()], value, valuePage: 0, competence: { item: issued, date: isoDate(issued.text) } }
}

/** Receipt whose value is inside a sentence (no label at all -> notFound). */
function receipt(o: { cents: number; words: string }): Built {
  const p = new PageBuilder()
  p.text(252, 60, 'RECIBO', 20, { bold: true })
  p.text(268, 90, 'Nº 07/2026', 9, { color: '#5b5b60' })
  const lead = p.text(60, 160, `Recebi de ${FULANO.name} a importância de`, 10)
  const value = p.after(lead, formatBRLPlain(o.cents), 10, { bold: true })
  p.text(60, 178, `(${o.words}), referente a serviços de consultoria prestados em julho de 2026.`, 10)
  p.text(300, 250, 'Cidade Exemplo - UF, 31 de julho de 2026.', 10)
  p.line(300, 330, 540, 330, '#77777c')
  p.text(340, 336, PROVIDER.name, 9)
  p.text(350, 348, `CNPJ ${PROVIDER.doc}`, 8, { color: '#5b5b60' })
  p.footer(0, 1, 'Recibo de demonstração · dados fictícios')
  return { pages: [p.build()], value, valuePage: 0 }
}

/**
 * Simple receipt with "Valor total:" (found by the heuristic). A numeric reference ("06/2026")
 * gives the competence; one in words ("julho de 2026") is not read.
 */
function rentReceipt(o: { cents: number; reference?: string }): Built {
  const p = new PageBuilder()
  p.text(40, 44, 'RECIBO DE ALUGUEL', 16, { bold: true })
  p.text(40, 80, `Locador: ${FULANO.name}`, 9)
  p.text(40, 94, `Locatário: ${PROVIDER.name}`, 9)
  p.text(40, 108, 'Imóvel: Sala 12, Rua Exemplo, 200 - Cidade Exemplo - UF', 9)
  const reference = p.text(40, 122, `Referência: ${o.reference ?? 'julho de 2026'}`, 9)
  p.box(40, 150, 515, 30, { fill: '#f5f5f7', stroke: '#e0e0e6', r: 3 })
  const label = p.text(50, 160, 'Valor total:', 11, { bold: true })
  const value = p.after(label, formatBRLPlain(o.cents), 11, { bold: true })
  p.text(40, 210, 'Documento fictício gerado para demonstração do nfsetable.', 8, { color: '#5b5b60' })
  p.footer(0, 1, 'Recibo de demonstração · dados fictícios')
  const date = isoDate(reference.text)
  return { pages: [p.build()], value, valuePage: 0, competence: date ? { item: reference, date } : undefined }
}

/** Monthly bill of a supplier (internet, phone...): "Valor total:" and "Referência: MM/AAAA". */
function bill(o: { supplier: string; doc: string; title: string; number: string; reference: string; cents: number; items: [string, number][] }): Built {
  const p = new PageBuilder()
  p.box(20, 20, 555, 70, { fill: '#f3f6fb', stroke: '#dbe3f0', r: 4 })
  p.text(36, 34, o.supplier, 14, { bold: true, color: '#23407a' })
  p.text(36, 54, `CNPJ ${o.doc} · ${o.title}`, 8, { color: '#5b5b60' })
  p.text(430, 34, `Fatura nº ${o.number}`, 9, { bold: true })
  const reference = p.text(430, 50, `Referência: ${o.reference}`, 9)
  p.text(430, 64, 'Vencimento: dia 10', 8, { color: '#5b5b60' })
  p.text(36, 110, 'CLIENTE', 7, { bold: true, color: '#5b5b60' })
  p.text(36, 122, PROVIDER.name, 9)
  p.text(36, 135, `CNPJ ${PROVIDER.doc}`, 8)
  p.box(36, 170, 523, 18, { fill: '#eef1f6', stroke: '#eef1f6' })
  p.text(44, 175, 'Descrição', 8, { bold: true })
  p.text(470, 175, 'Valor', 8, { bold: true })
  o.items.forEach(([desc, cents], i) => {
    p.text(44, 196 + i * 16, desc, 8)
    p.text(470, 196 + i * 16, formatBRLPlain(cents), 8)
  })
  p.box(300, 290, 259, 30, { fill: '#f5f5f7', stroke: '#e0e0e6', r: 3 })
  const label = p.text(312, 300, 'Valor total:', 11, { bold: true })
  const value = p.after(label, formatBRLPlain(o.cents), 11, { bold: true })
  p.text(36, 350, 'Documento fictício gerado para demonstração do nfsetable.', 8, { color: '#5b5b60' })
  p.footer(0, 1, 'Fatura de demonstração · dados fictícios')
  return { pages: [p.build()], value, valuePage: 0, competence: { item: reference, date: isoDate(reference.text) } }
}

/** A scanned page: an image only, no text layer (noText). */
function scannedPage(): Built {
  const p = new PageBuilder()
  p.shapes.push({ kind: 'smudge', x: 60, y: 60, w: 300, h: 14 })
  for (let i = 0; i < 34; i++) {
    const w = 180 + ((i * 97) % 300)
    p.shapes.push({ kind: 'smudge', x: 60, y: 120 + i * 17, w, h: 5 })
  }
  p.shapes.push({ kind: 'rect', x: 380, y: 690, w: 150, h: 60, stroke: '#8e897d', fill: 'none' })
  return { pages: [p.build(true)], value: null, valuePage: 0 }
}

// ---------------------------------------------------------------- documents

interface MockDoc {
  path: string
  name: string
  dir: string
  /** Identical for byte-identical copies (drives the fake hash). */
  contentId: string
  /** null = corrupt file. */
  pages: MockPage[] | null
  docType: 'NFS-e' | 'PDF'
  /** Value found by the regular extraction (built-in profile or heuristic), if any. */
  found: { item: TextItem; page: number; origin: Origin } | null
  /** Value printed on the page but not found by the regular extraction. */
  hidden: { item: TextItem; page: number } | null
  /** Optional fields found by the regular extraction. */
  service: { item: TextItem; page: number; origin: Origin } | null
  competence: { item: TextItem; page: number; origin: Origin; date: string } | null
  /** Large synthetic folder: the pages are only built when needed. */
  lazy?: () => void
}

const BUILTIN: Origin = { type: 'profile', id: 'danfse-national', name: 'DANFSe (padrão nacional)' }
const AUTO: Origin = { type: 'auto' }

export const MOCK_FOLDERS = {
  primary: 'C:/Notas/2026',
  subfolder: 'C:/Notas/2026/julho',
  secondary: '/home/fulano/notas',
  loose: 'C:/Notas/avulsas',
  /** Bills paid by the company (expenses): internet, rent and the accountant's note. */
  expenses: 'C:/Notas/despesas',
  /** 3000 synthetic notes, for performance checks (add it by path, e.g. through localStorage). */
  large: 'C:/Notas/lote-3000',
}

const DOCS = new Map<string, MockDoc>()

function fill(doc: MockDoc, built: Built | null, kind: 'found' | 'hidden', origin: Origin) {
  doc.pages = built ? built.pages : null
  doc.found = built?.value && kind === 'found' ? { item: built.value, page: built.valuePage, origin } : null
  doc.hidden = built?.value && kind === 'hidden' ? { item: built.value, page: built.valuePage } : null
  doc.service = built?.service ? { item: built.service, page: 0, origin } : null
  doc.competence = built?.competence?.date
    ? { item: built.competence.item, page: 0, origin, date: built.competence.date }
    : null
}

function add(dir: string, name: string, built: Built | null, kind: 'found' | 'hidden', docType: 'NFS-e' | 'PDF', origin: Origin, contentId?: string) {
  const path = `${dir}/${name}`
  const doc: MockDoc = {
    path,
    name,
    dir,
    contentId: contentId ?? path,
    pages: null,
    docType,
    found: null,
    hidden: null,
    service: null,
    competence: null,
  }
  fill(doc, built, kind, origin)
  DOCS.set(path, doc)
  return doc
}

/** Like `add`, but the layout is only built on first use (keeps 3000 documents cheap). */
function addLazy(dir: string, name: string, build: () => Built, kind: 'found' | 'hidden', docType: 'NFS-e' | 'PDF', origin: Origin) {
  const doc = add(dir, name, null, kind, docType, origin)
  doc.pages = []
  doc.lazy = () => {
    doc.lazy = undefined
    fill(doc, build(), kind, origin)
  }
}

/** Documents by path, with lazy layouts materialized. */
function getDoc(path: string): MockDoc | undefined {
  const doc = DOCS.get(path)
  doc?.lazy?.()
  return doc
}

function fnv1a(text: string, seed: number): number {
  let h = (0x811c9dc5 ^ seed) >>> 0
  for (let i = 0; i < text.length; i++) {
    h ^= text.charCodeAt(i)
    h = Math.imul(h, 0x01000193) >>> 0
  }
  return h >>> 0
}

function addDuplicate(of: MockDoc, name: string) {
  const path = `${of.dir}/${name}`
  DOCS.set(path, { ...of, path, name })
}

;(() => {
  const A = MOCK_FOLDERS.primary
  add(A, 'fatura-servicos-0031.pdf', unusualInvoice({ number: '0031', issued: '12/07/2026', client: DELTA, cents: 145075, shift: 0 }), 'hidden', 'PDF', AUTO)
  add(A, 'fatura-servicos-0032.pdf', unusualInvoice({ number: '0032', issued: '19/07/2026', client: DELTA, cents: 99990, shift: 40 }), 'hidden', 'PDF', AUTO)
  add(A, 'nfse-2026-001-cliente-alfa.pdf', danfse({ version: 2, number: '101', issued: '06/04/2026 09:12:40', competence: '04/2026', client: ALFA, cents: 985000, seed: 11 }), 'found', 'NFS-e', BUILTIN)
  const beta = add(A, 'nfse-2026-002-cliente-beta.pdf', danfse({ version: 2, number: '102', issued: '07/05/2026 14:03:11', competence: '05/2026', client: BETA, cents: 723456, seed: 12 }), 'found', 'NFS-e', BUILTIN)
  addDuplicate(beta, 'nfse-2026-002-cliente-beta_copia.pdf')
  add(A, 'nfse-2026-003-cliente-gama.pdf', danfse({ version: 1, number: '97', issued: '28/12/2025 16:45:02', competence: '12/2025', client: GAMA, cents: 684510, seed: 13 }), 'found', 'NFS-e', BUILTIN)
  add(A, 'nfse-2026-004-cliente-delta.pdf', danfse({ version: 2, number: '104', issued: '15/06/2026 10:32:00', competence: '06/2026', client: DELTA, cents: 837250, retained: 12750, pages: 2, seed: 14 }), 'found', 'NFS-e', BUILTIN)
  add(A, 'nfse-2026-005-cliente-alfa.pdf', danfse({ version: 2, number: '105', issued: '22/07/2026 11:20:54', competence: '07/2026', client: ALFA, cents: 1198000, seed: 15 }), 'found', 'NFS-e', BUILTIN)
  add(A, 'nfse-2026-006-cancelada.pdf', danfse({ version: 2, number: '106', issued: '23/07/2026 08:00:10', competence: '07/2026', client: BETA, cents: 500000, seed: 16 }), 'found', 'NFS-e', BUILTIN)
  add(A, 'nota-danificada.pdf', null, 'found', 'PDF', AUTO)
  add(A, 'nota-municipal-0187.pdf', municipal({ number: '0187', issued: '03/07/2026 14:05:11', longDate: '3 de julho de 2026', client: FULANO, cents: 161235 }), 'found', 'PDF', AUTO)
  add(A, 'nota-municipal-0192.pdf', municipal({ number: '0192', issued: '17/06/2026 09:41:37', client: GAMA, cents: 207540 }), 'found', 'PDF', AUTO)
  add(A, 'recibo-consultoria-julho.pdf', receipt({ cents: 110000, words: 'mil e cem reais' }), 'hidden', 'PDF', AUTO)
  add(A, 'scan-digitalizado-0007.pdf', scannedPage(), 'hidden', 'PDF', AUTO)

  const J = MOCK_FOLDERS.subfolder
  add(J, 'fatura-servicos-0035.pdf', unusualInvoice({ number: '0035', issued: '26/07/2026', client: EPSILON, cents: 234567, shift: 20 }), 'hidden', 'PDF', AUTO)
  add(J, 'nfse-2026-007-cliente-epsilon.pdf', danfse({ version: 2, number: '107', issued: '29/07/2026 17:10:05', competence: '07/2026', client: EPSILON, cents: 432109, seed: 17 }), 'found', 'NFS-e', BUILTIN)

  const B = MOCK_FOLDERS.secondary
  add(B, 'fatura-servicos-0036.pdf', unusualInvoice({ number: '0036', issued: '02/08/2026', client: FULANO, cents: 305000, shift: 0 }), 'hidden', 'PDF', AUTO)
  add(B, 'nfse-0042-servicos-web.pdf', danfse({ version: 2, number: '42', issued: '04/08/2026 10:00:00', competence: '08/2026', client: FULANO, cents: 1268232, retained: 19288, seed: 18 }), 'found', 'NFS-e', BUILTIN)
  add(B, 'nfse-0043-servicos-web.pdf', danfse({ version: 1, number: '43', issued: '20/11/2025 15:30:00', competence: '11/2025', client: FULANO, cents: 921000, seed: 19 }), 'found', 'NFS-e', BUILTIN)

  const L = MOCK_FOLDERS.loose
  add(L, 'nfse-2026-008-cliente-zeta.pdf', danfse({ version: 2, number: '108', issued: '01/09/2026 13:13:13', competence: '08/2026', client: ZETA, cents: 345678, seed: 20 }), 'found', 'NFS-e', BUILTIN)
  add(L, 'recibo-aluguel-sala.pdf', rentReceipt({ cents: 132000 }), 'found', 'PDF', AUTO)

  // Expenses: the internet bills and the accountant's DANFSe are classified by the user's rules
  // ("Internet", "Contador"); the rent receipt has no rule yet (it reads as revenue until one exists).
  const D = MOCK_FOLDERS.expenses
  const internet = (month: string, number: string, cents: number) =>
    bill({
      supplier: 'PROVEDOR NET EXEMPLO',
      doc: '88.888.888/0001-88',
      title: 'Internet fibra 500 Mb',
      number,
      reference: month,
      cents,
      items: [
        ['Plano fibra 500 Mb', cents - 1990],
        ['IP fixo', 1990],
      ],
    })
  add(D, 'internet-2026-05.pdf', internet('05/2026', '552190', 12990), 'found', 'PDF', AUTO)
  add(D, 'internet-2026-06.pdf', internet('06/2026', '561022', 13490), 'found', 'PDF', AUTO)
  add(D, 'aluguel-junho.pdf', rentReceipt({ cents: 150000, reference: '06/2026' }), 'found', 'PDF', AUTO)
  add(D, 'contador-2026-06.pdf', danfse({ version: 2, number: '2231', issued: '05/06/2026 08:30:00', competence: '06/2026', client: PROVIDER, emitter: CONTADOR, cents: 45000, seed: 31 }), 'found', 'NFS-e', BUILTIN)

  // 3000 notes over 12 months: mostly DANFSe, some municipal notes without competence, some
  // invoices whose value is not found.
  const G = MOCK_FOLDERS.large
  const clients = [ALFA, BETA, GAMA, DELTA, EPSILON, ZETA]
  const slugs = ['alfa', 'beta', 'gama', 'delta', 'epsilon', 'zeta']
  for (let i = 1; i <= 3000; i++) {
    const n = String(i).padStart(4, '0')
    const k = i % clients.length
    const cents = 40_000 + (fnv1a(`lote-${i}`, 5) % 1_200_000)
    const month = ((i * 7) % 12) + 1
    const year = month >= 10 ? 2025 : 2026
    const mm = String(month).padStart(2, '0')
    if (i % 61 === 0) {
      addLazy(G, `fatura-lote-${n}.pdf`, () => unusualInvoice({ number: n, issued: `10/${mm}/${year}`, client: clients[k], cents, shift: 0 }), 'hidden', 'PDF', AUTO)
    } else if (i % 97 === 0) {
      addLazy(G, `nota-municipal-lote-${n}.pdf`, () => municipal({ number: n, issued: '', longDate: `${(i % 27) + 1} de março de 2026`, client: clients[k], cents }), 'found', 'PDF', AUTO)
    } else {
      addLazy(G, `nfse-lote-${n}-cliente-${slugs[k]}.pdf`, () => danfse({ version: 2, number: n, issued: `05/${mm}/${year} 10:00:00`, competence: `${mm}/${year}`, client: clients[k], cents, seed: i }), 'found', 'NFS-e', BUILTIN)
    }
  }
})()

const FOLDERS = new Set(Object.values(MOCK_FOLDERS))

/** Deterministic fake SHA-256 (64 hex chars) so overrides survive reloads. */
function fakeHash(id: string): string {
  let out = ''
  for (let i = 0; i < 8; i++) out += fnv1a(id, Math.imul(i + 1, 0x9e3779b1)).toString(16).padStart(8, '0')
  return out
}

function docText(doc: MockDoc): string {
  return compactText((doc.pages ?? []).flatMap((p) => p.items.map((i) => i.text)).join(' '))
}

function bboxOf(item: TextItem): Rect {
  return { x: item.x, y: item.y, w: item.w, h: item.h }
}

function fieldValue(item: TextItem, page: number, origin: Origin): FieldValue {
  return { raw: item.text, cents: parseMoney(item.text), date: null, text: null, origin, page, bbox: bboxOf(item) }
}

function dateValue(item: TextItem, page: number, origin: Origin, date: string): FieldValue {
  return { raw: item.text, cents: null, date, text: null, origin, page, bbox: bboxOf(item) }
}

// ---------------------------------------------------------------- region logic (mini engine)

function center(r: Rect) {
  return { x: r.x + r.w / 2, y: r.y + r.h / 2 }
}

/** Money item mostly inside `rect` (half of its width, vertical center inside); nearest wins. */
function findMoney(items: TextItem[], rect: Rect): TextItem | null {
  const c = center(rect)
  let best: TextItem | null = null
  let bestDistance = Infinity
  for (const item of items) {
    if (!isMoneyText(item.text)) continue
    const overlap = Math.min(rect.x + rect.w, item.x + item.w) - Math.max(rect.x, item.x)
    const cy = item.y + item.h / 2
    if (overlap < item.w * 0.5 || cy < rect.y || cy > rect.y + rect.h) continue
    const ic = center(bboxOf(item))
    const d = Math.hypot(ic.x - c.x, ic.y - c.y)
    if (d < bestDistance) {
      best = item
      bestDistance = d
    }
  }
  return best
}

/** Anchor for a drawn rect: nearest label above (40 pt), else to the left on the same line (250 pt). */
function deriveAnchor(items: TextItem[], rect: Rect): RegionAnchor | null {
  const labels = items.filter((i) => !isMoneyText(i.text) && normalizeText(i.text).replace(/[^a-z]/g, '').length >= 3)
  let best: TextItem | null = null
  let bestDistance = Infinity
  const left = rect.x - 20
  const right = rect.x + rect.w + 20
  for (const i of labels) {
    const bottom = i.y + i.h
    const d = rect.y - bottom
    if (bottom > rect.y + 2 || d > 40 || i.x + i.w < left || i.x > right) continue
    if (d < bestDistance) {
      best = i
      bestDistance = d
    }
  }
  if (!best) {
    const cy = rect.y + rect.h / 2
    for (const i of labels) {
      const icy = i.y + i.h / 2
      const d = rect.x - (i.x + i.w)
      if (Math.abs(icy - cy) > Math.max(i.h, rect.h) / 2 || i.x + i.w > rect.x + 2 || d > 250) continue
      if (d < bestDistance) {
        best = i
        bestDistance = d
      }
    }
  }
  if (!best) return null
  const round = (n: number) => Math.round(n * 10) / 10
  return { text: normalizeText(best.text), dx: round(rect.x - best.x), dy: round(rect.y - best.y), w: round(rect.w), h: round(rect.h) }
}

function evalRegion(doc: MockDoc, rule: RegionRule): { item: TextItem; page: number } | null {
  const page = doc.pages?.[rule.page]
  if (!page) return null
  const abs: Rect = { x: rule.rect.x * PAGE_W, y: rule.rect.y * PAGE_H, w: rule.rect.w * PAGE_W, h: rule.rect.h * PAGE_H }
  const anchor = rule.anchor
  if (anchor) {
    const exact = page.items.filter((i) => normalizeText(i.text) === anchor.text)
    const candidates = exact.length ? exact : page.items.filter((i) => normalizeText(i.text).includes(anchor.text))
    const ox = abs.x - anchor.dx
    const oy = abs.y - anchor.dy
    candidates.sort((a, b) => Math.hypot(a.x - ox, a.y - oy) - Math.hypot(b.x - ox, b.y - oy))
    for (const c of candidates) {
      const hit = findMoney(page.items, { x: c.x + anchor.dx, y: c.y + anchor.dy, w: anchor.w, h: anchor.h })
      if (hit) return { item: hit, page: rule.page }
    }
  }
  const hit = findMoney(page.items, abs)
  return hit ? { item: hit, page: rule.page } : null
}

function evalRule(doc: MockDoc, rule: Rule): { item: TextItem; page: number } | null {
  // Anchor rules only exist in the built-in profile, whose results are precomputed in DOCS.
  return rule.type === 'region' ? evalRegion(doc, rule) : null
}

// ---------------------------------------------------------------- SVG rendering

const fmt = (n: number) => String(Math.round(n * 100) / 100)

function escapeXml(text: string): string {
  return text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function qrPath(x: number, y: number, size: number, seed: number): string {
  const n = 25
  const m = size / n
  let state = seed * 2654435761
  const next = () => {
    state = (Math.imul(state, 1103515245) + 12345) >>> 0
    return state / 4294967296
  }
  const inFinder = (r: number, c: number) => (r < 8 && c < 8) || (r < 8 && c >= n - 8) || (r >= n - 8 && c < 8)
  let d = ''
  const cell = (r: number, c: number) => {
    d += `M${fmt(x + c * m)} ${fmt(y + r * m)}h${fmt(m)}v${fmt(m)}h-${fmt(m)}z`
  }
  for (let r = 0; r < n; r++) {
    for (let c = 0; c < n; c++) {
      if (inFinder(r, c)) continue
      if (next() < 0.47) cell(r, c)
    }
  }
  const finder = (r0: number, c0: number) => {
    for (let r = 0; r < 7; r++) {
      for (let c = 0; c < 7; c++) {
        const ring = r === 0 || r === 6 || c === 0 || c === 6
        const core = r >= 2 && r <= 4 && c >= 2 && c <= 4
        if (ring || core) cell(r0 + r, c0 + c)
      }
    }
  }
  finder(0, 0)
  finder(0, n - 7)
  finder(n - 7, 0)
  return `<path d="${d}" fill="#1d1d1f"/>`
}

function renderSvg(page: MockPage): string {
  const out: string[] = [
    `<svg xmlns="http://www.w3.org/2000/svg" width="${PAGE_W}" height="${PAGE_H}" viewBox="0 0 ${PAGE_W} ${PAGE_H}">`,
    '<defs><filter id="blur"><feGaussianBlur stdDeviation="0.9"/></filter></defs>',
    `<rect width="${PAGE_W}" height="${PAGE_H}" fill="${page.scanned ? '#f1eee6' : '#ffffff'}"/>`,
  ]
  if (page.scanned) out.push('<g transform="rotate(-0.7 297 421)">')
  for (const s of page.shapes) {
    if (s.kind === 'rect') {
      out.push(
        `<rect x="${fmt(s.x)}" y="${fmt(s.y)}" width="${fmt(s.w)}" height="${fmt(s.h)}" rx="${s.r ?? 0}" fill="${s.fill ?? 'none'}" stroke="${s.stroke ?? 'none'}" stroke-width="0.7"/>`,
      )
    } else if (s.kind === 'line') {
      out.push(
        `<line x1="${fmt(s.x1)}" y1="${fmt(s.y1)}" x2="${fmt(s.x2)}" y2="${fmt(s.y2)}" stroke="${s.stroke ?? '#b9bcc4'}" stroke-width="${s.width ?? 0.7}"/>`,
      )
    } else if (s.kind === 'qr') {
      out.push(qrPath(s.x, s.y, s.size, s.seed))
    } else {
      out.push(`<rect x="${fmt(s.x)}" y="${fmt(s.y)}" width="${fmt(s.w)}" height="${fmt(s.h)}" fill="#7d786c" opacity="0.55" filter="url(#blur)"/>`)
    }
  }
  if (page.scanned) out.push('</g>')
  out.push(`<g font-family="${FONT}">`)
  for (const t of page.items) {
    out.push(
      `<text x="${fmt(t.x)}" y="${fmt(t.y + t.size * 0.9)}" font-size="${t.size}"${t.bold ? ' font-weight="700"' : ''} fill="${t.color}" textLength="${fmt(t.w)}" lengthAdjust="spacingAndGlyphs">${escapeXml(t.text)}</text>`,
    )
  }
  out.push('</g></svg>')
  return out.join('')
}

// ---------------------------------------------------------------- events

type Handler = (progress: Progress) => void
const handlers = new Map<string, Set<Handler>>()

function listen(event: string, handler: Handler): Unlisten {
  let set = handlers.get(event)
  if (!set) {
    set = new Set()
    handlers.set(event, set)
  }
  set.add(handler)
  return () => set.delete(handler)
}

function emit(event: string, progress: Progress) {
  for (const handler of handlers.get(event) ?? []) handler({ ...progress })
}

// ---------------------------------------------------------------- profiles

const BUILTIN_PROFILE: Profile = {
  id: 'danfse-national',
  name: 'DANFSe (padrão nacional)',
  builtin: true,
  fingerprint: ['danfse'],
  namePatterns: [],
  docType: 'NFS-e',
  kind: 'revenue',
  fields: {
    net_value: [
      { type: 'anchor', pattern: 'valor\\s*liquido\\s*da\\s*nfs-?e', excludeSuffix: '^\\s*\\+', direction: 'below', maxDistance: 24 },
      { type: 'anchor', pattern: 'valor\\s*liquido\\s*da\\s*nfs-?e', excludeSuffix: '^\\s*\\+', direction: 'right', maxDistance: 250 },
    ],
  },
}

/** Rules of a fresh demo: the accountant's notes and the internet bills are expenses. */
const SEEDED_PROFILES: Profile[] = [
  {
    id: 'contador',
    name: 'Contador',
    builtin: false,
    fingerprint: [],
    namePatterns: ['contador'],
    docType: 'Contador',
    kind: 'expense',
    fields: {},
  },
  {
    id: 'internet',
    name: 'Internet',
    builtin: false,
    fingerprint: [],
    namePatterns: ['internet-*'],
    docType: 'Internet',
    kind: 'expense',
    fields: {},
  },
]

// ---------------------------------------------------------------- data folder (profiles + store)

const DEFAULT_DATA_DIR = 'C:/Users/fulano/AppData/Roaming/io.github.nfsetable'
/** What the folder picker of the demo returns. */
const DRIVE_DATA_DIR = 'G:/Meu Drive/nfsetable'
const DATA_DIR_KEY = 'nfsetable.mock.dataDir'

function samePath(a: string, b: string): boolean {
  return pathKey(a) === pathKey(b)
}

/** The data folder in use (the chosen one, unless `?datadir=missing` simulates it unavailable). */
function chosenDataDir(): string | null {
  try {
    return window.localStorage.getItem(DATA_DIR_KEY)
  } catch {
    return null
  }
}

function dataDirUnavailable(): boolean {
  try {
    return new URLSearchParams(window.location.search).get('datadir') === 'missing'
  } catch {
    return false
  }
}

function currentDataDir(): string {
  const chosen = chosenDataDir()
  return chosen && !dataDirUnavailable() ? chosen : DEFAULT_DATA_DIR
}

/** localStorage prefix of a data folder; the default one keeps the keys of the first versions. */
function dirPrefix(dir: string): string {
  return samePath(dir, DEFAULT_DATA_DIR) ? 'nfsetable.' : `nfsetable.mockdir.${fnv1a(pathKey(dir), 3).toString(36)}.`
}

const storeKey = (dir: string, name: string) => `${dirPrefix(dir)}store.${name}`
const profilesKey = (dir: string) => `${dirPrefix(dir)}mock.profiles`

/** The user's profiles of the data folder in use (a fresh default folder gets the demo rules). */
function loadUserProfiles(): Profile[] {
  const dir = currentDataDir()
  try {
    const raw = window.localStorage.getItem(profilesKey(dir))
    if (raw == null) return samePath(dir, DEFAULT_DATA_DIR) ? ipcClone(SEEDED_PROFILES) : []
    const list: unknown = JSON.parse(raw)
    return Array.isArray(list) ? (list as Profile[]) : []
  } catch {
    return []
  }
}

function saveUserProfiles(list: Profile[]) {
  try {
    window.localStorage.setItem(profilesKey(currentDataDir()), JSON.stringify(list))
  } catch {
    // Ignore: the demo keeps working in memory.
  }
}

let userProfiles: Profile[] = []

// Results kept between runs, like the backend's <data folder>/cache/results.json: valid only for
// the profiles they were read with.
interface ResultCache {
  key: string
  results: Record<string, DocResult>
}
let resultCache: ResultCache | null = null
let resultCacheDir = ''
let resultCacheDirty = false

const resultsKey = (dir: string) => `${dirPrefix(dir)}mock.results`
const resultCacheKey = (profiles: Profile[]) => `mock-1:${JSON.stringify(profiles)}`

/** The cache of the data folder in use, emptied when `key` differs from the one it was read with. */
function results(key: string): ResultCache {
  const dir = currentDataDir()
  if (!resultCache || resultCacheDir !== dir) {
    resultCache = null
    resultCacheDir = dir
    try {
      const raw: unknown = JSON.parse(window.localStorage.getItem(resultsKey(dir)) ?? 'null')
      if (raw && typeof raw === 'object') resultCache = raw as ResultCache
    } catch {
      // A corrupt cache is an empty one.
    }
  }
  if (!resultCache || resultCache.key !== key) {
    resultCache = { key, results: {} }
    resultCacheDirty = true
  }
  return resultCache
}

/** Deep copy through JSON, like Tauri's IPC does (also works on Svelte state proxies). */
function ipcClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

function fingerprintMatches(profile: Profile, doc: MockDoc): boolean {
  if (!profile.fingerprint.length) return true
  const text = docText(doc)
  return profile.fingerprint.every((f) => text.includes(compactText(f)))
}

/** Fingerprint and file name patterns, like the engine. */
function profileMatches(profile: Profile, doc: MockDoc): boolean {
  return nameMatches(profile.namePatterns ?? [], doc.name) && fingerprintMatches(profile, doc)
}

/**
 * Type and kind: the user's profiles first (so a rule can say "the DANFSe of my accountant is an
 * expense"), then the built-in one; the type defaults to the layout's.
 */
function classify(doc: MockDoc, profiles: Profile[]): { docType: string; kind: DocKind } {
  const matching = [...profiles, BUILTIN_PROFILE].filter((p) => profileMatches(p, doc))
  const docType = matching.find((p) => p.docType)?.docType ?? doc.docType
  const kind = matching.find((p) => p.kind)?.kind ?? 'revenue'
  return { docType, kind }
}

// ---------------------------------------------------------------- extraction

const CORRUPT = 'O arquivo não é um PDF válido ou está corrompido.'
const MISSING = 'Arquivo não encontrado.'

function extractOne(path: string, profiles: Profile[] = userProfiles): DocResult {
  const doc = getDoc(path)
  const base = {
    path,
    message: null,
    fields: {},
    docType: doc?.docType ?? 'PDF',
    kind: 'revenue' as DocKind,
    pageCount: doc?.pages?.length ?? 0,
  }
  if (!doc) return { ...base, status: 'error', message: MISSING }
  if (!doc.pages) return { ...base, status: 'error', message: CORRUPT }
  Object.assign(base, classify(doc, profiles))
  if (!doc.pages.some((p) => p.items.length > 0)) {
    return { ...base, status: 'noText', message: 'Nenhuma página tem texto extraível (provavelmente é uma imagem digitalizada).' }
  }
  // Optional fields never decide the status.
  const optional: Record<string, FieldValue> = {}
  if (doc.service) optional.service_value = fieldValue(doc.service.item, doc.service.page, doc.service.origin)
  if (doc.competence) {
    const c = doc.competence
    optional.competence = dateValue(c.item, c.page, c.origin, c.date)
  }
  if (doc.found) {
    return { ...base, status: 'ok', fields: { net_value: fieldValue(doc.found.item, doc.found.page, doc.found.origin), ...optional } }
  }
  for (const profile of profiles) {
    if (!profileMatches(profile, doc)) continue
    for (const rule of profile.fields.net_value ?? []) {
      const hit = evalRule(doc, rule)
      if (hit) {
        const origin: Origin = { type: 'profile', id: profile.id, name: profile.name }
        return { ...base, status: 'ok', fields: { net_value: fieldValue(hit.item, hit.page, origin), ...optional } }
      }
    }
  }
  return { ...base, status: 'notFound', message: 'Valor líquido não encontrado.', fields: optional }
}

// ---------------------------------------------------------------- exports (browser download)

function csvField(value: string): string {
  return /[;"\r\n]/.test(value) ? `"${value.replace(/"/g, '""')}"` : value
}

function csvAmount(cents: number): string {
  const negative = cents < 0
  const abs = Math.abs(cents)
  return `${negative ? '-' : ''}${Math.trunc(abs / 100)},${String(abs % 100).padStart(2, '0')}`
}

function buildCsv(rows: ExportRow[]): string {
  const lines = ['Arquivo;Caminho;Tipo;Natureza;Competência;Valor;Status;Origem']
  let revenue = 0
  let expense = 0
  for (const r of rows) {
    const isExpense = r.kind === 'expense'
    if (r.cents != null) {
      if (isExpense) expense += r.cents
      else revenue += r.cents
    }
    const competence = r.competence ? formatMonth(r.competence) : ''
    const kind = isExpense ? 'Despesa' : 'Receita'
    lines.push(
      [r.file, r.path, r.docType, kind, competence, r.cents == null ? '' : csvAmount(r.cents), r.status, r.origin]
        .map(csvField)
        .join(';'),
    )
  }
  lines.push(`TOTAL RECEITAS;;;;;${csvAmount(revenue)};;`)
  if (expense) lines.push(`TOTAL DESPESAS;;;;;${csvAmount(expense)};;`)
  return '\ufeff' + lines.join('\r\n') + '\r\n'
}

function sqlText(value: string | null, mysql: boolean): string {
  if (value == null) return 'NULL'
  const escaped = mysql ? value.replace(/\\/g, '\\\\').replace(/'/g, "''") : value.replace(/'/g, "''")
  return `'${escaped}'`
}

function sqlAmount(cents: number | null): string {
  if (cents == null) return 'NULL'
  const abs = Math.abs(cents)
  return `${cents < 0 ? '-' : ''}${Math.trunc(abs / 100)}.${String(abs % 100).padStart(2, '0')}`
}

/**
 * Same shape the core writes (simplified): a transaction with CREATE TABLE IF NOT EXISTS notas and
 * the INSERTs; competence as a DATE ('yyyy-mm-01'), amounts with a decimal point, NULL when unknown.
 */
function buildSql(request: ExportRequest): string {
  const mysql = request.format === 'mysql'
  const q = (name: string) => (mysql ? `\`${name}\`` : name)
  const lines = [
    `-- Exportado pelo nfsetable em ${request.generatedAt} (modo demonstração)`,
    `-- ${request.rows.length} notas`,
    '',
    ...(mysql
      ? ['SET NAMES utf8mb4;', 'START TRANSACTION;']
      : ["SET client_encoding = 'UTF8';", 'BEGIN;']),
    '',
    `CREATE TABLE IF NOT EXISTS ${q('notas')} (`,
    mysql ? '    `id` INT AUTO_INCREMENT PRIMARY KEY,' : '    id SERIAL PRIMARY KEY,',
    `    ${q('arquivo')} ${mysql ? 'VARCHAR(512)' : 'TEXT'} NOT NULL,`,
    `    ${q('caminho')} TEXT NOT NULL,`,
    `    ${q('tipo')} ${mysql ? 'VARCHAR(100)' : 'TEXT'} NOT NULL,`,
    `    ${q('competencia')} DATE${mysql ? ' NULL' : ''},`,
    `    ${q('valor')} ${mysql ? 'DECIMAL(14,2) NULL' : 'NUMERIC(14,2)'},`,
    `    ${q('status')} ${mysql ? 'VARCHAR(40)' : 'TEXT'} NOT NULL,`,
    `    ${q('origem')} ${mysql ? 'VARCHAR(200)' : 'TEXT'} NOT NULL`,
    mysql ? ') ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;' : ');',
    '',
  ]
  if (request.rows.length) {
    const columns = ['arquivo', 'caminho', 'tipo', 'competencia', 'valor', 'status', 'origem'].map(q).join(', ')
    lines.push(`INSERT INTO ${q('notas')} (${columns}) VALUES`)
    request.rows.forEach((r, i) => {
      const values = [
        sqlText(r.file, mysql),
        sqlText(r.path, mysql),
        sqlText(r.docType, mysql),
        r.competence ? `'${r.competence}-01'` : 'NULL',
        sqlAmount(r.cents),
        sqlText(r.status, mysql),
        sqlText(r.origin, mysql),
      ]
      lines.push(`    (${values.join(', ')})${i === request.rows.length - 1 ? ';' : ','}`)
    })
    lines.push('')
  }
  lines.push('COMMIT;')
  return lines.join('\n') + '\n'
}

function download(content: string, type: string, path: string) {
  const blob = new Blob([content], { type })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = baseName(path)
  document.body.appendChild(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

// ---------------------------------------------------------------- controls for dev / screenshots

export interface MockControls {
  /** Clears everything the app stored (sources, overrides...) and reloads. */
  reset(): void
  /** Simulates a missing PDFium library (reloads the page). */
  setPdfiumMissing(missing: boolean): void
  /** Box (points) of the value printed on a mock document, found or not. */
  valueBox(path: string): { page: number; rect: Rect } | null
  folders: typeof MOCK_FOLDERS
  files: { unusual: string; unusualShifted: string; danfse: string; receipt: string }
  /** The data folders of the demo (default and the one the folder picker returns). */
  dataDirs: { default: string; drive: string }
  /** Every `export_table` request received (CSV and SQL are also downloaded; Excel/PDF are not). */
  exports: ExportRequest[]
  /** Every `tax_report` input received. */
  taxInputs: TaxInput[]
  /** Calls seen by the backend: scans, and the store documents read (in order). */
  log: { scans: number; reads: string[]; extracted: number }
}

const exportLog: ExportRequest[] = []
const taxLog: TaxInput[] = []
const callLog = { scans: 0, reads: [] as string[], extracted: 0 }

const STORE_NAME = /^[a-z0-9-]{1,40}$/
const STORE_MAX_BYTES = 2 * 1024 * 1024

const PDFIUM_FLAG = 'nfsetable.mock.pdfiumMissing'

function pdfiumMissing(): boolean {
  try {
    if (new URLSearchParams(window.location.search).get('pdfium') === 'missing') return true
    return window.sessionStorage.getItem(PDFIUM_FLAG) === '1'
  } catch {
    return false
  }
}

function installControls() {
  const controls: MockControls = {
    reset() {
      clearAll()
      window.location.reload()
    },
    setPdfiumMissing(missing: boolean) {
      try {
        if (missing) window.sessionStorage.setItem(PDFIUM_FLAG, '1')
        else window.sessionStorage.removeItem(PDFIUM_FLAG)
      } catch {
        // Ignore.
      }
      window.location.reload()
    },
    valueBox(path: string) {
      const doc = getDoc(path)
      const v = doc?.found ?? doc?.hidden
      return v ? { page: v.page, rect: bboxOf(v.item) } : null
    },
    folders: MOCK_FOLDERS,
    files: {
      unusual: `${MOCK_FOLDERS.primary}/fatura-servicos-0031.pdf`,
      unusualShifted: `${MOCK_FOLDERS.primary}/fatura-servicos-0032.pdf`,
      danfse: `${MOCK_FOLDERS.primary}/nfse-2026-001-cliente-alfa.pdf`,
      receipt: `${MOCK_FOLDERS.primary}/recibo-consultoria-julho.pdf`,
    },
    dataDirs: { default: DEFAULT_DATA_DIR, drive: DRIVE_DATA_DIR },
    exports: exportLog,
    taxInputs: taxLog,
    log: callLog,
  }
  window.__mock = controls
}

// ---------------------------------------------------------------- backend

let dataDirError: string | null = null

function mockAppInfo(): AppInfo {
  const missing = pdfiumMissing()
  const chosen = chosenDataDir()
  if (chosen && dataDirUnavailable() && dataDirError == null) {
    dataDirError = `A pasta de dados escolhida (${chosen}) não está disponível; usando a pasta padrão até ela voltar.`
  }
  const dataDir = currentDataDir()
  return {
    version: '0.1.0',
    pdfiumOk: !missing,
    pdfiumError: missing
      ? 'Não foi possível carregar a biblioteca PDFium. Procurado em: C:/Program Files/nfsetable/pdfium.dll; C:/Program Files/nfsetable/resources/pdfium/pdfium.dll (modo demonstração)'
      : null,
    profilesDir: `${dataDir}/profiles`,
    dataDir,
    defaultDataDir: DEFAULT_DATA_DIR,
    dataDirError,
  }
}

export function createMockBackend(): Backend {
  installControls()
  userProfiles = loadUserProfiles()
  const folderCycle = [MOCK_FOLDERS.primary, MOCK_FOLDERS.secondary, MOCK_FOLDERS.expenses]
  let folderCalls = 0
  const nextFolder = () => folderCycle[folderCalls++ % folderCycle.length]

  return {
    async appInfo(): Promise<AppInfo> {
      await sleep(80)
      return mockAppInfo()
    },

    // Same rules as the backend: the new folder must be usable; with `copy`, the current profiles
    // and store documents are copied there first (replacing the ones with the same name).
    async setDataDir(path: string | null, copy: boolean): Promise<AppInfo> {
      await sleep(220)
      const target = path?.trim() || DEFAULT_DATA_DIR
      if (/^[a-z]:\/?$/i.test(target) || target.startsWith('Z:')) throw `Não foi possível gravar na pasta ${target} (modo demonstração).`
      const current = currentDataDir()
      if (copy && !samePath(target, current)) {
        try {
          const from = dirPrefix(current)
          const to = dirPrefix(target)
          const keys: string[] = []
          for (let i = 0; i < window.localStorage.length; i++) {
            const key = window.localStorage.key(i)
            if (key && (key.startsWith(`${from}store.`) || key === `${from}mock.profiles`)) keys.push(key)
          }
          for (const key of keys) window.localStorage.setItem(to + key.slice(from.length), window.localStorage.getItem(key)!)
          // Profiles never saved in the default folder are still the demo rules: copy them too.
          if (!keys.includes(profilesKey(current))) window.localStorage.setItem(profilesKey(target), JSON.stringify(userProfiles))
        } catch {
          throw 'Não foi possível copiar os dados (armazenamento do navegador cheio).'
        }
      }
      try {
        if (samePath(target, DEFAULT_DATA_DIR)) window.localStorage.removeItem(DATA_DIR_KEY)
        else window.localStorage.setItem(DATA_DIR_KEY, target)
      } catch {
        throw 'Não foi possível salvar a configuração.'
      }
      dataDirError = null
      userProfiles = loadUserProfiles()
      return mockAppInfo()
    },

    async scanSources(options: ScanOptions): Promise<ScanResult> {
      callLog.scans++
      await sleep(jitter(180, 120))
      const terms = options.exclude.map(normalizeText).filter(Boolean)
      const excludedName = (name: string) => terms.some((t) => normalizeText(name).includes(t))
      const infos: SourceInfo[] = []
      const found = new Map<string, MockDoc>()
      const excluded = new Set<string>()
      for (const source of options.sources) {
        const path = trimTrailingSeparators(source.path)
        if (FOLDERS.has(path)) {
          let count = 0
          for (const doc of DOCS.values()) {
            const inside = doc.dir === path || (source.recursive && doc.dir.startsWith(`${path}/`))
            if (!inside) continue
            if (excludedName(doc.name)) {
              excluded.add(doc.path)
              continue
            }
            found.set(doc.path, doc)
            count++
          }
          infos.push({ path: source.path, isDir: true, exists: true, fileCount: count })
        } else if (DOCS.has(path)) {
          const doc = DOCS.get(path)!
          const skip = excludedName(doc.name)
          if (skip) excluded.add(doc.path)
          else found.set(doc.path, doc)
          infos.push({ path: source.path, isDir: false, exists: true, fileCount: skip ? 0 : 1 })
        } else {
          infos.push({ path: source.path, isDir: !isPdfPath(path), exists: false, fileCount: 0 })
        }
      }
      const docs = [...found.values()].sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0))
      const firstByContent = new Map<string, string>()
      const files: ScannedFile[] = docs.map((doc) => {
        const first = firstByContent.get(doc.contentId)
        if (!first) firstByContent.set(doc.contentId, doc.path)
        return {
          path: doc.path,
          name: doc.name,
          dir: doc.dir,
          size: 38_000 + (fnv1a(doc.contentId, 7) % 90_000),
          hash: fakeHash(doc.contentId),
          duplicateOf: first ?? null,
        }
      })
      return { sources: infos, files, excluded: [...excluded].sort() }
    },

    async cachedResults(keys: string[]): Promise<Record<string, DocResult>> {
      await sleep(jitter(15, 10))
      const cache = results(resultCacheKey(userProfiles))
      return ipcClone(Object.fromEntries(keys.filter((k) => cache.results[k]).map((k) => [k, cache.results[k]])))
    },

    async extractDocuments(paths: string[], keys: string[]): Promise<DocResult[]> {
      if (keys.length !== paths.length) throw 'Erro interno: uma chave por arquivo.'
      // Like the backend: the profiles of the whole call are the ones in use when it starts.
      const profiles = userProfiles
      const out: DocResult[] = []
      // The large folder is read fast (it exists to measure the table, not the reading).
      const fast = paths.length > 0 && paths.every((p) => p.startsWith(`${MOCK_FOLDERS.large}/`))
      if (fast) await sleep(2)
      for (let i = 0; i < paths.length; i++) {
        if (!fast) await sleep(jitter(35, 40))
        out.push(extractOne(paths[i], profiles))
        callLog.extracted++
        emit('extract-progress', { done: i + 1, total: paths.length })
      }
      const cache = results(resultCacheKey(profiles))
      out.forEach((doc, i) => {
        if (!keys[i] || doc.status === 'error') return
        cache.results[keys[i]] = { ...ipcClone(doc), path: '' }
        resultCacheDirty = true
      })
      return out
    },

    async saveCachedResults(): Promise<void> {
      if (!resultCacheDirty || !resultCache) return
      resultCacheDirty = false
      try {
        window.localStorage.setItem(resultsKey(resultCacheDir), JSON.stringify(resultCache))
      } catch {
        // Ignore: the demo reads the files again next time.
      }
    },

    async renderPage(path: string, page: number, _width: number): Promise<RenderedPage> {
      await sleep(jitter(90, 80))
      const doc = getDoc(path)
      if (!doc) throw MISSING
      if (!doc.pages) throw CORRUPT
      const count = doc.pages.length
      if (page < 0 || page >= count) throw `A página ${page + 1} não existe (o documento tem ${count} página(s)).`
      const svg = renderSvg(doc.pages[page])
      return {
        dataUrl: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`,
        widthPt: PAGE_W,
        heightPt: PAGE_H,
        pageCount: count,
      }
    },

    async readRegion(path: string, page: number, rect: Rect): Promise<RegionRead> {
      await sleep(jitter(140, 80))
      const doc = getDoc(path)
      if (!doc) throw MISSING
      if (!doc.pages) throw CORRUPT
      const items = doc.pages[page]?.items ?? []
      const inside = items
        .filter((i) => {
          const c = center(bboxOf(i))
          return c.x >= rect.x && c.x <= rect.x + rect.w && c.y >= rect.y && c.y <= rect.y + rect.h
        })
        .sort((a, b) => a.y - b.y || a.x - b.x)
      const hit = findMoney(items, rect)
      const rel: Rect = { x: rect.x / PAGE_W, y: rect.y / PAGE_H, w: rect.w / PAGE_W, h: rect.h / PAGE_H }
      return {
        text: inside.map((i) => i.text).join(' '),
        value: hit ? fieldValue(hit, page, { type: 'region' }) : null,
        suggestedRule: { type: 'region', rect: rel, page, anchor: deriveAnchor(items, rect) },
      }
    },

    async testRule(paths: string[], rule: Rule): Promise<RuleTest[]> {
      const out: RuleTest[] = []
      for (let i = 0; i < paths.length; i++) {
        await sleep(jitter(45, 45))
        const path = paths[i]
        const doc = getDoc(path)
        if (!doc) out.push({ path, value: null, error: MISSING })
        else if (!doc.pages) out.push({ path, value: null, error: CORRUPT })
        else {
          const hit = evalRule(doc, rule)
          out.push({ path, value: hit ? fieldValue(hit.item, hit.page, { type: 'region' }) : null, error: null })
        }
        emit('test-progress', { done: i + 1, total: paths.length })
      }
      return out
    },

    async listProfiles(): Promise<Profile[]> {
      await sleep(40)
      const users = [...userProfiles].sort((a, b) => a.name.localeCompare(b.name, 'pt-BR'))
      return [ipcClone(BUILTIN_PROFILE), ...ipcClone(users)]
    },

    async saveProfile(profile: Profile): Promise<Profile> {
      await sleep(120)
      if (profile.id === BUILTIN_PROFILE.id) throw 'Perfis embutidos não podem ser alterados.'
      if (!profile.name.trim()) throw 'Regra inválida: o perfil precisa de um nome'
      if ((profile.namePatterns ?? []).some((p) => !normalizeText(p))) throw 'Regra inválida: padrão de nome de arquivo vazio'
      if (profile.docType != null && !profile.docType.trim()) throw 'Regra inválida: tipo vazio'
      let id = profile.id
      if (!id) {
        const base = slugify(profile.name) || 'perfil'
        id = base
        for (let n = 2; id === BUILTIN_PROFILE.id || userProfiles.some((p) => p.id === id); n++) id = `${base}-${n}`
      }
      const copy = ipcClone(profile)
      const saved: Profile = {
        ...copy,
        namePatterns: copy.namePatterns ?? [],
        docType: copy.docType ?? null,
        kind: copy.kind ?? null,
        id,
        builtin: false,
      }
      userProfiles = [...userProfiles.filter((p) => p.id !== id), saved]
      saveUserProfiles(userProfiles)
      return ipcClone(saved)
    },

    async deleteProfile(id: string): Promise<void> {
      await sleep(80)
      if (id === BUILTIN_PROFILE.id) throw 'Perfis embutidos não podem ser excluídos.'
      userProfiles = userProfiles.filter((p) => p.id !== id)
      saveUserProfiles(userProfiles)
    },

    async exportTable(request: ExportRequest): Promise<void> {
      await sleep(120)
      exportLog.push(ipcClone(request))
      // The browser cannot write to `path`: text formats are downloaded instead; Excel and PDF are
      // only simulated (the real files are written by the core).
      if (request.format === 'csv') download(buildCsv(request.rows), 'text/csv;charset=utf-8', request.path)
      else if (request.format === 'postgresql' || request.format === 'mysql')
        download(buildSql(request), 'application/sql;charset=utf-8', request.path)
    },

    async taxReport(input: TaxInput): Promise<TaxReport> {
      // Logged when called, so the last entry is always the latest request.
      const copy = ipcClone(input)
      taxLog.push(copy)
      await sleep(jitter(90, 90))
      return mockTaxReport(ipcClone(copy))
    },

    async taxCatalog(): Promise<TaxCatalog> {
      await sleep(60)
      return ipcClone(MOCK_TAX_CATALOG)
    },

    // Same rules as the backend store (<data folder>/store/<name>.json), kept in localStorage.
    async readStore(name: string): Promise<unknown> {
      callLog.reads.push(name)
      await sleep(30)
      if (!STORE_NAME.test(name)) throw `Nome de armazenamento inválido: "${name}".`
      const raw = window.localStorage.getItem(storeKey(currentDataDir(), name))
      if (raw == null) return null
      try {
        return JSON.parse(raw) as unknown
      } catch {
        throw `O arquivo "${name}.json" está corrompido.`
      }
    },

    // Written right away (then the simulated latency), so a save started while the page is being
    // closed or reloaded still lands, like an IPC message already sent.
    async writeStore(name: string, value: unknown): Promise<void> {
      if (!STORE_NAME.test(name)) throw `Nome de armazenamento inválido: "${name}".`
      const json = JSON.stringify(value)
      if (json.length > STORE_MAX_BYTES) throw 'Os dados passam do limite de 2 MB.'
      try {
        window.localStorage.setItem(storeKey(currentDataDir(), name), json)
      } catch {
        throw 'Não há espaço para salvar (armazenamento do navegador cheio).'
      }
      await sleep(40)
    },

    async deleteStore(name: string): Promise<void> {
      if (!STORE_NAME.test(name)) throw `Nome de armazenamento inválido: "${name}".`
      try {
        window.localStorage.removeItem(storeKey(currentDataDir(), name))
      } catch {
        throw 'Não foi possível apagar os dados.'
      }
      await sleep(30)
    },

    onExtractProgress: async (handler) => listen('extract-progress', handler),
    onTestProgress: async (handler) => listen('test-progress', handler),

    async pickFolders(): Promise<string[]> {
      await sleep(150)
      return [nextFolder()]
    },

    async pickFiles(): Promise<string[]> {
      await sleep(150)
      return [`${MOCK_FOLDERS.loose}/nfse-2026-008-cliente-zeta.pdf`, `${MOCK_FOLDERS.loose}/recibo-aluguel-sala.pdf`]
    },

    async pickDataDir(): Promise<string | null> {
      await sleep(150)
      return DRIVE_DATA_DIR
    },

    async pickExportPath(format: ExportFormat, defaultName: string): Promise<string | null> {
      await sleep(150)
      return `${MOCK_FOLDERS.primary}/${defaultName || EXPORT_FORMATS[format].defaultName}`
    },

    async openPath(_path: string): Promise<void> {
      await sleep(60)
      throw 'No modo demonstração (navegador) não é possível abrir arquivos.'
    },

    async revealItemInDir(_path: string): Promise<void> {
      await sleep(60)
      throw 'No modo demonstração (navegador) não é possível abrir pastas.'
    },

    async onDragDrop(handler: (state: DragDropState) => void): Promise<Unlisten> {
      // Browsers do not expose file paths: a drop adds the next synthetic folder.
      let depth = 0
      const hasFiles = (e: DragEvent) => !!e.dataTransfer && [...e.dataTransfer.types].includes('Files')
      const onEnter = (e: DragEvent) => {
        if (!hasFiles(e)) return
        e.preventDefault()
        depth++
        handler({ type: 'over' })
      }
      const onOver = (e: DragEvent) => {
        if (hasFiles(e)) e.preventDefault()
      }
      const onLeave = (e: DragEvent) => {
        if (!hasFiles(e)) return
        depth = Math.max(0, depth - 1)
        if (depth === 0) handler({ type: 'leave' })
      }
      const onDrop = (e: DragEvent) => {
        if (!hasFiles(e)) return
        e.preventDefault()
        depth = 0
        handler({ type: 'drop', paths: [nextFolder()] })
      }
      window.addEventListener('dragenter', onEnter)
      window.addEventListener('dragover', onOver)
      window.addEventListener('dragleave', onLeave)
      window.addEventListener('drop', onDrop)
      return () => {
        window.removeEventListener('dragenter', onEnter)
        window.removeEventListener('dragover', onOver)
        window.removeEventListener('dragleave', onLeave)
        window.removeEventListener('drop', onDrop)
      }
    },
  }
}
