# Not in this fork

The launcher in this fork has no telemetry at all: no collection code, no consent screen, no switches, no
install IDs and nothing sent anywhere. The launcher never contacts a telemetry server, and it starts the
game with the game's own ping and telemetry turned off (`HORIZON_NO_HEARTBEAT=1`, `HORIZON_TELEMETRY_URL=off`),
so a game started from this fork sends nothing either. Running the game binary directly is outside that
guarantee: the game keeps its opt-in paths (this fork does not strip them), and without those variables
set the ping still exists. The telemetry crate (`crates/horizon-telemetry`) remains in the tree for those
paths; nothing in the launcher uses it.

The upstream project documents its telemetry for its own builds; those docs live in the upstream
repository, at `horizon-rust/horizon` (`docs/telemetry.md`), and describe that project's choices, not
this fork's.
