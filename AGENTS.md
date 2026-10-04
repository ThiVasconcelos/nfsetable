# AGENTS.md

Guide for AI coding agents and new contributors. Read it before changing anything.
Also see [README.md](README.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Project overview

nfsetable is an open-source (MIT), 100% local desktop app for Brazilian freelancers and small
companies. It turns NFS-e PDF invoices into a table with totals and gives a tax and
financial-health view for MEI and Simples Nacional (Lucro Presumido for comparison). The UI is in
Brazilian Portuguese. No network, no account, no telemetry.

- **Stack:** Tauri 2 · Rust (edition 2021, Cargo workspace) · PDFium through `pdfium-render`,
  loaded at runtime (pinned build `chromium/8066` from bblanchon/pdfium-binaries) · Svelte 5
  (runes) + TypeScript + Vite.
- **Layers** (only the engine reads PDFs):
  `app/src` (UI) → `invoke`/events → `app/src-tauri` (commands) → `crates/core` (engine) → PDFium.
- **Crates:** `nfsetable-core` (lib `nfsetable_core`, in `crates/core`) and `nfsetable` (the
  desktop binary, in `app/src-tauri`). App identifier: `io.github.nfsetable`.

## Directory map

```
crates/core/                 nfsetable-core: no UI, no network
  src/model.rs               THE contract: every type exchanged between the layers
  src/engine.rs              Engine: binds PDFium; extract, read_region, test_rule, render_page
  src/layout.rs              positioned text: glyphs -> lines -> segments, display coordinates
  src/extract.rs             rules (anchor, region), profile order, AUTO_*_LABELS heuristic
  src/profile.rs             built-in fields and profiles (BUILTIN_PROFILE_SOURCES), matching
  src/scan.rs                PDFs of folders/files: exclude filter, SHA-256, duplicates
  src/export/                CSV, XLSX, PDF (drawn with PDFium), SQL (PostgreSQL/MySQL)
  src/tax/                   tax engine: tables, revenue, payroll, mei, simples, presumido,
                             scenarios (comparison, leftover, pricing, cash flow), window (each
                             month's RBT12 and Fator R), report, money (rounding)
  profiles/*.json            built-in extraction profiles (danfse-national.json)
  tax/br.json                versioned tax tables and CNAE catalog
  examples/                  extract.rs (command line), gen_fixtures.rs (synthetic test PDFs)
  tests/                     extraction, scan_export, export, tax; fixtures/ (synthetic only)
app/src-tauri/src/           nfsetable binary
  main.rs                    setup, NFSETABLE_DATA_DIR override, PDFium search paths
  commands.rs                Tauri commands: thin wrappers, heavy work off the UI thread
  store.rs                   JSON documents in <data folder>/store/<name>.json
  data_dir.rs                data folder choice (config.json in the default folder)
  profiles.rs                user profiles in <data folder>/profiles/*.json
  results.rs                 extraction results kept between runs (<data folder>/cache)
app/src/                     Svelte 5 UI (runes); all text in pt-BR
  lib/api.ts                 typed wrappers of every command (the mock outside Tauri)
  lib/types.ts               TypeScript mirror of model.rs
  lib/store.svelte.ts        notes: sources, results, overrides, filters, period, selection
  lib/tax.svelte.ts          tax page: planning, revenue per month, costs, report
  lib/companies.svelte.ts    companies and app startup ("Escolha a empresa")
  lib/mock.ts                browser mock of the backend (synthetic data, `npm run dev`)
  lib/mock-tax.ts            browser mock of the tax engine
  lib/labels.ts              shared UI labels
  components/                Svelte components
scripts/                     fetch-pdfium.sh, fetch-pdfium.ps1
```

Persisted data: the data folder holds `profiles/` and `store/` (`companies`, and per company
`notes-<id>` and `planning-<id>`), plus `cache/results.json`: extraction results by file SHA-256
and name, valid only for the same `EXTRACTION_VERSION`, build and profiles, so opening the app reads
only new or changed files.
localStorage keeps only UI preferences (theme, sort, startup choice); keys left by the first
versions are migrated into the store once.

## Commands

```sh
# Setup, once, from the repo root. PDFium lands in vendor/pdfium/<platform>/:
# the Rust tests and the desktop app need it, the browser UI does not.
bash scripts/fetch-pdfium.sh                                        # Linux, macOS, Git Bash
powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1   # Windows PowerShell
cd app && npm install

# Run (in app/)
npm run dev              # UI only, in a browser, with the mock (no Tauri, no PDFium)
npm run tauri dev        # desktop app; uses the REAL data folder unless isolated (rule 10)
npm run tauri build      # installers in target/release/bundle/

# Checks before a PR; all must pass (CI runs them on Ubuntu and Windows)
cargo fmt --all -- --check                              # repo root
cargo clippy --workspace --all-targets -- -D warnings   # repo root
cargo test --workspace                                  # repo root
npm run check                                           # in app/ (svelte-check)
npm run build                                           # in app/ (vite build)

# Tools (repo root)
cargo run -p nfsetable-core --example extract -- <folder> [--exclude <text>]... [--recursive]
cargo run -p nfsetable-core --example gen_fixtures      # rewrites crates/core/tests/fixtures/
```

## Hard rules

1. **Privacy.** Never commit, attach or paste real invoices or user data: no real PDFs, names,
   CNPJ/CPF numbers, addresses, bank details, amounts or personal paths, in code, tests,
   fixtures, docs, issues, PRs or commit messages. Use synthetic data ("EMPRESA EXEMPLO LTDA",
   CNPJ 00.000.000/0001-91, "Cidade Exemplo - UF"); test PDFs come only from `gen_fixtures.rs`.
   Never read, copy or change the user's data folder.
2. **Local only.** No network calls, telemetry, update checks or CDN assets. The CSP in
   `app/src-tauri/tauri.conf.json` blocks remote origins; keep it that way.
3. **Money.** Integer cents in the contract (`i64` in Rust, an integer `number` in TS).
   `rust_decimal::Decimal` (in reais) inside the tax engine, rounded to the cent half away from
   zero at each tax step (`tax/money.rs`). Never floats for money; rates cross the contract as
   `f64` converted from the exact decimals.
4. **Tax tables are data.** Rates, limits and brackets live in `crates/core/tax/br.json`, in
   blocks with `valid_from`, `valid_to`, `source` (the legal basis) and optional
   `"unverified": true`. They are chosen by the month being computed (the reference month), never
   by today's date. Do not hard-code them in Rust. Each rule change gets a case in
   `crates/core/tests/tax.rs` with every expectation computed by hand in comments.
5. **Positional extraction.** Values come from the layout model in display coordinates (PDF
   points, top-left origin, page rotation applied): a label plus the value below or to its right
   (`anchor`), or a rectangle (`region`, optionally anchored to nearby text). Never parse values
   with regexes over the text flow: the 2026 DANFSe broke "the last R$ on the line".
6. **PDFium is not thread-safe.** The `Engine` operations (`extract`, `read_region`, `test_rule`,
   `render_page`) each hold a global lock. Code that uses `Engine::pdfium()` directly (the PDF
   export, `gen_fixtures`) must hold `Engine::lock()` for the whole sequence and must not call
   those operations while holding it (same lock: deadlock).
7. **One contract.** The types live in `crates/core/src/model.rs` and are mirrored field by field
   in `app/src/lib/types.ts` (serde camelCase; data-carrying enums tagged with `"type"`;
   `Option<T>` is `T | null`). Change both in the same PR; `json_shape_matches_the_frontend_types`
   (`crates/core/tests/scan_export.rs`) catches part of the drift.
8. **The mock follows the engine.** `app/src/lib/mock-tax.ts` mirrors the core tax engine
   (simplified figures, same contract and behavior) and `lib/mock.ts` the other commands, so the
   whole UI works in a browser. Change the mock together with the engine.
9. **Language.** Everything committed is in English (code, comments, identifiers, file names,
   JSON keys, docs, commit messages), except what the user reads: UI text, user-facing errors
   and warnings, the sources and notes in `tax/br.json`, and `README.pt-BR.md` (the Portuguese
   copy of the README). Tax terms without a real translation keep their names (`das`, `fator_r`,
   `pro_labore`, `rbt12`). `docs/` is git-ignored: local notes that are never published.
10. **Isolated end-to-end runs.** Development builds share the identifier with the installed app,
    so by default they open the user's real data. Every run of the real app for testing (by hand
    or automated, e.g. Playwright over WebView2) must point `NFSETABLE_DATA_DIR` and, for the
    WebView2 storage on Windows, `WEBVIEW2_USER_DATA_FOLDER` to throwaway folders, and delete
    them afterwards:

    ```powershell
    $env:NFSETABLE_DATA_DIR = "$env:TEMP\nfsetable-e2e\data"
    $env:WEBVIEW2_USER_DATA_FOLDER = "$env:TEMP\nfsetable-e2e\webview"
    ```

    Never clear localStorage, write store documents, or move or back up the user's real data
    folder or WebView profile as a workaround. To drive the app over the Chrome DevTools Protocol,
    note that recent WebView2 runtimes (seen with 154.0.4258.48) ignore
    `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`: build a test-only binary with an extra Tauri config
    (`npm run tauri build -- --config <file>`) that sets `additionalBrowserArgs` with
    `--remote-debugging-port=<port>` on the `main` window and `"bundle": {"active": false}`, into a
    separate `CARGO_TARGET_DIR`. Never ship it or commit that config.
11. **Estimates, not advice.** Tax output is an estimate for planning. Keep the pt-BR warnings and
    sources of `TaxReport` and the "Não substitui um contador" notice in the UI; never present
    results as accounting advice.

## Reuse before you write

AI agents love to write a fresh helper next to one that already exists. Don't. Before adding a
function, search for it (`rg -n "fn <name>|function <name>"`, and the table below). One helper per
job in each layer: extend it with a parameter instead of writing a near-copy, and when you find a
copy, fold it into the original. For example, never add `sum_abc(a, b, c)` next to `sum(a, b)`:
make `sum` take a slice or an iterator and delete the variant. The same goes for a function that
only differs in its input type or format (cents vs. decimals, `mar/26` vs. `mar/2026`): one
function, with a parameter or a conversion at the call site. Small lambdas that rebuild the same thing in several components
(e.g. a month label) count as copies: move them to `lib/format.ts`. The only duplication on
purpose is across layers: `lib/mock.ts` and `lib/mock-tax.ts` mirror the engine, and text
normalization exists once in Rust and once in TS.

| Job | Rust (`crates/core/src`) | TypeScript (`app/src/lib`) |
|---|---|---|
| accent- and case-insensitive text | `text::normalize`, `text::compact`, `text::fold_char` | `normalizeText` (`format.ts`) |
| amounts in document text | `parse::find_money`, `parse::parse_money` | (the engine reads them) |
| amounts typed by the user | — | `parseMoneyInput` |
| format money | `parse::format_brl`, `parse::format_decimal`, `parse::format_decimal_with` (separator); `tax::money::brl` wraps `format_brl` for decimals | `formatBRL`, `formatAmount`, `formatBRLCompact`, `formatBRLPlain` |
| dates in document text | `parse::find_dates`, `parse::parse_date`, `parse::days_in_month` | `monthOfDate` |
| months `yyyy-mm` | `tax::month::Month` (`parse`, `parse_month_or_date`, `plus`, `months_until`, `display`, `iso`) | `isMonthKey`, `addMonths`, `parseMonthInput`, `maskMonthTyping`, `formatMonth*` |
| amounts by month, costs of a month | `tax::revenue::sum_by_month`, `revenue::annualize`, `Input::costs_in` | — |
| Fator R and annex | `tax::simples::fs12`, `simples::fator_r_and_annex`, `Das::outside_iss`, `tax::mei::it_blocked` | — |
| rounding in the tax engine | `tax::money` (`round_money`, `ceil_money`, `trunc2`, `to_cents`, `from_cents`, `percent`) | — |
| file name patterns (`*`, `?`) | `profile::name_matches` (native glob, no regex) | `compileNamePatterns` (compile once, test many), `nameMatches` (`rules.ts`) |
| regex that comes from data (profile rules) | `regex_cache::compiled` | — |
| file names and paths | `scan::file_name` | `pathKey`, `baseName`, `fileStem`, `shortenPath` |
| plurals and labels | `text::count_label` | `plural` (`format.ts`), `labels.ts` |
| JSON files in the data folder (app) | `app/src-tauri/src/files.rs`: `slug_path`, `write_atomic`, `remove_if_exists` | — |
| PDFium in tests, examples and debug builds | `dev_pdfium_dir()`; integration tests share `crates/core/tests/common` (`engine`, `fixture`) | — |

## Regex

Regex is the last option, not the first: agents reach for it by reflex and the code fills up with
patterns nobody can review.

1. **Native first.** Rust: `starts_with`, `ends_with`, `contains`, `split`, `trim`, `replace`,
   `char::is_ascii_digit`, `str::parse`, `serde_json`. TS: `startsWith`, `endsWith`, `includes`,
   `split`, `trim`, `replaceAll`, `Number()`, `Intl`, `JSON.parse`. And the helpers above before
   either. Never parse nested structure (HTML, XML, JSON) with regex, and never read invoice values
   with regex over the text flow (hard rule 5).
2. **The form, when regex is the answer.**
   - Rust: compiled once in a `static` (`LazyLock<Regex>`, as in `parse.rs`), never inside a
     function that runs per line, per file or per rule; patterns that come from data (the anchor
     rules of profiles) go through `regex_cache::compiled`; a raw string `r"…"`; named groups
     `(?P<name>…)`; one comment line above saying what it accepts; `(?x)` with comments past about
     40 characters. To validate a whole string use `^…$` (in the `regex` crate `$` is the real end
     of the text unless `(?m)` is on).
   - TS: a module-level `const NAME_RE = /…/` with the same comment line, named groups
     `(?<name>…)`, `^…$` for a whole string, the `u` flag for non-ASCII text, and never
     `new RegExp(text)` from user input without escaping it (see `compileNamePatterns`).
3. **The engines differ.** The Rust `regex` crate runs in linear time (no ReDoS, which is why the
   patterns users save in profiles are safe) but has no lookaround or backreferences, which is why
   anchor rules carry an `excludeSuffix`. JavaScript's `RegExp` backtracks: no ambiguous nested
   quantifiers (`(a+)+`, `(\w+\s*)+`, `(.*,)*`, `(a|aa)*`), prefer `[^x]+` to `.+?` (lazy is not a
   fix), and cap the length of user text before matching it.
4. **Tests.** A regex that encodes a business rule (money, dates, CNPJ, ids, file patterns) lives in
   `crates/core` when it can and gets a table test with at least 5 accepted and 5 rejected inputs,
   including the near-misses that motivated it. No regex101 links: the table is the documentation.
5. **Don't churn.** Don't rewrite a working pattern only because it looks dense; density is not a
   defect, ambiguity and undocumented intent are.

## Product decisions to respect

- **MEI:** no pró-labore anywhere. The Resumo shows a month-by-month table with revenue, taxes,
  expenses and profit.
- **ME** (Simples Nacional or Lucro Presumido): a simple pró-labore card in the Resumo and a
  "Pró-labore" tab with INSS/IRRF and, in the Simples, each month's Fator R (from the revenue of
  its 12 previous months) and the minimum pró-labore that keeps that month in Anexo III. The
  automatic pró-labore is the smallest amount that keeps every month considered in Anexo III.
- **Cash flow:** each Simples month has its own RBT12, Fator R and annex (`tax/window.rs`).
- **No MEI exit model.** The old start-of-pró-labore simulator was removed on purpose; the
  MEI → ME transition is for the accountant. Do not bring it back without a roadmap decision.

## Where to look

| I want to… | Go to |
|---|---|
| support a new invoice layout | `crates/core/profiles/` + `profile.rs`, fixture in `gen_fixtures.rs`, test in `tests/extraction.rs` |
| change the automatic heuristic | `AUTO_*_LABELS` in `crates/core/src/extract.rs` (any change in what a PDF reads to: bump `EXTRACTION_VERSION` in `engine.rs`) |
| change how lines and columns are built | `crates/core/src/layout.rs` |
| update tax values (new year) | `crates/core/tax/br.json` + `crates/core/tests/tax.rs` (+ `mock-tax.ts`) |
| change a tax calculation | `crates/core/src/tax/` + `tests/tax.rs` + `app/src/lib/mock-tax.ts` |
| add a command | `model.rs` → `commands.rs` → `main.rs` → `types.ts` and `api.ts` (+ `mock.ts`) |
| change an export format | `crates/core/src/export/` + `tests/export.rs`, `tests/scan_export.rs` |
| change saved data or the data folder | `app/src-tauri/src/store.rs`, `data_dir.rs`; `app/src/lib/persist.svelte.ts`, `datadir.svelte.ts` |
| change companies or the startup picker | `app/src/lib/companies.svelte.ts`, `components/CompanyPicker.svelte` |
| change the tax page | `app/src/lib/tax.svelte.ts`, `components/Tax*.svelte` |
| change UI text | components in `app/src/components/` and `lib/labels.ts` |

## Before you finish

- Run the checks above; add or update tests for what changed.
- Keep `model.rs`/`types.ts` and the engine/mock pairs in sync.
- Update README.md and README.pt-BR.md when behavior changes.
- Read your diff for personal data before handing it over.
