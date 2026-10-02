**What changes**

**How I tested it**

- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `npm run check` and `npm run build` in `app/` (if the UI changed)
- [ ] Contract or tax changes: `crates/core/src/model.rs` and `app/src/lib/types.ts` stay in sync,
      the browser mock (`app/src/lib/mock-tax.ts`) follows the core, and new expectations in
      `crates/core/tests/tax.rs` are computed by hand in comments
- [ ] No real invoices or personal data in the PR (synthetic fixtures only)
