# SeqQC Studio planning baseline

**Status:** Frozen for implementation planning  
**Baseline:** 1  
**Product:** SeqQC Studio  
**Engine:** SeqQC

These documents define what the first stable release is expected to become.
They deliberately distinguish three scopes:

- **Version 1 scope:** capabilities that must exist before the stable release.
- **Early milestone scope:** the smaller, testable increments used to reach
  version 1.
- **Validated deployment scope:** features that help an organization validate a
  particular build but do not, by themselves, make SeqQC a regulated medical
  device or a clinically validated system.

## Decision control

The high-level product definition is frozen. A proposed change must document:

1. the requirement being changed;
2. scientific, security, user-experience, and compatibility effects;
3. migration effects on saved projects and reports;
4. validation work that must be repeated; and
5. the decision owner and approval date.

Minor clarification that does not change observable behavior may be made in
place. Everything else receives a decision record under `docs/decisions/` once
that directory is introduced.

The initial metric inventory is maintained in
[`02a-metric-registry.csv`](02a-metric-registry.csv) so stable IDs and scope can
be reviewed independently of narrative documentation.

## Normative language

“Must” identifies a release requirement. “Should” identifies the preferred
design unless testing reveals a better option. “May” identifies optional or
future behavior.
