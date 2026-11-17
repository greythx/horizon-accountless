# Not in this fork

This fork has no Microsoft sign-in, anywhere: no account system, no Azure app to configure, no
`HORIZON_MSA_CLIENT_ID`, no accounts.json and no Minecraft session hand-off. The launcher starts the
game offline, prepared from the player's own Minecraft install. See [FORK.md](../FORK.md).

The upstream project documents its sign-in for its own builds; those docs live in the upstream
repository, at `horizon-rust/horizon` (`docs/microsoft-account.md`), and describe that project's
choices, not this fork's. The offline placeholder identity a Vanilla or Modded instance launches
with can be refused by online-mode servers, exactly as the official client's offline mode can be.
