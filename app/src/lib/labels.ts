// pt-BR display texts for statuses, origins, rules, export formats and taxes (also used in the
// exports).

import { collapseSpaces } from './format'
import type { ExportFormat, MeiStatus, Origin, Rule, SimplesActivity, Status, TaxRegime } from './types'

/** Status shown in the table: the extraction status plus UI-only states. */
export type RowStatus = Status | 'duplicate' | 'pending'

export const STATUS_LABEL: Record<RowStatus, string> = {
  ok: 'OK',
  notFound: 'Não encontrado',
  noText: 'Sem texto',
  error: 'Erro',
  duplicate: 'Duplicada',
  pending: 'Lendo…',
}

/** Shorter labels for narrow tables (the full label stays in the tooltip and for screen readers). */
export const STATUS_SHORT: Record<RowStatus, string> = {
  ok: 'OK',
  notFound: 'Sem valor',
  noText: 'Sem texto',
  error: 'Erro',
  duplicate: 'Duplicada',
  pending: 'Lendo…',
}

export const STATUS_HINT: Record<RowStatus, string> = {
  ok: 'Valor encontrado.',
  notFound: 'O PDF tem texto, mas o valor não foi encontrado. Marque o valor na prévia.',
  noText: 'O PDF não tem texto (provavelmente é uma imagem digitalizada). Digite o valor.',
  error: 'Não foi possível abrir o arquivo.',
  duplicate: 'Cópia idêntica de outro arquivo; fica fora dos totais.',
  pending: 'Lendo o arquivo…',
}

export function originLabel(origin: Origin | null | undefined): string {
  if (!origin) return ''
  switch (origin.type) {
    case 'profile':
      return origin.name
    case 'auto':
      return 'Automático'
    case 'region':
      return 'Região'
    case 'manual':
      return 'Manual'
  }
}

/** Makes an anchor regex readable: "valor\\s*liquido\\s*da\\s*nfs-?e" -> "valor liquido da nfs-e". */
export function humanizePattern(pattern: string): string {
  const readable = pattern
    .replace(/\\s[*+?]?/g, ' ')
    .replace(/-\?/g, '-')
    .replace(/[\^$\\()[\]{}|?*+]/g, '')
  return collapseSpaces(readable)
}

/** One-line pt-BR description of a rule. */
export function describeRule(rule: Rule): string {
  if (rule.type === 'anchor') {
    const where = rule.direction === 'below' ? 'abaixo do' : 'à direita do'
    return `Valor ${where} rótulo “${humanizePattern(rule.pattern)}”`
  }
  const page = `página ${rule.page + 1}`
  return rule.anchor
    ? `Região na ${page}, ancorada em “${rule.anchor.text}”`
    : `Região fixa na ${page}`
}

// ---------------------------------------------------------------- export formats

export interface ExportFormatInfo {
  /** Menu label. */
  label: string
  /** Second line of the menu item. */
  hint: string
  /** Used in toasts: "<noun> exportado: notas.csv". */
  noun: string
  /** Save dialog: title, suggested file name and filter. */
  dialogTitle: string
  defaultName: string
  filterName: string
  extension: string
}

/** Formats offered by the "Exportar" menu, in menu order. */
export const EXPORT_FORMATS: Record<ExportFormat, ExportFormatInfo> = {
  csv: {
    label: 'CSV',
    hint: 'Planilha simples, separada por ponto e vírgula',
    noun: 'CSV',
    dialogTitle: 'Salvar planilha CSV',
    defaultName: 'notas.csv',
    filterName: 'CSV',
    extension: 'csv',
  },
  xlsx: {
    label: 'Excel (.xlsx)',
    hint: 'Moeda, filtro e resumo por tipo',
    noun: 'Planilha do Excel',
    dialogTitle: 'Salvar planilha do Excel',
    defaultName: 'notas.xlsx',
    filterName: 'Excel',
    extension: 'xlsx',
  },
  pdf: {
    label: 'PDF',
    hint: 'Relatório para imprimir ou enviar',
    noun: 'PDF',
    dialogTitle: 'Salvar relatório em PDF',
    defaultName: 'notas.pdf',
    filterName: 'PDF',
    extension: 'pdf',
  },
  postgresql: {
    label: 'SQL PostgreSQL (.sql)',
    hint: 'CREATE TABLE + INSERT, em uma transação',
    noun: 'Script SQL',
    dialogTitle: 'Salvar script SQL (PostgreSQL)',
    defaultName: 'notas-postgresql.sql',
    filterName: 'SQL',
    extension: 'sql',
  },
  mysql: {
    label: 'SQL MySQL (.sql)',
    hint: 'Para MySQL e MariaDB (utf8mb4)',
    noun: 'Script SQL',
    dialogTitle: 'Salvar script SQL (MySQL)',
    defaultName: 'notas-mysql.sql',
    filterName: 'SQL',
    extension: 'sql',
  },
}

export const EXPORT_FORMAT_ORDER: ExportFormat[] = ['csv', 'xlsx', 'pdf', 'postgresql', 'mysql']

/** Suggested file name of an export, with the company slug: "notas-minha-empresa.xlsx". */
export function exportFileName(format: ExportFormat, companySlug: string | null): string {
  const name = EXPORT_FORMATS[format].defaultName
  if (!companySlug) return name
  const dot = name.lastIndexOf('.')
  const stem = name.slice(0, dot)
  const suffix = stem.startsWith('notas-') ? stem.slice('notas'.length) : ''
  return `notas-${companySlug}${suffix}${name.slice(dot)}`
}

// ---------------------------------------------------------------- taxes

export const REGIME_LABEL: Record<TaxRegime, string> = {
  mei: 'MEI',
  simples: 'Simples Nacional (ME/EPP)',
  presumido: 'Lucro Presumido',
}

export const ACTIVITY_LABEL: Record<SimplesActivity, string> = {
  fatorR: 'Anexo III ou V, pelo Fator R',
  annexIii: 'Sempre Anexo III',
}

export const MEI_STATUS_LABEL: Record<MeiStatus, string> = {
  within: 'Dentro do limite',
  upToTolerance: 'Acima do limite (até 20%)',
  aboveTolerance: 'Mais de 20% acima do limite',
}

export const MEI_STATUS_HINT: Record<MeiStatus, string> = {
  within: 'Cabe no limite do MEI.',
  upToTolerance: 'Sai do MEI em janeiro do ano seguinte e paga a diferença sobre o excesso.',
  aboveTolerance: 'Desenquadramento retroativo a janeiro (ou à abertura): impostos recalculados pelo Simples.',
}

/** The same bands when only the projection reaches them (the year to date is still below). */
export const MEI_PROJECTED_LABEL: Record<MeiStatus, string> = {
  within: MEI_STATUS_LABEL.within,
  upToTolerance: 'Projeção acima do limite',
  aboveTolerance: 'Projeção acima da tolerância',
}

export const MEI_PROJECTED_HINT: Record<MeiStatus, string> = {
  within: MEI_STATUS_HINT.within,
  upToTolerance: 'Se o ritmo continuar, sai do MEI em janeiro do ano seguinte e paga a diferença sobre o excesso.',
  aboveTolerance:
    'Se o ritmo continuar, passa dos 20% de tolerância: o desenquadramento é retroativo a janeiro (ou à abertura), com os impostos recalculados pelo Simples.',
}

const TAX_NAMES: Record<string, string> = {
  irpj: 'IRPJ',
  csll: 'CSLL',
  cofins: 'Cofins',
  pis: 'PIS',
  cpp: 'CPP',
  iss: 'ISS',
  icms: 'ICMS',
  inss: 'INSS',
  irrf: 'IRRF',
  das: 'DAS',
  ibs: 'IBS',
  cbs: 'CBS',
}

const TAX_HINTS: Record<string, string> = {
  irpj: 'Imposto de Renda da Pessoa Jurídica',
  csll: 'Contribuição Social sobre o Lucro Líquido',
  cofins: 'Contribuição para o Financiamento da Seguridade Social',
  pis: 'Programa de Integração Social',
  cpp: 'Contribuição Patronal Previdenciária (o INSS da empresa)',
  iss: 'Imposto Sobre Serviços (municipal)',
}

/** "irpj" -> "IRPJ"; unknown keys are shown uppercased. */
export function taxName(tax: string): string {
  return TAX_NAMES[tax.toLowerCase()] ?? tax.toUpperCase()
}

/** Full name of a tax, for tooltips ("cpp" -> "Contribuição Patronal Previdenciária..."). */
export function taxHint(tax: string): string | undefined {
  return TAX_HINTS[tax.toLowerCase()]
}
