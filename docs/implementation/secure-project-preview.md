# Secure project preview (`0.1.0-alpha.3`)

SeqQC Studio now provides the first working encrypted project workflow in the
native Windows, macOS, and Linux application.

## First launch

1. Start SeqQC Studio while signed in to the intended operating-system account.
2. Enter a local display name.
3. Create a local passphrase of at least 12 characters and confirm it.
4. Store the passphrase in the institution's approved offline password manager.

SeqQC has no online account and cannot reset this passphrase. The passphrase is
used only to unwrap a random profile key. It is cleared from the UI buffer after
each unlock attempt and is never saved to disk.

## Create a project

1. Open **Projects** and choose **New encrypted project**.
2. Enter the encrypted display name, assay, and clinical/research context.
3. Choose an existing parent folder. SeqQC creates an opaque folder such as
   `SeqQC-a1b2c3d4.seqqc`; the display name is not exposed in that folder name.
4. Optionally choose a FASTQ/BAM/CRAM input folder.
5. Select **Create and open project**.

The outer `project.json` contains only non-sensitive unlock metadata. Project
names, contexts, paths, recents, audit records, and random project keys remain
inside encrypted SQLCipher databases.

## Reopen and lock

- Unlock the local profile with the local passphrase.
- Open a project from the encrypted recent list or choose its `.seqqc` folder.
- Only one writer can open a project at a time.
- **Lock session** immediately closes the project and encrypted profile and
  releases their writer leases.
- The application locks automatically after 15 minutes without user input.

## Current limitations

- Losing the passphrase currently makes the profile and its project keys
  unrecoverable.
- Copying a `.seqqc` directory alone does not transfer its key. The separately
  reviewed encrypted portable-archive workflow is not implemented yet.
- Crash-time stale-lock recovery is implemented in the core but is not yet
  exposed as a guided UI recovery flow.
- Cryptographic formats and parameters remain engineering-preview material
  pending independent review. This release is not validated for diagnosis,
  clinical decisions, or regulated production use.
