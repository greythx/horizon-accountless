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

1. **No Microsoft account system at all.** There is no sign-in, no accounts.json, no Minecraft
   session hand-off, no account settings or top-bar chip, no MCP or CLI account tools. The fork has
   no `HORIZON_MSA_CLIENT_ID` and never will, so nothing about Microsoft sign-in exists here:
   - the game is prepared from the player's own Minecraft install (any launcher's: the official
     one, Prism, MultiMC, the Modrinth App, ATLauncher, GDLauncher), as before — or, once the
     player accepts the one-time disclaimer ("I own Minecraft: Java Edition"), from a download
     from Mojang's servers into the launcher's own `minecraft/` folder: any version on Mojang's
     list for which a Java instance is made, the pinned 26.3 for the game's preparation, each
     file checked against its size and sha1. The disclaimer is the ownership check; nothing
     verifies it (nothing can without sign-in), as it is for every launcher without accounts.
     `Settings > Storage > Minecraft's files > Forget` takes the consent back; the launcher
     asks again before the next download. If no install holds the files and the disclaimer was
     not accepted, the launcher says so; the Java instances and the preparation itself cannot
     proceed on their own. Files downloaded this way are never redistributed or shipped in the
     repository; they are checked against Mojang's listed hashes on the way in, exactly as
     before. This is the same exception upstream makes (`prepare --download`, for a signed-in
     account the Minecraft services said owns the game): this fork makes it accountless, behind
     the disclaimer instead;
   - Java Edition (Minecraft proper, as instances of "Vanilla" or "Modded") starts from that same
     install, or Horizon's own download of the version, with an offline placeholder name;
     online-mode servers refuse it, as they would any offline launch;
   - the game itself downloads upstream Horizon's unmodified release build from
     [github.com/horizon-rust/horizon](https://github.com/horizon-rust/horizon) releases, checked
     against the release's signed checksums; that is this fork's only source of the game binary.
     When GitHub's API rate-limits (or is unreachable), the release list and other API reads of
     Horizon's repository may be served by Horizon's read-only GitHub mirror
     (`mcp.playhorizon.dev/github/public`, replaced by `HORIZON_GITHUB_MIRROR`); the game binary
     itself downloads from GitHub's release asset URLs. Building from source works too.
2. **No telemetry or data collection.** No "I'm playing" ping, no consent screen, no data-collection
   settings or CLI, no `telemetry` code paths in the launcher, no phone-home of any kind. The fork
   keeps no counters of players and shows none ("N playing now", "M on the official SMP").
3. **No ads or promotion.** No partner ("Sculk Hosting") banner or card anywhere, no Discord,
   Sponsor or Website buttons in the sidebar/topbar/settings/crash page, no "What's new"/patch-log
   feed of upstream's release notes on Home, and no Horizon official-server entry seeded into the
   game's server list. Horizon's registry (the mod browser's "Horizon" source) remains listed:
   it is how upstream content mods are distributed.
4. **No launcher self-update. Ever.** The launcher never replaces itself with upstream's build, and
   never redirects elsewhere for one: the update machinery, the Updates settings page, the "Check
   now" flow and the `update`/`account`/`data`/`links` CLI commands are gone. New Horizon game
   versions install by hand (`horizon-launcher install`, or the Versions page); nothing about the
   launcher moves by itself.
5. **Wardrobe, Shop and the friends invite bar are gone.** Horizon's cosmetics service is not
   contacted; there is no cosmetics tab, no cosmetics settings, no `horizon-friends` invite code on
   Home, and no `horizon-friends` dependency.
6. **Play with Friends' launcher side is gone** (the friend-code invite row on Home and in the Play
   menu). LAN and dedicated servers are unaffected, as is the Tailscale feature for sharing your
   own server over Tailscale (`Servers`, "friends over the internet"), which keeps working with a
   Tailscale access token the player pastes in.
7. **Upstream `main` is synced into this fork by merge.** [GOVERNANCE.md](GOVERNANCE.md) documents
   how Horizon the project is run; it is not binding on this fork, and the fork does not edit it.
8. **No pushes to upstream.** This fork never pushes to `horizon-rust/horizon`. Upstream releases
   are not republished from here; any release published by this fork is a community build, not an
   official Horizon release, and none is planned yet (releases may come later).
9. **Hosted servers always run offline-mode.** Servers created by the launcher (Servers page and
   `servers create`) always start with `--offline`: with no accounts anywhere in this fork, there is
   no Minecraft account verification for them to do. Hosted servers cannot run online mode in this
   fork (no sign-in anywhere), and the launcher does not expose the online-mode toggle.
10. **No parity audits.** Horizon's vanilla-parity harnesses need a self-hosted runner holding
   the Minecraft 26.3 jars (Horizon never downloads them); this fork has no such runner, so the
   job would queue forever with no merge effect — parity is not a required check, and this fork
   never changes a harness or an oracle. `parity-scope` in `.github/workflows/ci.yml` short-
   circuits to `run=false` on this repository, and `nightly audit`/`parity-main` stay disabled
   here. If this fork ever registers a `horizon-game` runner, delete that short-circuit.

Everything else — the game, the mods system, the launcher's features — is built from the upstream
source with the changes above. If something looks different from upstream, that difference is a bug or
a merge from upstream, never a silent relicensing.

## Building

See [launcher/README.md](launcher/README.md) and [docs/native-client.md](docs/native-client.md). There
is nothing to configure: the built launcher starts the game offline, prepared from the player's own
Minecraft install or (once the disclaimer is accepted) from a download from Mojang's servers.
