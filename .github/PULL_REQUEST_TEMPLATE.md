## What

<!-- What was ported, fixed or changed, with the original addresses where it applies. -->

## Checks

- [ ] `make check` passes (writer gate)
- [ ] `make verify` IDENTICAL on every recording, or the difference is recorded in the divergence ledger
- [ ] Scenario / repro with an empty blocker list (for native fixes)
- [ ] Follows ADR-003: no duplicated code, shared logic in its owning crate, platform `cfg`s only in backends
- [ ] WORKLOG entry
