# Milestone 1 engineering qualification evidence

Review date: 2026-11-01 (America/Chicago).

Source: `dbdb8c42ee97a9cc51659f6c0d060780300ae48d` on
`codex/milestone-1-qualification`.

[Successful CI run](https://github.com/saikatbib/SeqQC/actions/runs/41376946382).
All seven jobs passed: repository policy, three production-feature native-test
jobs, and three synthetic native qualification jobs.

## Platform results

| Platform | Production tests | Native scenes | Runtime observation |
| --- | --- | --- | --- |
| Windows Server 2025 x86-64 | Passed | 42 PNGs, pinned Mesa software renderer | No TCP/UDP endpoints sampled in process family for 10 seconds |
| macOS 15 ARM64 | Passed | 42 PNGs, native OpenGL | No TCP/UDP endpoints sampled in process family for 10 seconds |
| Ubuntu 24.04 x86-64 | Passed | 42 PNGs under Xvfb | No AF_INET/AF_INET6 operation in scene syscall traces or sustained startup trace |

Every scene covers one of seven screens in three themes at two text scales.
The frames and their executable/hash manifests are downloadable as
`m1-qualification-windows-2025`, `m1-qualification-macos-15`, and
`m1-qualification-ubuntu-24.04` artifacts from the run. Local copies are under
`target/qualification/ci-verified/`; these are generated artifacts, not source.

Qualification executable SHA-256 values:

- Windows: `b9852af1feb0a3d7a300aa95d6fcb9158771075d0e985b25f6b271a1783daec1`
- macOS: `e3510dd3c6661a0522acb79cf564053b390efe7384109c8d6079dbf5250a7a48`
- Linux: `2396299d89eea419497e9fd7faa5924baac02215fcd046ed6539e83c0d6ac41e`

These hashes identify synthetic qualification builds, not distributable release
artifacts. Mesa DLL hashes and the verified archive digest are in the Windows
artifact's `test-driver.json`. Mesa is a CI test dependency only.

The local default-feature release was restored at
`target/release/seqqc-native-desktop.exe`, SHA-256
`7d90283ee9d55749fe5cac8d67a4709c452af3e312e402fc487e3472bbf077eb`.
Its fixture marker is absent. Production workspace tests passed (46 tests,
one intentionally ignored interactive credential-store test). The final
source-archive metadata/policy follow-up passed six Node tests and repository
policy checks; it does not change native runtime source after the qualified
commit above.

## Failures found and corrected

The [first run](https://github.com/saikatbib/SeqQC/actions/runs/41241645907)
passed production tests on all platforms but exposed missing qualification
infrastructure:

- Linux lacked `libxkbcommon-x11`; the graphical prerequisite list now includes it.
- The macOS observer's awk membership lookups created false-valued keys, which
  incorrectly included unrelated runner processes. The extracted process-family
  implementation emits only true members. Its regression test excludes
  ancestors, siblings, and PID zero and passed on macOS/Linux.
- Windows hosted graphics exposed OpenGL 1.1. A checksum-pinned Mesa 26.2.5
  software renderer now supports that job. Local Windows workstation captures
  also succeeded without installing this driver.

Native capture initially used an after-swap helper that produced blank/partial
frames. It was replaced with a pre-swap egui screenshot event and a blank-frame
guard. The production feature set excludes fixture setup and capture code.

## Visual and accessibility engineering review

Inspected Windows native frames at 100%/200%, with additional macOS and Linux
artifact samples. Layout/contrast defects found during iteration were corrected:
light-theme status colors, selected-control contrast, header overlap at 200%,
the non-scrolling locked form, input-path overflow, and wide table overflow.
Palette regression checks cover 4.5:1 ordinary-text contrast on the configured
panel/control/selection backgrounds in all themes.

Live Windows AccessKit/UI Automation and keyboard observations used disposable
synthetic profiles only; no user credentials were entered:

- Named theme and text-size controls were keyboard reachable.
- Reverse Tab navigation reached sample-specific file-evidence buttons and the
  explicitly named filter field.
- Typing `orphan` left exactly one matching group with `Needs review` intact.
- Enter opened evidence containing `orphan_R1.fastq` and pairing provenance;
  Escape closed it. The UI Automation focus returned to the root, and reverse
  Tab navigation could continue; formal screen-reader review should assess this
  focus-return behavior.
- System readiness opened from keyboard navigation. Reference/annotation/target
  actions and simulator controls exposed meaningful names.
- The simulator entered `Running: simulated input review`, then completed with
  `Completed — simulation only; no QC results`. Its short duration completed
  before the attempted live keyboard cancellation; cancellation correctness is
  covered by the state-machine tests, not claimed as a live user-study success.
- A synthetic session auto-locked after inactivity and removed project controls
  from the accessible tree. No attempt was made to automate an unlock dialog.

Desktop screenshot capture and coordinate-based automation remained unreliable
on this host. Native framebuffer captures and keyboard-based interactions were
used instead. UI Automation field-setting also failed, so the successful filter
check used actual keyboard input after observed focus.

## External-provider dossier

All eight candidates remain pending and external execution is disabled. The
official SAMtools 1.25 source archive was downloaded without executing code;
its locally computed SHA-256 matched the upstream value:
`8bbd2b3cb447953149a74ab0b00e3ec4e88a789a27b8373a31fa6b5b2dee02b1`.
The archive contains the main SAMtools, bundled HTSlib, and LZ4 license files.
This establishes source-archive integrity and a license inventory only. It does
not approve a built executable, transitive runtime closure, scientific method,
redistribution, or clinical/institutional use.

## Acceptance boundary

The automated platform engineering matrix is passed for the scope above.
Milestone 1 is not thereby independently approved or deployment-qualified.
Still required:

- Representative-user sessions in the three specified roles, and spoken
  Narrator/VoiceOver/Orca review with actual users.
- Named independent scientific/security/licensing approvers for each proposed
  external executable and its exact build/runtime closure. No external executable
  has been selected, installed in the app, or approved by this evidence.
- Clean end-user installation/uninstallation, real GPU/display variants,
  network-share crash/locking behavior, and full ordinary/failure/recovery
  workflow observations on intended deployment hosts.
- Corroborating network-attempt evidence where the current platform check only
  samples endpoints. Endpoint sampling cannot exclude brief attempts.

The [review protocol](../m1-qualification-protocol.md) contains the unfilled
participant and approval forms. No representative participants or independent
reviewers have been identified in this session, so their decisions are not
fabricated or inferred from CI.
