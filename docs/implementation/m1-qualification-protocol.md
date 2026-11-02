# Milestone 1 qualification and independent review protocol

Prepared 2026-10-31. This is an executable engineering protocol plus a human
review form. An unfilled form is not approval or evidence of a user study.

## Automated platform matrix

The `milestone-one-qualification` job in `.github/workflows/ci.yml` builds an
opt-in synthetic qualification binary on Windows x86-64, macOS ARM64, and Linux
x86-64. The ordinary `native-test` job tests the production feature set.

`qualification` is disabled by default. It requires an explicitly marked,
disposable directory and uses only fixed synthetic profiles and fixtures.
Never distribute that binary as the application. Native framebuffer capture
uses an egui screenshot event before buffer swap; solid blank frames fail.

The screenshot matrix covers locked/create-profile, overview, project creation,
input review, readiness, recovery, and the last page of 10,000 synthetic sample
rows, in night/light/high-contrast themes at 100% and 200% scale. The large UI
fixture is generated presentation state; the separate discovery benchmark
actually scans 20,000 files. PNG hashes, dimensions, platform, and executable
hash are recorded in `results.json` alongside the images.

Linux uses `strace` for internet-family syscall attempts in each scene and a
sustained runtime interval under Xvfb. Windows and macOS sample process-family
TCP/UDP endpoints during a sustained interval and reject early application exit.
Endpoint polling cannot rule out short attempts between samples. These are
runner-based engineering checks, not signed-installer, physical display/GPU,
network-mount, or deployment qualification.

Windows hosted runners expose only legacy OpenGL 1.1. Their visual test job
therefore uses a checksum-pinned Mesa 26.2.5 MSVC software renderer, deployed only
beside the disposable qualification executable. No system driver settings are
changed and no Mesa binaries enter release packages. The artifact manifest
records the archive and copied DLL hashes. The ordinary Windows workstation
captures use the installed driver instead. This distinguishes software-renderer
CI evidence from native GPU/deployment qualification. See the
[Mesa LLVMpipe documentation](https://docs.mesa3d.org/drivers/llvmpipe.html) and
[the pinned Windows distribution](https://github.com/pal1000/mesa-dist-win/releases/tag/26.2.5).

## Live accessibility and visual review

Use a synthetic qualification binary without `SEQQC_QA_SCREENSHOT` to retain
the interactive window. Never enter user credentials in automated tests.

For every platform and theme:

1. Inspect the screenshot for clipped/overlapping text, missing controls,
   misleading statuses, and unreadable paths at both scales.
2. Use Tab/Shift+Tab/Enter/Space to reach navigation, input filtering, page
   controls, file evidence, reference selection, and simulation queue/cancel.
   Record focus order and whether focus is visibly identifiable.
3. Inspect accessible names of fields, buttons, checkboxes, and statuses. Test
   Narrator on Windows, VoiceOver on macOS, and Orca on Linux with an actual
   screen-reader user. UI Automation/AccessKit trees alone do not certify spoken
   output or usability.
4. Verify that text labels communicate every status without depending on color.
   Automated palette checks target 4.5:1 for ordinary text, including semantic
   labels and selected controls, following
   [WCAG contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
5. Verify 200% scaling with all primary actions reachable by scrolling or reflow;
   a large screenshot alone is not a pass.

## Representative-user formative study

Recruit at least one participant in each role from the planning baseline:
beginner/wet-lab, core/bioinformatics, and controlled-workflow. The engineer must
not substitute themselves for these participants. Use no patient information.

| Task | Expected safe behavior |
| --- | --- |
| Create/open a synthetic project | Correct assay/context, encrypted save, correct project on reopen |
| Inspect paired, orphan, and duplicate groups | Identify proposals versus confirmed facts; retain unresolved status |
| Find the final synthetic sample | Search or navigate to the final page and inspect its files |
| Configure a missing reference | Distinguish inaccessible/unconfigured from scientific incompatibility |
| Queue/cancel the simulator | Explain that no biological result or diagnostic finding was produced |
| Reopen interrupted simulated work | Identify interruption and safely restart |
| Encounter an active writer | Decline recovery of an active project |
| Lock and reopen | Verify drafts and metadata from another project are not shown |

Record task completion, time, assistance, mistakes, interpretation, and severity.
Acceptance requires no unresolved serious input-mapping/recovery error. Record
the prespecified study threshold and rationale before recruiting; the later
version-1 90% completion target is not a fabricated Milestone 1 study result.

Participant ID: ___  Role: ___  Platform/build hash: ___  Date: ___

Tasks completed/attempted: ___  Assistance: ___  Serious errors: ___

Keyboard/screen-reader/scaling findings: ___  Remediation and retest: ___

Independent usability reviewer: ___  Decision/date: ___

## External provider approval

All eight external candidates are pending in `config/provider-approvals.json`.
There are no approved executable artifacts and external execution remains off.
Repository checks reject enabling it or replacing a pending decision with an
unsubstantiated approval. Built-in discovery and simulation do not require an
external scientific provider.

The first technical dossier contains upstream records for
[SAMtools 1.25](https://github.com/samtools/samtools/releases/tag/1.25) and
[fastp v1.4.0](https://github.com/OpenGene/fastp/releases/tag/v1.4.0).
SAMtools' upstream release API reports source-archive SHA-256
`8bbd2b3cb447953149a74ab0b00e3ec4e88a789a27b8373a31fa6b5b2dee02b1`;
the downloaded source archive was rehashed locally on 2026-11-01 and matched.
Its license inventory includes the main, bundled HTSlib, and LZ4 license files.
This source check is not an executable digest or independent approval. The fastp release API
lists no attached assets. Neither source record approves a platform binary.

Review each proposed executable and its full runtime closure for exact version,
artifact provenance/hash/signature, licenses/notices, offline behavior, allowed
platform/architecture, adapter safety, scientific tolerances and fixtures,
resource limits, and update/revalidation policy. SAMtools/HTSlib remote-resource
capabilities require specific review. Do not execute a candidate merely to
discover its version before provenance/security checks.

The pinned license references are
[SAMtools LICENSE](https://github.com/samtools/samtools/blob/1.25/LICENSE) and
[fastp LICENSE](https://github.com/OpenGene/fastp/blob/v1.4.0/LICENSE).
Transitive-license and scientific approval are still separate work.

Artifact/version/hash/platform: ___  Build/SBOM evidence: ___

Scientific reviewer/evidence/decision/date: ___

Security/licensing reviewer/evidence/decision/date: ___

Deployment owner/intended-use approval/date: ___

## Additional deployment checks

On representative clean hosts, verify signed/unsigned installer behavior,
first-launch and uninstall, offline failures, keyboard/display/GPU behavior,
and interrupted migrations. On each supported network filesystem, use synthetic
projects to test concurrent writers, disconnect/reconnect, crash residue, and
stale-lock evidence. Keep results separate from local-filesystem CI evidence.
