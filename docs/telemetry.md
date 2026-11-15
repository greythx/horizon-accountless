# Not in this fork

This fork has no telemetry at all: no collection code, no consent screen, no switches, no install IDs
and nothing sent anywhere. The launcher never contacts a telemetry server. The telemetry crate
(`crates/horizon-telemetry`) remains in the tree because the game has its own opt-in paths, but
nothing in the launcher uses it.

The upstream project documents its telemetry for its own builds; those docs live in the upstream
repository, at `horizon-rust/horizon` (`docs/telemetry.md`), and describe that project's choices, not
this fork's.
