# Contributing to SeqQC

Thank you for improving SeqQC. The project welcomes scientific, security,
accessibility, documentation, and engineering contributions.

## Ground rules

- Never commit real sequencing data, protected health information, credentials,
  project archives, or identifiable sample metadata.
- Do not describe a metric, threshold, or workflow as validated without the
  evidence required by `docs/planning/08-validation-strategy.md`.
- Preserve the offline boundary. New runtime network, updater, shell, or broad
  filesystem capability requires an accepted architecture decision and a
  security review.
- Keep alignment execution Linux-only until a decision record changes that
  constraint.
- Every behavior change needs tests and user-facing limitations when relevant.

## Development setup

Install Node.js 22 or newer, pnpm 11.19.0, Rust 1.98.0, and the native Tauri 2
prerequisites for your operating system. On Windows, Strawberry Perl is also
needed to build the vendored SQLCipher/OpenSSL dependency.

```text
pnpm install --frozen-lockfile
pnpm check
pnpm test
pnpm desktop:build
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Windows contributors may use `tools/invoke-rust.ps1` to load Visual Studio's
native toolchain before a Cargo command.

## Pull requests

Open a focused pull request against `main`. Complete the pull-request template,
link the relevant decision or issue, include synthetic fixtures only, and state
which platforms were tested. Changes to schemas, scientific methods, security,
or release packaging require a second reviewer familiar with that area.

By submitting a contribution, you agree that it is licensed under Apache-2.0.

