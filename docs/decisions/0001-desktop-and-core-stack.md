# ADR 0001: Desktop and core technology stack

- Status: Superseded by ADR 0009 for the production desktop shell
- Date: 2026-08-26
- Owners: Project owner and SeqQC architecture

## Context

SeqQC Studio must provide a professional Windows/macOS/Linux desktop interface,
process very large local sequencing files, isolate scientific execution from
presentation, and make the absence of runtime networking verifiable.

## Decision

- Use Tauri 2 as the desktop shell.
- Use React 19 with TypeScript for the packaged user interface.
- Use Rust stable for the trusted application core, workflow engine, discovery,
  security-sensitive operations, normalized metrics, and native QC modules.
- Keep UI ↔ core commands typed and narrow. The UI does not access the project
  database or launch scientific tools directly.
- Use OS webviews only to render packaged application assets. Do not include a
  general web server or remote frontend.
- Pin exact toolchain and dependency versions in lockfiles at scaffold time.

Tauri's capabilities model is relevant because it permits a deliberately small
frontend permission surface. Only explicitly referenced capability files will
be enabled. Shell access will not be granted to frontend code.

## Consequences

- Native builds must be produced and tested on each target operating system.
- OS webview differences require visual and accessibility tests.
- Rust/Tauri toolchains must be provisioned explicitly; the application must not
  install them at runtime.
- UI code remains replaceable without changing scientific/storage contracts.
- Release builds omit updater/network plugins and remote assets.

## Alternatives considered

- Electron: broad ecosystem but larger runtime and attack/packaging surface.
- PySide/Qt: viable scientific ecosystem but more complex licensing/packaging
  review and less direct fit for the planned dashboard.
- Local browser application: rejected because the product requirement is a
  desktop-only experience and it complicates local service exposure.
- Single-language UI/core: rejected because it weakens either the modern UI or
  large-file/safety goals.

## Evidence required during Milestone 0

- Empty Tauri application builds offline after dependencies are vendored/cached.
- Capabilities expose only approved project dialogs and core commands.
- Packaged UI attempts no network connection.
- A 10,000-sample synthetic view remains responsive.

## Supersession note

The Windows package failed the runtime-network criterion on 2026-08-27.
Descendant `msedgewebview2.exe` processes opened non-loopback HTTPS connections
despite the application's deny-connect CSP and absence of network plugins.
ADR 0009 therefore limits this stack to an engineering preview and requires a
non-system-webview production shell unless an independently verified containment
mechanism can demonstrate zero attempted network activity.
