---
name: Invoice not read
about: An NFS-e layout the app does not recognize
labels: layout
---

_You can write in English or Portuguese._

**Do not attach the invoice.** Describe the layout without personal data.

- City / issuer of the invoice:
- Is it the national-standard DANFSe or a city hall's own model?
- Label shown near the amount (e.g. "Valor Líquido", "Total a pagar"):
- Is the amount to the right of the label, below it, or somewhere else?
- Does the invoice have selectable text? (if not, it is a scanned image)

If you already marked the amount in the app and saved a profile, you can paste the profile's JSON
here. It holds positions and label texts, but check its `fingerprint` and `namePatterns` first and
remove any CNPJ, names or other personal data a rule may use to recognize a document.
