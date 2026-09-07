# Magic runtime crash candidate

This branch is an offline candidate, not a hardware-qualified installation.
The default installation remains rolled back. No keyboard was accessed during
this work; testing either half requires separate authorization and recovery
artifacts. Local evidence and build outputs are under `.handoff/` and are not
part of these commits.

The runtime apply evidence stops during keymap writes, before lighting writes.
It does not identify the faulting instruction or prove stack overflow. A host
replay through the actual Rynk service and storage task reproduces uncached
garbage collection doing 339,243 ready flash reads in one executor poll.
The storage adapter now yields after at most 32 flash operations. Repeated
bulk writes force collection and preserve serialized key actions, including
tap-hold profile indices, after reopening the same flash bytes.

Shared lighting engines now live in static storage and the processor borrows
them. An explicit command call boundary limits temporary stack growth. Both
boards retain their previous table capacities and protocol/storage layouts.
Full-capacity host tests cover atomic conditional-table replacement, advanced
layer/lock predicates, brightness, output controls, and 20-second Magic linger.

The candidate carries all 36 previous RMK assembly pins plus
`fix/cooperative-storage`. Its generated tree is
`6af1700c3fb2dd2b32b02cd7bf6a4396c78a58eb`, reproduced by a locked build.
The assembled candidate is published to `candidate/magic-runtime-crash`;
the assembly and product sources use `fix/magic-runtime-crash`. No default
published installation branch was moved.

The ordinary build-hash mismatch policy remains unchanged: a different
firmware build can reinitialize persisted configuration. Storage-format
compatibility does not imply that an upgrade automatically preserves settings.
Any later hardware qualification must account for the saved runtime backup.
