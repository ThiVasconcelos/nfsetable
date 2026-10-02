English · [Português](README.pt-BR.md)

# nfsetable

**Turn your NFS-e PDF invoices into a table with totals, and see the taxes and financial health
of your MEI or Simples Nacional company. Everything runs on your computer.**

> **Made for Brazil.** nfsetable reads Brazilian service invoices and estimates Brazilian taxes,
> and the app's interface is in Brazilian Portuguese.

nfsetable is a free, open-source desktop app for Brazilian freelancers and small companies. Point
it at the folder with your NFS-e PDFs (Nota Fiscal de Serviço eletrônica, Brazil's electronic
service invoice), check the values it reads and get the total of any selection: no spreadsheet, no
back-of-the-envelope math. A tax planning page turns that revenue into estimates for the MEI and
the Simples Nacional, with the pró-labore, your costs and what is left at the end of each month.
It runs on Windows, Linux and macOS and is built with Tauri 2, Rust, PDFium and Svelte 5.

## Privacy

- **100% local.** The app never goes online. No account, no telemetry, no data collection.
- Your PDFs never leave your computer. Profiles, edits and tax planning are saved as files in the
  app's data folder, which you can choose.

## Features

**Companies**

- One or more companies (for example, your one-person company and a SaaS with partners), each with
  its own invoice folders, edits and tax planning. Classification rules are shared by all of them.
- At startup you pick the company (with only one, the app opens it directly). That screen shows the
  data folder and has a gear to change it. In the settings you can choose to always open the last
  one used; you can also switch companies from the top bar.

**Invoices**

- Reads whole folders (with or without subfolders) and single files. Remove items one by one or all
  selected at once, and ignore files by name (by default, those containing "cancelada").
  What was read is kept, so the next time the app opens only new or changed files are read.
- Finds the **net value** (valor líquido), the **service value** (valor do serviço, the gross
  amount) and the **competence month** (competência, the month the service belongs to for tax
  purposes) in:
  - the national-standard DANFSe (Documento Auxiliar da NFS-e, the invoice's PDF), both the layout
    used until 2025 and the 2026 layout with IBS/CBS (the new taxes of Brazil's tax reform). Every
    MEI and every city on the national standard issues it;
  - other layouts, by looking for labels such as "Valor Líquido", "Valor dos Serviços" and
    "Data de emissão".
- A table with file, type, competence, value, status and origin: sort by any column, select ranges
  with Shift, and see the grand total, the selection total and totals by type. Duplicates and
  invoices typed "Cancelada" are listed but left out of the total.
- **Period** filter by competence (this month, this year, from MM/YYYY to MM/YYYY…), applied to
  the cards, the table, the totals and the export.
- PDF preview with the value highlighted, so you can check where it came from.
- **Teach it where the value is**: on an invoice that was not read, draw a rectangle around the
  value, test it on the other failed invoices (or on all of them), apply it and save it as a profile
  for next time.
- **Revenue and expenses**: besides the invoices you issue, the table takes the PDFs of your bills
  (internet, rent, your accountant's NFS-e…). Profiles recognize each document by file name (e.g.
  `*internet*`) or by its text (e.g. the issuer's CNPJ, the company tax ID) and set its type and
  whether it is revenue or an expense; you can also mark it by hand.
- Manual editing of value, type, competence and nature (revenue or expense).
- **Export** to CSV, Excel (XLSX), PDF and SQL (PostgreSQL and MySQL), plus "copy total".

**Taxes (planning)**

- Pick the months to consider: their average becomes the "typical month" (mês típico) of the
  estimates. The first months of a new company are enough to estimate the whole year.
- **Projection**: type how much you expect to receive in any month, even without an invoice, and
  choose month by month whether the invoices' total or the forecast counts. It works even with no
  invoice loaded.
- **Month by month in the Resumo** (summary): a small table with each month's revenue, taxes,
  expenses and profit; for an ME also the operating profit (revenue − taxes − expenses, before the
  pró-labore) and the pró-labore.
- **MEI** (Microempreendedor Individual, the regime for individual micro-entrepreneurs, with a
  fixed monthly tax): the year's revenue limit, a projection at the current pace and the DAS-MEI,
  the monthly tax slip. The MEI has no pró-labore, so none is shown.
- **Simples Nacional** (the simplified tax regime for micro and small companies, ME/EPP): Fator R,
  annex (III or V), bracket, effective rate, DAS (the monthly tax slip) and its split by tax.
- **Pró-labore** (the monthly pay a partner draws from the company), for an ME in the Simples
  Nacional or in the Lucro Presumido (the presumed-profit regime):
  - a simple card in the Resumo;
  - a "Pró-labore" tab with the INSS (social security) and IRRF (income tax withheld at source,
    with the Lei 15.270/2025 reduction) breakdown and the net amount;
  - in the Simples, the tab also shows each month's **Fator R** (payroll divided by revenue over
    12 months; at 28% or more, activities such as software development are taxed in Anexo III
    instead of the higher-rate Anexo V), computed from the actual revenue of the 12 months before
    it, and the minimum pró-labore that keeps that month in Anexo III. The automatic pró-labore is
    the smallest amount that keeps every month considered in Anexo III (never below the minimum
    wage).

  The calculation assumes the pró-labore was paid in all of those months; the app does not model
  leaving the MEI, which is a job for your accountant.
- Comparison of **MEI × Simples III × Simples V × Lucro Presumido**, "sobra do mês" (what is left
  each month) and "quanto cobrar" (how much to charge).
- **Costs and reserve**: fixed costs (monthly or yearly, with a start month and, optionally, an end
  month), variable costs by month (typed by hand or taken from expense documents), a reserve as a
  percentage of revenue, and the revenue base of the "sobra do mês" (the average, the average
  without invoices typed "Bônus", or a fixed amount). The Resumo uses the costs of the reference
  month (the last month considered): the fixed costs in effect in it, at full value, and its
  variable costs. In the cash flow, each month has its own.
- **Fluxo mês a mês** (month-by-month cash flow): revenue (from invoices or forecast) − taxes −
  costs − reserve = what is left each month. In the Simples, each month is computed with its own
  RBT12 (the revenue of its 12 previous months), Fator R and annex.
- Everything you type is saved as files in the app's data folder, which you can switch to another
  one (for example, a synced folder for backups).
- CNAE (activity code) picker with the annex rule of each IT activity.

The tax tables cover 2025 and 2026 and live in
[crates/core/tax/br.json](crates/core/tax/br.json), where each block cites its legal basis (the app
lists the sources under "Avisos e fontes"). Everything is an estimate for planning and **does not
replace an accountant**.

## Download

Installers are on the [Releases](../../releases) page:

| System | File |
|---|---|
| Windows 10/11 | `.msi` or `-setup.exe` |
| Linux | `.deb`, `.rpm` or `.AppImage` |
| macOS (Apple Silicon) | `.dmg` (unsigned: right-click → Open) |

## How value reading works

For each PDF the app tries, in this order:

1. **Built-in profiles.** The "DANFSe (padrão nacional)" profile recognizes the invoice by its text
   and reads the value right below the "Valor Líquido da NFS-e" label, in the same column. It skips
   the "Valor Líquido + IBS/CBS" column of the 2026 layout.
2. **Your profiles.** Rules you saved by marking the value on an invoice.
3. **Automatic.** Looks for known labels and takes the nearest R$ amount to the right or below.

Reading is based on the **position** of the text on the page (through
[PDFium](https://pdfium.googlesource.com/pdfium/), Chrome's PDF engine), not on the text flow.
That is why values from neighboring columns never get mixed up.

When you mark a rectangle, the app also stores the text closest to it (for example,
"Montante a pagar") and the offset to the value, so the rule keeps working when that block changes
height from one invoice to the next.

Scanned invoices (images only, with no selectable text) show up as "Sem texto" (no text).

## Building from source

Prerequisites:

- [Rust](https://rustup.rs) (stable) and [Node.js](https://nodejs.org) 22 or newer.
- **Windows:** WebView2 (already included in Windows 10/11) and the
  [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with C++.
- **Linux (Debian/Ubuntu):**

  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    build-essential curl wget file libssl-dev libxdo-dev
  ```

Steps:

```sh
# 1. Download PDFium for your platform into vendor/pdfium/
bash scripts/fetch-pdfium.sh             # Linux, macOS or Git Bash on Windows
# or, in PowerShell:
powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1

# 2. Install the UI dependencies
cd app
npm install

# 3. Run in development mode
npm run tauri dev

# 4. Build the installers (they land in target/release/bundle/)
npm run tauri build
```

To run only the UI, in the browser with sample data: `npm run dev` inside `app/`.

## Portable and test mode

The `NFSETABLE_DATA_DIR` environment variable forces the data folder (profiles, edits and
planning), for example to run the app from a USB drive or to test it without touching your data:

```sh
NFSETABLE_DATA_DIR=/path/to/test-data npm run tauri dev               # Linux, macOS, Git Bash
```

```powershell
$env:NFSETABLE_DATA_DIR = "C:\path\to\test-data"; npm run tauri dev   # PowerShell
```

Development builds share their data folder with the installed app, so set it whenever you try
things out. On Windows, the WebView keeps its own storage too: for a fully isolated test run, also
point `WEBVIEW2_USER_DATA_FOLDER` to a throwaway folder.

## Command line

The reading engine also works in the terminal, without the UI:

```sh
cargo run -p nfsetable-core --example extract -- ~/invoices/2026 --exclude cancelada --recursive
```

It lists the value of each invoice and prints the total at the end (the output is in Portuguese).
`--exclude` can be repeated, and `--recursive` includes subfolders.

## Repository layout

```
crates/core            reading engine (Rust): positioned text, rules, scan, export, taxes
crates/core/profiles   built-in profiles in JSON (e.g. the national-standard DANFSe)
crates/core/tax        tax tables and CNAE catalog, versioned by validity dates
app/                   UI (Svelte 5 + TypeScript)
app/src-tauri          desktop app (Tauri 2) that connects the UI to the engine
scripts/               PDFium download
```

How the pieces fit together (layers, directory map and rules) is in [AGENTS.md](AGENTS.md), the
starting point for contributors and coding agents.

## Contributing

Profiles for new invoice layouts, fixes and ideas are welcome; see the
[contributing guide](CONTRIBUTING.md). Issues and pull requests in Portuguese are welcome too.
The most important rule: **never attach real invoices** to issues, tests, fixtures or pull
requests. They carry names, CNPJ/CPF numbers, addresses and amounts.

## Disclaimer

nfsetable helps you organize and add up values and estimates taxes for planning, but it does not
replace an accountant and is not accounting or tax advice. Always check the values against the
original documents.

## License

[MIT](LICENSE). The PDFium library bundled with the app is licensed under BSD-3-Clause (its text
ships with the installers, in `LICENSE-pdfium.txt`).
