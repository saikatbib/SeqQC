# ADR 0010: Native egui production-shell spike

- Status: Accepted for Milestone 1 spike
- Date: 2026-08-27
- Owners: Project owner and SeqQC architecture
- Extends: ADR 0009

## Context

ADR 0009 rejects an uncontained system webview for the production shell after
Windows runtime observation found outbound connections from WebView2 child
processes. SeqQC still requires one cross-platform desktop experience that can
reuse the existing Rust discovery, format, security, execution, and reporting
crates without creating a runtime network capability.

The first native-shell slice must be small enough to validate promptly while
remaining capable of rendering the planned sample tables, findings,
dashboards, themes, guided explanations, and large-project views.

## Decision

- Spike `eframe`/`egui` 0.36.1 as a native-only Rust presentation shell.
- Disable default features and enable only native accessibility, bundled
  fonts, the native OpenGL renderer, Wayland, and X11 support. The first
  Windows compile rejected the `wgpu` candidate because its locked `wgpu-hal`
  and allocator path resolved incompatible `windows` crate interfaces.
- Do not enable link-opening, web, inspection-server, updater, HTTP, or
  framework persistence capabilities.
- Use `rfd` 0.17.2 only for local native folder/file dialogs. On Linux it uses
  the local XDG desktop portal.
- Keep presentation state separate from SeqQC domain crates. Long-running
  discovery and analysis work must execute off the UI thread and communicate
  through bounded application messages.
- Retain the Tauri/React program only as the Milestone 0 UX reference until the
  native shell reaches functional parity; it is not a production release path.

## Why this candidate

- The native executable has no browser or system-webview process dependency.
- `eframe` supports Windows, macOS, Linux, GPU rendering, keyboard input, and
  platform accessibility through AccessKit.
- MIT/Apache-2.0 licensing is compatible with SeqQC's Apache-2.0 repository.
- Immediate-mode rendering is a practical fit for dense, frequently changing
  scientific dashboards and keeps the spike within the existing Rust toolchain.

## Acceptance gates

This decision does not by itself establish strict-offline compliance. Promotion
from spike to production requires all of the following:

1. Locked dependency review shows no runtime HTTP client, updater, webview, or
   enabled inspection listener in the native-shell dependency graph.
2. Clean Windows, macOS, and Linux builds start and support folder selection.
3. Full process-family observation records zero attempted DNS, TCP, and UDP
   activity on all three platforms.
4. Screen-reader, keyboard-only, scaling, light/night/high-contrast, and large
   project navigation evidence passes the Milestone 1 thresholds.
5. Packaging, signing, crash recovery, and encrypted project lifecycle gates
   remain satisfied independently.

## Alternatives considered

- `iced`: viable MIT-licensed native candidate, but the egui spike provides a
  smaller migration step for the current dashboard-oriented preview.
- `Slint`: capable native toolkit, but its multi-license decision adds review
  work that is unnecessary for the first spike.
- Platform-specific UI frameworks: rejected for the first spike because three
  separate presentation implementations would slow functional and validation
  parity.
- Continue Tauri/React: rejected by ADR 0009 for a strict-offline production
  build.
