[English](README.md) · Português

# nfsetable

Suas NFS-e em PDF viram uma tabela com valores e totais, sem planilha e sem conta de padaria.
Aponte a pasta das notas, confira os valores e veja o total da seleção. Tudo roda no seu computador.

## Privacidade

- **100% local.** O app não acessa a internet, não tem conta e não coleta dados.
- Seus PDFs nunca saem do computador. Os perfis que você cria ficam na pasta de dados do app.

## O que ele faz

**Empresas**

- Uma ou várias empresas (por exemplo, o seu PJ e um SaaS com sócios), cada uma com as próprias
  pastas de notas, edições e planejamento de impostos. As regras de classificação valem para todas.
- Ao abrir o app você escolhe a empresa (com uma só, ele entra direto). Essa tela mostra a pasta
  onde ficam os dados e tem a engrenagem para trocá-la. Nas configurações dá para ir sempre para a
  última usada; a troca de empresa também fica no topo da tela.

**Notas**

- Lê pastas inteiras (com ou sem subpastas) e arquivos soltos. Você pode remover itens (um a um ou
  todos os selecionados) e ignorar arquivos pelo nome (por padrão, os que contêm "cancelada").
  O que já foi lido fica guardado: ao abrir o app de novo, só os arquivos novos ou alterados são lidos.
- Encontra sozinho o **valor líquido**, o **valor do serviço** (bruto) e a **competência**:
  - DANFSe do padrão nacional (layout até 2025 e layout 2026 com IBS/CBS), que é o que todo MEI
    e os municípios do padrão nacional emitem;
  - outros layouts, procurando rótulos como "Valor Líquido", "Valor dos Serviços" e "Data de emissão".
- Tabela com arquivo, tipo, competência, valor, status e origem: ordena por qualquer coluna,
  seleciona intervalos com Shift, mostra total geral, total da seleção e totais por tipo.
  Duplicadas e notas do tipo "Cancelada" aparecem, mas ficam fora do total.
- **Período** pela competência (este mês, este ano, de MM/AAAA até MM/AAAA…), que vale para os
  cards, a tabela, os totais e a exportação.
- Prévia do PDF com o valor destacado, para você conferir de onde ele saiu.
- **Ensinar onde fica o valor**: numa nota que não foi lida, desenhe um retângulo em volta do valor,
  teste nas outras notas que falharam (ou em todas), aplique e salve como perfil para as próximas vezes.
- **Receitas e despesas**: além das notas que você emite, a tabela aceita os PDFs das suas contas
  (internet, aluguel, NFS-e do contador…). Perfis reconhecem cada documento pelo nome do arquivo
  (ex.: `*internet*`) ou pelo texto (ex.: o CNPJ de quem emitiu) e definem o tipo e se é receita ou
  despesa; também dá para marcar à mão.
- Edição manual de valor, tipo, competência e natureza.
- **Exportação** em CSV, Excel (XLSX), PDF e SQL (PostgreSQL e MySQL), além de "copiar total".

**Impostos (planejamento)**

- Escolha os meses que quer considerar: a média deles vira o "mês típico" das contas. Dá para
  estimar o ano inteiro com os primeiros meses de uma empresa nova.
- **Projeção**: digite quanto espera receber em qualquer mês, mesmo sem nota, e escolha mês a mês
  se vale o total das notas ou o previsto. Funciona até sem nenhuma nota carregada.
- **Mês a mês no Resumo**: uma tabelinha com receita, impostos, gastos e lucro de cada mês; na ME
  também o lucro operacional (receita − impostos − gastos, antes do pró-labore) e o pró-labore.
- **MEI**: limite do ano, projeção no ritmo atual e DAS-MEI. No MEI não existe pró-labore, então
  ele não aparece em lugar nenhum.
- **Simples Nacional (ME/EPP)**: Fator R, anexo (III ou V), faixa, alíquota efetiva, DAS e repartição.
- **Pró-labore** (empresa ME, no Simples Nacional ou no Lucro Presumido):
  - um card simples no Resumo;
  - a aba "Pró-labore", com INSS, IRRF (já com a redução da Lei 15.270/2025) e o líquido;
  - no Simples, a aba mostra também o Fator R de cada mês, calculado com a receita real dos 12
    meses anteriores a ele, e o pró-labore mínimo que mantém aquele mês no Anexo III. O pró-labore
    automático é o menor valor que mantém todos os meses considerados no Anexo III (nunca abaixo
    do salário mínimo).

  A conta supõe o pró-labore pago em todos esses meses; o app não modela a saída do MEI, que fica
  com o contador.
- Comparativo **MEI × Simples III × Simples V × Lucro Presumido**, "sobra do mês" e "quanto cobrar".
- **Custos e reserva**: gastos fixos (mensais ou anuais, com mês de início e, se quiser, de fim),
  gastos variáveis por mês (lançados à mão ou vindos das notas de despesa), uma reserva em % do
  faturamento e a base da sobra do mês (média, média sem as notas do tipo "Bônus" ou um valor fixo).
  No Resumo entram os custos do mês de referência (o último mês considerado): os fixos que valem
  nele, com o valor cheio, e os gastos variáveis dele. No fluxo, cada mês tem os seus.
- **Fluxo mês a mês**: receita (das notas ou prevista) − impostos − custos − reserva = sobra de
  cada mês. No Simples, cada mês é calculado com o próprio RBT12, Fator R e anexo.
- O que você anota fica salvo em arquivos na pasta de dados do app, que você pode trocar por outra
  (por exemplo, uma pasta sincronizada para ter backup).
- Seletor de CNAE com a regra de anexo de cada atividade de TI.

As tabelas são de 2025 e 2026 e ficam em [crates/core/tax/br.json](crates/core/tax/br.json), com a
base legal de cada bloco (o app mostra as fontes em "Avisos e fontes").
É estimativa para planejar e **não substitui um contador**.

## Download

Os instaladores ficam na página de [Releases](../../releases):

| Sistema | Arquivo |
|---|---|
| Windows 10/11 | `.msi` ou `-setup.exe` |
| Linux | `.deb`, `.rpm` ou `.AppImage` |
| macOS (Apple Silicon) | `.dmg` (não assinado: abra com clique direito → Abrir) |

## Como funciona a leitura

Para cada PDF o app tenta, nesta ordem:

1. **Perfis embutidos.** O perfil "DANFSe (padrão nacional)" reconhece a nota pelo texto e lê o valor
   logo abaixo do rótulo "Valor Líquido da NFS-e", na mesma coluna. Ele ignora a coluna
   "Valor Líquido + IBS/CBS" do layout de 2026.
2. **Seus perfis.** Regras que você salvou ao marcar o valor numa nota.
3. **Automático.** Procura rótulos conhecidos e pega o valor em R$ mais próximo, à direita ou abaixo.

A leitura usa a **posição** do texto na página (via [PDFium](https://pdfium.googlesource.com/pdfium/),
o motor de PDF do Chrome), e não o texto corrido. Por isso valores de colunas vizinhas não se misturam.

Quando você marca um retângulo, o app guarda também o texto mais próximo dele (por exemplo,
"Montante a pagar") e a distância até o valor. Assim a regra continua funcionando quando o bloco
muda de altura entre uma nota e outra.

Notas escaneadas (só imagem, sem texto selecionável) aparecem como "Sem texto".

## Compilar a partir do código

Pré-requisitos:

- [Rust](https://rustup.rs) (stable) e [Node.js](https://nodejs.org) 22 ou mais novo.
- **Windows:** WebView2 (já vem no Windows 10/11) e o
  [Build Tools do Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/) com C++.
- **Linux (Debian/Ubuntu):**

  ```sh
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    build-essential curl wget file libssl-dev libxdo-dev
  ```

Passos:

```sh
# 1. Baixe o PDFium da sua plataforma para vendor/pdfium/
bash scripts/fetch-pdfium.sh             # Linux, macOS ou Git Bash no Windows
# ou, no PowerShell:
powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1

# 2. Instale as dependências da interface
cd app
npm install

# 3. Rode em modo de desenvolvimento
npm run tauri dev

# 4. Gere os instaladores (saem em target/release/bundle/)
npm run tauri build
```

Só a interface, no navegador e com dados de exemplo: `npm run dev` dentro de `app/`.

**Modo portátil / testes:** a variável de ambiente `NFSETABLE_DATA_DIR` força a pasta dos dados
(perfis, edições e planejamento), por exemplo para rodar de um pendrive ou testar sem mexer nos
seus dados.

## Linha de comando

O motor de leitura também funciona no terminal, sem interface:

```sh
cargo run -p nfsetable-core --example extract -- ~/notas/2026 --exclude cancelada --recursive
```

Ele lista o valor de cada nota e mostra o total no final.

## Estrutura

```
crates/core            motor de leitura (Rust): texto posicionado, regras, varredura, exportação, impostos
crates/core/profiles   perfis embutidos em JSON (ex.: DANFSe do padrão nacional)
crates/core/tax        tabelas tributárias e catálogo de CNAE, versionados por vigência
app/                   interface (Svelte 5 + TypeScript)
app/src-tauri          app desktop (Tauri 2) que liga a interface ao motor
scripts/               download do PDFium
```

Como as peças se encaixam (camadas, mapa de pastas e regras) está no [AGENTS.md](AGENTS.md), em
inglês.

## Contribuindo

Perfis para novos layouts de nota, correções e ideias são bem-vindos. Veja o
[guia de contribuição](CONTRIBUTING.md) (em inglês; issues e PRs em português são bem-vindos).
A regra mais importante: **nunca anexe notas reais** em issues, testes ou PRs.

## Aviso

O nfsetable ajuda a organizar e somar valores, mas não substitui um contador nem é
consultoria contábil ou fiscal. Confira sempre os valores com os documentos originais.

## Licença

[MIT](LICENSE). O PDFium distribuído junto com o app é licenciado sob BSD-3-Clause
(o texto vai junto nos instaladores, em `LICENSE-pdfium.txt`).
