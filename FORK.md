# FORK.md — this is an unofficial fork of Horizon

**This is an unofficial fork of [horizon-rust/horizon](https://github.com/horizon-rust/horizon), MIT licensed.**
It is not affiliated with, endorsed by, approved by or connected to the Horizon project or its
maintainers in any way, and it does not speak for them. The MIT license applies to everything here
(read [LICENSE](LICENSE)) — keep it (and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)) with all
copies.

The upstream project's site is [playhorizon.dev](https://playhorizon.dev); the upstream repository is
`horizon-rust/horizon`. Nothing on this page changes what Horizon the project decides; it documents
what *this fork* does differently.

## What this fork deliberately changes

1. **Microsoft account NOT required to launch.** The launcher starts the game for everyone. Without
   a signed-in account (or one whose sign-in can't be renewed), the game starts offline. Online-mode
   (authenticated) servers still need an account that owns Minecraft: Java Edition, as they do
   upstream.
2. **The Mojang download stays owner-only.** Downloading the game files from Mojang, and playing on
   online-mode authenticated servers, remain limited to an account the Minecraft services say owns
   Minecraft: Java Edition. This is Mojang's requirement, not Horizon's policy, and this fork does
   not relax it — nor the account requirement of Mojang's servers.
3. **This build has no `HORIZON_MSA_CLIENT_ID`.** Microsoft sign-in is therefore not offered as
   something that works: where sign-in appears, it says this build has no sign-in configured.
   Sign-in buttons, where kept, are optional and only for online play, never to start the game.
   A fork that wants working Microsoft sign-in must register its own Azure app with Mojang
   ([docs/microsoft-account.md](docs/microsoft-account.md)) and build with `HORIZON_MSA_CLIENT_ID`
   set at build time (a run-time override of the same variable works too).
4. **Play with Friends, LAN and dedicated servers are unaffected.** The game's own behaviour is
   unchanged; the fork changes only the launcher's sign-in requirement.
5. **Upstream `main` is synced into this fork by merge.** [GOVERNANCE.md](GOVERNANCE.md) documents
   how Horizon the project is run; it is not binding on this fork, and the fork does not edit it.
6. **No pushes to upstream.** This fork never pushes to `horizon-rust/horizon`. Tag releases are
   not cut here (no Actions); any release published by this fork is a community build, not an
   official Horizon release.

Everything else — the game, the mods system, the launcher's features — is built from the upstream
source with the change above. If something looks different from upstream, that difference is a bug or
a merge from upstream, never a silent relicensing.

## Building

See [launcher/README.md](launcher/README.md) and [docs/native-client.md](docs/native-client.md). Without
a `HORIZON_MSA_CLIENT_ID`, the built launcher starts the game offline and says so where it would
offer Microsoft sign-in.
