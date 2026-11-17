# Native mods

Native mod docs live in the modding book: [native mods](modding/src/how-to/native-mods.md), [trait mods, Parts and attachments](modding/src/concepts/parts.md), [injections](modding/src/concepts/injections.md) and the [hook notes](https://github.com/horizon-rust/horizon/tree/main/crates/horizon-mod-vocab/src/hooks).

## Living death version 2

`Hooks::living_death` receives a mutable `LivingDeathEvent` on the server only.
Its fields are `entity`, `kind`, `source: &HurtSource`,
`weapon: Option<&ItemStack>` and `cancel`. This is a clean contract break:
the old top-level `damage_type` and `attacker` fields and callback signature
are removed, with no compatibility callback.

The source is the accepted killing blow seen by `living_incoming_damage`:
damage type, real causing entity ID (including players), direct entity ID
(including arrows), and original optional source position. The optional weapon
is that blow's complete stack, including damage and component overrides, not
a later read of the attacker's hand.
`/damage` keeps the selected registered trident type rather than relabeling it
as thrown damage. Its explicit `by` (direct) and `from` (causing) entities remain
independent, including a locally bound causing player. A damage type alone does
not manufacture an arrow-family direct entity.

Cancellation suppresses death loot, experience and death messages without
writing health. A listener that sets health to 5 leaves the entity alive at 5;
an unhealed body at health <= 0 is removed after 20 ticks without redispatch
or loot. Cancellation alone does not heal or force health to 1.
The cancelled continuation belongs to the living body: full-body teleports preserve
its original removal deadline, rather than restarting it or retrying death.
A later positive `set_health` revives a cancelled dying body, clears that deadline
and permits a subsequent genuine death; transaction abort restores the prior
health and continuation together. Saved mobs retain the optional Horizon-owned
`horizon:death_canceled` bit alongside their real `DeathTime`; ordinary loading
resumes a saved dying body without running its death callback or loot again.

### Portable contract regression

`crates/horizon-mod-wasm/tests/fixture/src/lending.rs` supplies the same
`Lender::living_death` listener and `check_death` assertions to the native
`collect_positions_copy_in_and_out_natively_as_in_the_real_guest` test and
the compiled-guest `real_member` test. They dispatch a player/arrow source
with distinct causing/direct IDs and a nonintegral position, and check the
bow's identity, damage and complete custom-component patch, both optional
weapon branches, cancellation, and an unchanged borrowed weapon. Their scope is
the portable record boundary, not Game death timing or loot behavior.

Build the matching real guest and standard module, then run both selectors:

```sh
SCRATCH=$(mktemp -d)
export HORIZON_HOME="$SCRATCH/game" HORIZON_LAUNCHER_HOME="$SCRATCH/launcher"
export XDG_DATA_HOME="$SCRATCH/data" XDG_CONFIG_HOME="$SCRATCH/config"
export HORIZON_NO_AUDIO=1
export HORIZON_LAUNCHER_TRASH="$SCRATCH/trash"
export HORIZON_LAUNCHER_CLIPBOARD="$SCRATCH/clipboard"
export HORIZON_LAUNCHER_DESKTOP="$SCRATCH/desktop"
```

Install and prepare the base release in that scratch launcher home first, then test your own licensed bundle:

1. `horizon-launcher native-build bundle/ --server-output $ENGINE --version TAG --trust-native-code` (scratch homes,
   `HORIZON_NO_AUDIO=1`).
2. `horizon-server --dir SERVER config set engine.path=$ENGINE/horizon-engine`; `horizon-server --dir SERVER run`.
3. Join with the same mod's client half where the hook needs one; test lifecycle, tick, command and net hooks.
4. Headless compatibility: the mod's server code must not require an audio device; run the server audio-free (`HORIZON_NO_AUDIO=1`).

## Left to do

- A list of every native build, to delete the ones no instance needs.
- A prebuilt horizon-mixins from its CI instead of building it on the player's computer.

## Named hook catalog (draft)

`native-hooks.json` maps stable hook names and contract versions to the current
source targets. A contract includes the signature, call timing and behavior.
Keep an existing contract or add a new version when it changes incompatibly.
The catalog does not cover helper methods or arbitrary private game code.
Injections name them with `hook` and `hook_version` ("Named hook points" above); raw
targets and the `hooks` metadata continue to work as before.

### Knockback hook versions

`living_knockback_v2` keeps `living_knockback` version 1 unchanged, as `menu_button_v2` does for
its older hook: v1's function signature is a target mixin mods attach to, so it cannot gain
arguments. Both already receive the raw strength before knockback resistance (v1 did before
v2 existed). V2 adds three things: the attacker's network entity id (falling back to a known
last hurt-by mob, or 0 when unknown), what the attacker holds (`attacker_main` and
`attacker_off`: a player's selected and off-hand stacks, a mob's main hand; none when unknown,
recorded only while a native mod is loaded), and armor-stand sweeps. V2 runs after v1 on
their shared paths; cancellation from either prevents motion, so a trait mod that listens to
both sees each hit twice and should pick one. Mob melee, player hits, sweeps and spears use the
shared knockback path; direct velocity pushes (explosions, Punch arrows, hoglin throws) are not
`LivingEntity.knockback` and do not fire it. The fragment assigns the new API number when
stamped; the existing hook stays version 1.

### Server settings (draft)

`native_online_mode()` is the dedicated server's `online-mode`: true when its players sign in with their accounts. `native_server_port()` is the port it listens on, `server-port` of server.properties or the `--port` it started with. `native_proxy_forwarding()` is `"velocity"` when the server takes its players from a Velocity proxy (`forwarding.secret` beside server.properties, offline mode only) and `"none"` otherwise; the secret itself is never returned. All three are read from the server's live network config, so they match what it does. They are None off a dedicated server (a single player's or LAN world's) and before it listens. The `enforce-secure-profile` and `hardcore` settings are not exposed here.

### Reading server blocks (draft)

A native server tick hook can call `native_server_block` with its context, a dimension id and a block position. It returns the block name and state properties from a loaded chunk. It does not load a chunk or open a dimension. Reads outside that hook, from a stale context, from unavailable dimensions or outside the world bounds return an error.

### Neighbour observations (draft)

`native_neighbor_changed` observes each authoritative server `neighborChanged` before the block handles it. It supplies the dimension, position, current state ID, changed block ID and moved flag. Resolve IDs with the level's registries during the hook; the hook does not load chunks or change the update queue.

### Server block changes (draft)

`native_block_changed` observes a committed authoritative server `Level.setBlock` change before block side effects and neighbour updates. It supplies the dimension, position and old and new state IDs. It does not observe chunk loading or direct storage writes that bypass `Level.setBlock`.

### Server commands and players

For mods that run where the world is simulated: a dedicated server, a LAN host, single player's server thread
(`crates/horizon-client/src/game/native_server.rs`). The SMP mod of the official server is built on them
([smp.md](smp.md)).

- **`native_register_commands(commands)`**: push a `NativeCommand` per usage, asked once when a world opens to
  players. A usage is words, then arguments `<name:kind>`, optional ones last in brackets:
  `NativeCommand::new("pay <player:player> <amount:integer>")`, `NativeCommand::op("eco set <player:player>
  <amount:integer>", 3)` for operators of level 3. Kinds: `word`, `text` (the rest of the line, last), `integer`,
  `number`, `boolean`, `player`, `item`, `block_pos`. The usages of one command merge into one tree, which every
  player receives after vanilla's (filtered by their level, as vanilla's is), so their chat box completes and colours
  it. A name vanilla has, or a usage that doesn't parse, is left out with a warning in the log.
- **`native_refresh_commands()`** (`host.refresh_commands()` for trait mods, API 447): a helper for
  a mod whose commands change while the server runs (a plugin host that reloads a plugin, a mod turned on at run time).
  At the start of the next server tick every mod's `native_register_commands` is asked again, the commands players may
  run (`native_perform_command`, with their levels) are rebuilt, and every player on the server is sent their tree
  again, after `native_player_commands_sent`, as vanilla sends `Commands.sendCommands` to each player after `/reload`.
  Call it after you change what you register; nothing happens where no world is open to players, and a server whose
  mods never call it is unchanged.
- **`native_armor_stand_manipulate(player, name, stand_id, slot, cancel)`**: a player takes an item from an armor stand or
  puts one on it, heard after the stand's disabled-slot checks and before anything moves. `slot` is `mainhand`,
  `offhand`, `feet`, `legs`, `chest` or `head`. Set `cancel` and nothing changes (the click is spent). The player's
  state is swapped in; `player` is their UUID. Not told of a click that changes nothing; breaking or blasting a stand and
  a dispenser armouring one are other ways its equipment moves and are not covered.
- **`native_vehicle_moved(player, kind, from, to)`**: a boat or minecart that carries a server player (`player`, its UUID)
  moved since the last tick, heard once a tick after the move with its positions before and after. The rider's state is
  swapped in, so `native_acting_player`, `native_tell` and the item helpers reach them. A vehicle a player rode and left is
  still told of, with no `player` and nobody swapped in, until it has stood still for ten seconds. Mounts (horses...) and
  vehicles nobody ever rode are not covered. It only hears and costs one position compare a tick for each watched vehicle.
- **`native_container_opened(player, name, kind, pos, cancel)`**: a player is about to open one of the game's own menus
  (a chest, furnace, crafting table, villager, anvil, lectern, a horse's inventory...). `kind` is the menu type
  (`minecraft:generic_9x3`, `minecraft:merchant`...), `pos` the block it belongs to, none for a villager, a mount or a
  vehicle's chest. Heard after the checks that refuse the menu (a locked chest), before the open menu closes. Set
  `cancel` and nothing opens. The player's state is swapped in, so `native_acting_player`, `native_tell` and the item
  helpers reach them. Menus a mod opens (`native_open_menu`) are not heard.
- **`native_creative_slot(player, name, slot, item, count, cancel)`**: a creative-mode player sets a slot from the creative
  screen (`slot` is the inventory menu's slot; below 0 is a stack thrown out of the screen), with the `item` (`minecraft:air`
  for an emptied slot) and `count`. Heard after vanilla's checks, before the slot changes. Set `cancel` and the packet is
  ignored. The player's state is swapped in. Stacks a creative player gets other ways (pick block, `/give`) are not heard.
- **`native_player_game_mode_changed(player, name, from, to, cancel)`**: a server player's game mode is about to change
  (0 survival, 1 creative, 2 adventure, 3 spectator, as `NativeServerPlayer.game_mode`): the `/gamemode` command, the
  F3+F4 switcher's packet and a hardcore world's respawn into spectator, once per change to a different mode. The mode a
  player joins with (`defaultgamemode`) is not heard. Set `cancel` and the player keeps their mode. Inside the command and
  the switcher the player's state is swapped in; for the hardcore respawn it is not (use the UUID with the helpers that
  take one).
- **`native_player_changed_dimension(player, name, from, to)`**: a player changed dimension, once per move
  (`from` and `to` are ids such as `minecraft:overworld` and `minecraft:the_nether`), heard when the move has ended:
  the portal's exit is found and the player stands there, a command's teleport has landed, or a respawn in another
  level than the death's is placed for good. `to` is the level in place and the player's state is swapped in (as in
  `native_perform_command`), so `native_acting_player` and `native_tell` reach them. Heard for Nether and End portals
  (also riding, and leaving the End as its winner), `/execute in ... run tp` and any command teleporting into another
  level, and the death respawn. Not heard: a move within a level (`/tp`, an End gateway), a player who leaves before
  the move ends, a respawn in the dimension of the death, and anything outside a dedicated server.
- **`native_player_toggle_flight(player, name, flying, cancel)`**: a player's client started (`flying`) or stopped
  flying, once per change, for a player who may fly (creative or spectator), on a server. The player's state is swapped
  in, so `native_tell` and the other helpers reach them. Set `cancel` to refuse it: the player keeps the state they
  had and gets their abilities back, so the client lands or flies again. Only what a client asks for is heard: not the
  flight a game-mode change starts or ends, and not a LAN host's own player.
- **`native_player_item_held(player, name, from_slot, to_slot, cancel)`**: a server player selected a different
  hotbar slot, after the slot range check and before it changes. The player's state is not swapped in, so helpers
  that look them up through `self.lan` may find nothing while a guest packet is being handled. Set `cancel` to
  keep the old slot; the client is sent that slot again.
- **`native_furnace_burn(pos, fuel)`**: a furnace consumed `fuel` and started burning at `pos`.
- **`native_furnace_smelt(pos, item, result)`**: a furnace finished smelting `item` into `result` at `pos`.
- **`native_player_exp_change(player, name, amount)`**: a server player's experience points from an orb pickup are
  about to be added, after mending has taken its share. Change `amount` to change the points added; negative values
  become zero. The player's state is not swapped in, so helpers that look them up through `self.lan` may find nothing
  while a guest packet is being handled. Command experience changes use another path and are not heard.
- **`native_player_item_cooldown(player, name, item, ticks, cancel)`**: an item cooldown is about to start for a
  server player, including a used item's cooldown and a shield disabled by a hit. `item` is the item id and `ticks`
  its duration. Set `cancel` to skip the cooldown; shield disable still stops the use and plays its configured sound.
  The player's state is not swapped in, so helpers that look them up through `self.lan` may find nothing while a guest
  packet is being handled.
- **`native_player_recipe_discovered(player, name, recipe, cancel)`**: a recipe id is about to become known to a
  server player through an advancement reward, a knowledge book or `/recipe give`. Set `cancel` to keep it locked;
  the client is sent the matching removal. The player's state is not swapped in, so helpers that look them up through
  `self.lan` may find nothing while a guest packet is being handled.
- **`native_player_spawn_point_set(player, name, dimension, pos, cause, cancel)`**: a server player's respawn point
  is about to change. `cause` is `bed`, `respawn_anchor`, `command` or `death_reset`; `pos` is its block position.
  Set `cancel` to keep the old point. A canceled `death_reset` keeps the old point even if its bed or anchor is gone.
  The player's state is not swapped in, so helpers that look them up through `self.lan` may find nothing while a guest
  packet is being handled.
- **`native_player_portal(player, name, from_dimension, to_dimension, cause, cancel)`**: a server player is about
  to travel through a Nether portal, End portal or End gateway; `cause` is `nether_portal`, `end_portal` or
  `end_gateway`. Set `cancel` to leave the player where they are. The player's state is not swapped in, so helpers that
  look them up through `self.lan` may find nothing while a guest tick is handled.
- **`native_block_damage_abort(player, name, pos)`**: a server player stopped digging `pos` before it broke. It is
  heard after the server clears the dig and sends the block state back to the client. The player's state is not
  swapped in, so helpers that look them up through `self.lan` may find nothing while a guest packet is handled.
- **`native_player_pick_item(player, name, from_entity, pos, entity_id, cancel)`**: a server player's middle-click
  pick resolved a block or entity's pick stack and is about to put it in the hotbar. For a block, `pos` is its
  position and `entity_id` is zero; for an entity, `from_entity` is true, `pos` is `[0, 0, 0]` and `entity_id`
  is its network id. The player's state is swapped in, though helpers that look them up through `self.lan` may
  find nothing while a guest packet is handled. Set `cancel` to leave the inventory and client unchanged. A
  pick with no result is not heard.
- **`native_sign_changed(player, name, dimension, pos, front, lines, cancel)`**: a player finished a sign they were
  editing, heard once every check has passed (editing that sign, in reach, not waxed, the chat filter) and right before
  the text goes on it. `lines` holds the four lines as written, formatting codes still in; rewrite them and that is
  what the sign gets (the chat filter does not look at it again), or set `cancel` and it keeps its old text. The
  player's state is swapped in, so `native_tell` reaches them. Not told: the host's own editing on a LAN or
  single-player world, and changes that are no text edit (dye, glow ink, wax, `/data`, `/setblock`).
- **`native_player_client_loaded_world(player, name)`**: the server heard that a guest's client finished loading
  the level around them after joining, respawning or changing dimension. The player's state is not swapped in, so
  helpers that look them up through `self.lan` may find nothing while the guest packet is handled.
- **`native_player_stop_using_item(player, name)`**: a server player released the use-item key before the item
  finished being used. The player's state is not swapped in, so helpers that look them up through `self.lan` may
  find nothing during packet handling.
- **`native_player_recipe_book_settings_change(player, name, book, open, filtering)`**: a server player's recipe
  book open or filter setting changed; `book` is the book type's protocol id. The player's state is not swapped in.
- **`native_player_advancement_criterion_grant(player, name, advancement, criterion)`**: a server player earned
  one advancement criterion, after its progress was recorded.
- **`native_player_command(player, name, command, canceled, replies)`**: any command a player typed (without the
  slash), vanilla's too and commands the player may not run, heard before anything looks at it, and each line of a
  quiet command batch. Set `canceled` and nothing runs; push lines to `replies` and the player gets them as system chat
  (a quiet batch drops them). The player's state is not swapped in, and `native_tell` and the other helpers that take a
  UUID reach nobody inside this hook: answer through `replies`. Commands may carry secrets (`/msg`, a login plugin's
  password), so don't log them carelessly.
- **`native_perform_console_command(command, ran)`**: a registered command typed on the dedicated server's console
  (without its slash), run as the server at level 4: no player, so `native_acting_player()` is None and the player-only
  helpers give nothing. The mod parses the line and sets `ran`; `native_feedback` answers on the console. A name no mod
  registered goes to vanilla's dispatcher as before. The single player owner's console (`horizon-ctl command`, the MCP
  command tool) is a player's: it reaches `native_perform_command` with the owner, at level 4.
- **`native_player_respawned(player, name)`**: a dead player came back, once per respawn, on a server: after
  `PlayerList.respawn` reset their health, food and experience and placed them at their bed, anchor or the world spawn
  (NeoForge's `PlayerRespawnEvent`). The player's state is swapped in for the call, as for their other actions, so
  `native_player`, `native_give_items` and the other helpers reach them. A respawn at an anchor in another dimension, or
  at the world spawn's free column, may still move them on a later tick, once that level has loaded round them; the
  End-exit respawn (after the credits) does not fire it.
- **`native_player_login(player, name, cancel, reason)`**: a player the server has accepted (their name, the whitelist,
  bans and the player limit are checked) and who holds a place, heard before their saved state is loaded and before they
  are placed in a level (NeoForge's `PlayerNegotiationEvent`). Set `cancel` and they are disconnected with `reason` (a
  plain "Disconnected" when it stays `None`); no join message and no `native_player_logged_in` follow. It runs after the
  client's configuration, so the client sees the refusal as an in-game disconnect screen. The player is not placed yet,
  so no player's state is swapped in and `native_tell` reaches nobody: the refusal text is the answer. Players the
  server refuses itself (name, whitelist, bans, a full server, the proxy's forwarding) are not heard.
- **`native_player_teleported(player, name, from, to, dimension)`**: a server player a command (`/tp`, `/teleport`,
  also what a mod's `run_command` reaches) moved, inside their level or into another; `from` and `to` are their feet
  before and after and `dimension` the level they are in now. Heard after the move has reached the player's state and
  their client (into another level: when that level is entered, before it has loaded around them), so it cannot stop it
  (NeoForge's `EntityTeleportEvent`). The player's state is swapped in, so `native_tell` and the acting-player helpers
  work. Ender pearls and chorus fruit have their own hooks below; not heard yet: a spectator's teleport menu; portals
  and respawns are not teleports here.
- **`native_ender_pearl_teleport(player, name, pearl, from, target, damage, cancel)`**: a server player's ender pearl
  landed and is about to teleport them (NeoForge's `EntityTeleportEvent.EnderPearl`, `ThrownEnderpearl.onHit`).
  `pearl` is the pearl's entity id, `from` their feet, `target` where the pearl started its last tick and `damage` the
  fall damage they then take (5). A mod may move `target` or change `damage` (0 or less: none). Canceling keeps the
  player where they are: no endermite roll, no teleport, no damage, no sound; the pearl is used up either way. Mobs'
  pearls are not heard. Bundles need `"hooks": 312`.
- **`native_chorus_fruit_teleport(player, name, from, target, cancel)`**: a server player whose `teleport_randomly`
  consume effect (chorus fruit) is about to try a spot (NeoForge's `EntityTeleportEvent.ChorusFruit`,
  `TeleportRandomlyConsumeEffect.apply`): once per attempt, up to 16, after they stopped riding. A mod may move
  `target` (its y is kept inside the level's height); the game still looks down from it for ground to land on, inside
  the border. Canceling ends the effect: no
  teleport and no more attempts. Foxes eating chorus fruit are not heard. Bundles need
  `"hooks": 311`.
- **`native_player_clone(player, name, was_death)`**: a player's data was copied to their new player (NeoForge's
  `PlayerEvent.Clone`, from `ServerPlayer.restoreFrom`): after a death respawn (`was_death` true) and when a player
  leaves the End after the credits (`was_death` false, vanilla's `keepAllPlayerData`). It is heard after the copy and
  before the new player is placed, once each, on a server. The uuid stays the same, so `player` names the old and the
  new player alike; no player's state is swapped in, so the helpers that look a player up may find nothing. It cannot
  stop anything.
- **`native_player_load_from_file(player, name, directory)`**: a joining player's saved state was read and applied
  (NeoForge's `PlayerEvent.LoadFromFile`, `PlayerList.loadPlayerData`), before they are placed in a level: once for each
  join on a server, for an integrated server's owner (loaded from the file `level.dat` names, else their own) as for
  anyone else. `directory` is the world's `players/data` folder, created if missing; read your own file there, named `<uuid>.<suffix>`. It
  is heard whether or not the player was in the world before (a new player has no file). No player's state is swapped
  in. It cannot change or stop anything.
- **`native_player_save_to_file(player, name, directory)`**: a placed player's data was saved (NeoForge's
  `PlayerEvent.SaveToFile`, `PlayerList.save`): when they leave or the world closes, when the dedicated server saves
  everyone, and for an integrated server's owner when the world saves their state into `level.dat`. `directory` is the
  world's `players/data` folder, created if missing; write your own file there, named `<uuid>.<suffix>`, and never the player's `.dat`.
  The player may be leaving, so no player's state is swapped in. It cannot change or stop anything.
- **`native_player_commands_sent(player, name, names)`**: the command tree a player is sent, when they join and when
  their level changes (`/op`, `/deop`, the LAN settings). `names` are the root commands their client would get, vanilla's
  they may use and the mods'; remove a name and that client's tree lacks it, so its chat box neither completes nor
  colours it. This hides the command from the client only, a typed one still runs (stop it with
  `native_player_command`). For a player already on, their state is swapped in and `native_tell` works; a joining
  player has no world yet, so nothing can be told them. The tree the hub re-sends by itself (after a respawn or a change
  of level) reuses the last answer.
- **`native_player_sprint(player, name, sprinting)`**: a server player's client started or stopped sprinting (the sprint
  key, a double tap forward, letting go), once for each change, after the server's player took it. It only hears. It
  reports what the client asks, not what the server does: a sprint that ends by itself (a hit's knockback, hunger, a
  wall) is not heard. The player's state is swapped in, so `native_tell` and the acting-player helpers work.
- **`native_resource_pack_status(player, name, status)`**: a server player's client answered a resource pack while
  playing (`ServerboundResourcePackPacket`): `status` is `Action`'s number (0 loaded, 1 declined, 2 failed download,
  3 accepted, 4 downloaded, 5 invalid URL, 6 failed reload, 7 discarded). It only hears. The pack a dedicated server
  offers during the configuration is answered before the player has joined and is not heard. The player's state is
  swapped in, so `native_tell` works.
- **`native_projectile_hit(owner, kind, hit_entity, pos, cancel)`**: a projectile a server player launched (arrow,
  snowball, egg, thrown trident, ender pearl, ...) hit an entity (`hit_entity`) or a block at `pos`, heard before any
  of its effects (NeoForge's `ProjectileImpactEvent`). Set `cancel` and it does nothing: it vanishes, a thrown trident
  flies on through. The shooter's state is swapped in, so `native_tell` answers them. Mobs' and the host's
  projectiles are not heard, nor are fishing bobbers and firework rockets.
- **`native_tnt_primed(player, pos, cancel)`**: TNT at `pos` is about to be lit by a flint and steel or fire charge
  (`player` is whoever clicked, whose state is swapped in), fire, a redstone signal, an arrow or a dispenser (`player` is
  None). Set `cancel` and it is not primed: the block stays, or is put back if fire or a signal had already taken it
  out. TNT set off by a blast, a TNT minecart and a sulfur cube are not asked; `native_explosion_detonate` can spare
  the blocks of a blast.
- **`native_hanging_place(player, name, kind, pos, cancel)`**: a server player places an item frame, a glow item
  frame, a painting or a leash knot (tying a lead to a fence that has none; `pos` is the fence), once it fits and
  before it is placed or the item used. Set `cancel` and nothing happens; the item stays in the hand. The player's
  state is swapped in, so `native_tell` answers them. `/summon` and a lead tied to a mob are not heard.
- **`native_hanging_break(player, kind, pos, cancel)`**: an item frame, a glow item frame or a painting (`kind`)
  hanging in `pos` is about to break: a server player's hit (`player`, whose state is swapped in), a blast, or the
  wall it hangs on gone (`player` None for both), before anything drops or sounds. Set `cancel` and it stays as it
  was. A hit that only takes a frame's item out, the game's own player and `/kill` are not asked.
- **`native_bucket_empty(player, name, pos, fluid, cancel)`**: a server player empties a bucket (`fluid` is
  `minecraft:water`, `minecraft:lava` or `minecraft:powder_snow`; fish and axolotl buckets pour water) into `pos`,
  right before the fluid is placed. Set `cancel` and nothing is poured. The player's state is swapped in, so
  `native_tell` answers them. `native_fill_bucket` hears the same click without the player and the fluid; dispensers
  and cauldrons are not heard. A fluid or mob bucket is also heard at the block beside the clicked face whenever that
  block is not `pos`, since a block that refuses the fluid passes it there; the fluid may still go into `pos`.
- **`native_player_bed_enter(player, name, pos, cancel)`**: a server player passed the sleep checks and is about to
  enter the bed at its head position. Set `cancel` to keep them out; helpers that look them up through `self.lan` may
  find nothing while a guest packet is handled.
- **`native_player_bed_enter(player, name, pos, cancel)`**: a server player passed the sleep checks and is about to
  enter the bed at its head position. Set `cancel` to keep them out; helpers that look them up through `self.lan` may
  find nothing while a guest packet is handled.
- **`native_player_bed_leave(player, name, pos)`**: A server player woke or stood up from a bed, after their sleeping state changed. The head position is `pos`; helpers looking up the guest through `self.lan` may find nothing during guest packet handling.
- **`native_player_edit_book(player, name, pages, signing, cancel)`**: A server player book-and-quill edit or signing packet is about to be applied. `pages` is its page count and `signing` says it has a title. Set `cancel` to drop the packet.
- **`native_player_shear_entity(player, name, entity_id, off_hand, cancel)`**: A server player is about to shear a sheep, mooshroom, snow golem or bogged. The hook gives the mob entity id and whether the shears are in the off hand. Set `cancel` to leave the mob and shears unchanged.
- **`native_player_advancement_done(player, name, advancement)`**: A server player completed an advancement after its progress was recorded. `advancement` is its identifier, such as `minecraft:story/mine_stone`; helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_advancement_progress(player, name, advancement, criterion, granted)`**: A criterion of a server player's advancement was granted or revoked, after its progress changed and before any completion rewards, except the `minecraft:tick` criteria granted on a network player's first tick or a reload, which are heard after that tick's rewards and packets. `advancement` is its identifier, such as `minecraft:story/mine_stone`, and `granted` is false for a revoke. A grant or revoke that changes nothing is not heard; helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_stat_award(player, name, stat_type, stat, value, cancel)`**: A statistic is about to be awarded to a server player, before its count and scoreboard criteria change. `stat_type` and `stat` are ids such as `minecraft:mined` and `minecraft:stone`, or `minecraft:custom` and `minecraft:walk_one_cm`, and `value` is the amount. Set `cancel` to award nothing. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_riptide(player, name)`**: a server player launched with a riptide trident, after its velocity and spin are applied. The player's state is swapped in, though helpers that look them up through `self.lan` may find nothing during a guest packet.
- **`native_player_fish(player, name, state, cancel)`**: a player's fishing rod, on a server: `state` 0 the cast, 1 a fish
  biting, 2 the reel-in of a bite (the catch), 3 the reel-in with nothing on the line, 4 the reel-in of a bobber hooked
  to an entity or item. Set `cancel` and nothing is thrown (0), the fish gets away and the wait starts over (1), or the
  bobber stays out with no wear on the rod (2 to 4). The bobber ending by itself (too far, rod put away, owner dead) is
  not heard, nor is the host of a LAN world's rod. The player's state is swapped in, so `native_acting_player`,
  `native_tell` and the item helpers reach them.
- **`native_block_dispense_armor(dimension, pos, item, target)`**: a dispenser equipped `item` on the first eligible player, mob or armor stand in front of its block at `pos`. The equip, item consumption, sound and game event have completed; `target` names the kind of entity equipped.
- **`native_block_exp(dimension, pos, block, experience)`**: a block with a vanilla experience drop is about to award its points at `pos`. Change `experience` before the orbs are awarded.
- **`native_block_shear_entity(dimension, pos, entity, kind)`**: a dispenser sheared the entity of `kind` and id `entity` at the block in front of the dispenser at `pos`, after applying the shearing effects.
- **`native_block_receive_game(dimension, pos, block, event, cancel)`**: a sculk sensor, calibrated sensor or shrieker received the vibration `event`. Set `cancel` to suppress its response.
- **`native_brewing_start(pos, ingredient)`**: a brewing stand at `pos` starts its timer for `ingredient`, after it consumes fuel. Server-side.
- **`native_campfire_start(pos, item)`**: `item` was placed on the campfire at `pos` and began cooking. Server-side.
- **`native_block_cook(pos, item, result)`**: a campfire finishes cooking `item` into `result` at `pos`, before the result is dropped. Server-side.
- **`native_compost_item(pos, item)`**: a player adds a compostable item to the composter at `pos`, after its layer is applied. Server-side.
- **`native_crafter_craft(pos, recipe)`**: a crafter completed `recipe` at `pos`, after consuming its ingredients and handling its output. Server-side.
- **`native_player_attack_cooldown_reset(player, name)`**: a server player's punch was handled: the main-hand swing and attack-strength reset have both happened. The player's state is not swapped in, so helpers looking them up through `self.lan` may find nothing during a guest packet.
- **`native_player_bucket_entity(player, name, entity_id, entity)`**: a server player caught a bucketable mob after the mob was removed and the filled bucket was put in the player's hand or inventory. `entity` is the mob species id.
- **`native_player_change_beacon_effect(player, name, pos, primary, secondary)`**: a server player's beacon selection was stored. Effect values are ids, empty when none is selected.
- **`native_player_deep_sleep(player, name)`**: a server player reached 100 ticks asleep in a bed and now counts as deeply asleep for the night-skip check. Helpers looking up the guest through `self.lan` may find nothing during its tick.
- **`native_player_change_main_hand(player, name, left)`**: a server player's client options changed their dominant hand; `left` is true for left-handed. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_egg_throw(player, name)`**: a server player threw an egg after its projectile was launched and its use applied.
- **`native_player_elytra_boost(player, name)`**: a server player used a firework rocket to boost while gliding, after the rocket was launched and consumed.
- **`native_player_flower_pot_manipulate(player, name, pos, plant)`**: a server player put `plant` into the flower pot at `pos`, after its state changed and the item was consumed. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_insert_lectern_book(player, name, pos, book)`**: a server player put `book` on the lectern at `pos`, after the block entity and inventory changed. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_take_lectern_book(player, name, pos, book)`**: a server player removed `book` from the lectern at `pos`, after its block entity changed. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_lectern_page_change(player, name, pos, page)`**: a server player changed the lectern at `pos` to zero-based `page`, after its page and redstone pulse changed. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_item_frame_change(player, name, entity_id, action, item, rotation)`**: a server player changed a frame (`put`, `rotate`, `remove_item` or `break`), after its contents and comparator output changed. `item` is empty when none remains. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_item_mend(player, name, item, repair, experience)`**: an orb actually repaired `repair` durability on `item` for a server player, after the item changed. `experience` is the orb value. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_player_trade(player, name, entity_id)`**: a server player completed a villager or wandering trader trade, after the merchant applied it. `entity_id` is the merchant network id. Helpers looking up the guest through `self.lan` may find nothing during packet handling.
- **`native_trade_with_villager(merchant, kind, offer)`**: NeoForge's `TradeWithVillagerEvent`, heard once per trade a
  server player makes with a villager or a wandering trader (a shift-click that trades several times is heard for each),
  after the trade's statistic and `villager_trade` trigger. `merchant` is the trader's entity id, `kind` its type
  (`minecraft:villager` or `minecraft:wandering_trader`), and `offer` the `MerchantOffer` as it was right after the
  trade: its costs, `result`, `uses` (this trade counted), `max_uses` and `xp`. Listen only, heard once the whole click
  is done (its payments taken; NeoForge fires during each take). The player's state is swapped in, so `native_acting_player` names
  them. Unlike `native_player_trade`, which is heard once per click with no offer.
- **`native_player_shield_disable(player, name, ticks)`**: a blocked melee hit disabled the server player’s shield for `ticks` ticks. It fires just before the cooldown is set; guest lookup helpers may not find the guest during packet handling.
- **`native_player_loom_pattern_select(player, name, pattern)`**: a server player selected loom pattern button `pattern`, after the menu accepted it. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_name_entity(player, name, entity_id, entity_name)`**: a server player used a named name tag on a mob, after it became persistent and was named. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_open_sign(player, name, pos, front)`**: a server player opened the sign editor at `pos` for the front or back side, after the editor state was made. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_pickup_arrow(player, name, entity_id, item)`**: a server player picked up an arrow after it was removed and its pickup was applied. `item` is the pickup item id. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_stonecutter_recipe_select(player, name, recipe)`**: a server player selected stonecutter recipe button `recipe`, after the menu accepted it. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_swap_with_equipment_slot(player, name, slot)`**: a server player swapped their main hand with `slot` (currently `offhand`), after both hands changed. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_toggle_entity_age_lock(player, name, entity_id, locked)`**: a server player used a golden dandelion on an ageable mob, after its age lock changed and the item was used.
- **`native_player_unleash_entity(player, name, entity_id)`**: a server player's action removed an entity's leash, after its leash state was cleared. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_velocity(player, name, velocity)`**: the server synced changed motion to a player, with `velocity` in blocks per tick. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_stop_spectating_entity(player, name, entity_id)`**: a spectator's camera returned to their own player after they stopped watching `entity_id`.
- **`native_player_map_filled(player, name, map_id)`**: a server player filled an empty map, after the filled map and its saved map data were created. Guest lookup helpers may not find the guest during packet handling.
- **`native_player_teleport_end_gateway(player, name, gateway, cancel)`**: a player is about to travel through the End gateway at `gateway`. Set `cancel` to leave them at the entrance; the gateway does not start its cooldown. The player is not swapped in, so helpers may find no guest through `self.lan`.
- **`native_player_recipe_book_click(player, name, recipe, all)`**: a server player requests a known recipe be placed in their crafting grid through the recipe book. `all` is true for the shift click form. The player is not swapped in, so helpers may find no guest through `self.lan`.
- **`native_player_exp_cooldown_change(player, name, ticks)`**: the player’s XP orb pickup cooldown changed, after its new value is stored. This includes the two tick delay after an orb is picked up and each tick it counts down. The player is not swapped in, so helpers may find no guest through `self.lan`.
- **`native_player_locale_change(player, name, locale)`**: a server player changed their client language option. It fires after the new locale is stored. The player is not swapped in, so helpers may find no guest through `self.lan`.
- **`native_entity_enter_block(entity, kind, pos, block)`**: a bee was stored in the beehive or nest at `pos`.
- **`native_entity_change_block(entity, kind, pos, from, to)`**: a mob changed a block, after the new state was applied.
- **`native_entity_break_door(entity, kind, pos)`**: a mob finished breaking a door at `pos`, after the block was removed.
- **`native_creeper_power(entity, lightning)`**: a lightning strike charged the creeper.
- **`native_creeper_ignite(entity, player, item)`**: a player used a creeper igniter on the creeper, after it entered its ignited state.
- **`native_exp_bottle(pos, experience)`**: an experience bottle broke and its orbs were created.
- **`native_firework_explode(entity, pos, bursts)`**: a firework rocket completed its damage and explosion effects.
- **`native_potion_splash(item, pos)`**: a thrown splash or lingering potion finished applying its hit effects at `pos`. Experience bottles are excluded.
- **`native_entity_transform(entity, kind, transformed, transformed_kind)`**: a mob conversion completed through the shared `Mob.convertTo` path.
- **`native_entity_enter_love_mode(entity, kind, player)`**: a mob entered love mode after a player interaction; `player` is the feeder UUID when known.
- **`native_entity_teleport(dimension, entity, from, to)`**: a server entity completed a same-level teleport command. `entity` is its network id; `from` and `to` are coordinates in `dimension`. This reports the move after it happens.
- **`native_entity_breed(parent, partner, child)`**: two mobs completed a breeding action. `child` is absent when the vanilla action lays an egg instead of creating a baby.
- **`native_entity_tame(entity, kind, owner)`**: a mob became tamed after its owner and persistent state were set. `owner` is the server player's UUID when known.
- **`native_item_spawn(entity, item, count, pos)`**: a dropped item entity was successfully added to the server level. The count is its stack size and `pos` is its spawn position.
- **`native_entity_pose_change(player, name, from, to)`**: a guest player's synchronized pose changed. Pose names are `STANDING`, `CROUCHING`, `SWIMMING`, `FALL_FLYING`, `SPIN_ATTACK` or `SLEEPING`.
- **`native_entity_toggle_glide(player, name, gliding)`**: a guest player's fall flying state changed after the server accepted or ended a glide.
- **`native_entity_shoot_bow(player, name, projectile, force)`**: a player fired an arrow from a bow or crossbow, after the arrow entered the level. `force` is the shooting power. This reports the completed shot.
- **`native_arrow_nock(player, name, bow, hand, has_ammo, cancel)`**: a player is about to start using a bow, where `BowItem.use` runs on the server (NeoForge's `ArrowNockEvent`). `has_ammo` is whether `getProjectile` found ammunition, always true in creative. Set `cancel` to fail the use: the bow is not drawn. Crossbows and tridents do not ask, and the client's own prediction does not.
- **`native_living_get_projectile(player, name, entity, kind, weapon, projectile)`**: the ammunition `LivingEntity.getProjectile(weapon)` returns (NeoForge's `LivingGetProjectileEvent`). Every mod gets the event with `projectile` set to the vanilla ammunition (the stack found in the hands or inventory, a mob's held projectile or a new arrow, a creative player's default arrow, or empty) and may change or empty it. It is asked on the bow's start and release, the crossbow's start and draw, the client's prediction of those starts, a skeleton's or illusioner's shot, and a pillager's or piglin's crossbow load (`entity` and `kind` name the mob, `player` is empty); a mob's arrow takes the replaced arrow's item and effect, a mob's crossbow also fires a replaced firework rocket, and an emptied stack leaves its crossbow uncharged and silent. A replaced stack is not taken from the inventory: the hook owns it.
- **`native_projectile_launch(player, name, entity, kind)`**: a projectile was added to the server level. `kind` is its entity type; `player` and `name` identify a player launcher, and are `None` and empty for other sources.
- **`native_entity_portal(dimension, entity, pos)`**: a server entity finished its portal wait and is beginning to leave through the portal block at `pos`. The destination follows the existing portal rules; the hook reports the transfer without changing it.
- **`native_furnace_extract(player, name, pos, item, count)`**: a server player takes furnace output at `pos`, after the output is removed; the player state is not swapped in.
- **`native_furnace_start_smelt(pos, item)`**: a furnace at `pos` begins cooking `item`, when its cooking timer first advances. Server-side.
- **`native_brewing_complete(pos)`**: a brewing stand finished brewing at `pos`, after the bottle contents changed.
- **`native_brewing_fuel(pos, fuel)`**: a brewing stand at `pos` consumed `fuel` and gained brewing fuel. Server-side.
- **`native_enchant_item(player, name, pos, item, levels)`**: a player applied an enchantment at `pos`; `item` is the finished item and `levels` the levels charged. Guest state is not swapped in.
- **`native_prepare_item_enchant(player, name, pos, costs)`**: enchanting offers were calculated at `pos`; change the three offer costs before they are sent to the player's menu. Guest state is not swapped in.
- **`native_prepare_anvil(player, name, pos, result, cost)`**: an anvil result was prepared for a player at `pos`; change the result or level cost before it is sent to their menu. Guest state is not swapped in.
- **`native_anvil_update(player, name, left, right, item_name, vanilla_output, vanilla_xp_cost, vanilla_material_cost, output, xp_cost, material_cost, canceled)`**: NeoForge's `AnvilUpdateEvent`, where `AnvilMenu.createResult` ends, each time the inputs or the rename box's `item_name` change while the left slot holds an item. `right` is none when that slot is empty, and `item_name` is none until the client sent one (empty clears a custom name). `vanilla_output`, `vanilla_xp_cost` and `vanilla_material_cost` are vanilla's result. `output`, `xp_cost` (the level cost) and `material_cost` (how many of `right` the take uses) start as vanilla's, and every mod in turn may change them. Set `canceled` to leave no result, with both costs at 0; later mods aren't called. A negative level cost becomes 0, and a result that costs 0 levels can't be taken. A material cost of 0 or less, or above what `right` holds, takes the whole stack (none when the anvil only renames). It fires on both sides: on a host (single player's server thread, a LAN or dedicated host) for each player's menu, before `native_prepare_anvil`, and in the window's own menu, whose predicted take then charges what the station shows. The window works its result out again after each click in the menu, so it may ask about the same inputs again: a mod should answer from what it is told. Guest state is not swapped in.
- **`native_anvil_craft_pre(player, name, pos, craft, cancel)`**: a player took an anvil's result at `pos` (NeoForge's `AnvilCraftEvent.Pre`), before the levels are charged, the inputs used up and the anvil worn. `craft` has the left and right inputs and the result as they were before. Set `cancel` to refuse the take and skip `native_anvil_craft_post`: the result stays in its slot and the inputs, levels, inventory and cursor are as before the click. Fires on the server only; the window's predicted take does not call it.
- **`native_anvil_craft_post(player, name, pos, craft)`**: the levels, inputs and anvil wear of that take are done (NeoForge's `AnvilCraftEvent.Post`); a broken anvil is already air. `craft` is the same as in `native_anvil_craft_pre`. Not called when it cancelled.
- **`native_prepare_grindstone(player, name, pos, result)`**: a grindstone result was prepared for a player at `pos`; change it before it is sent to their menu. Guest state is not swapped in.
- **`native_prepare_smithing(player, name, pos, result)`**: a smithing result was prepared for a player at `pos`; change it before it is sent to their menu. Guest state is not swapped in.
- **`native_prepare_item_craft(player, name, pos, result)`**: a crafting-grid result was prepared; `pos` is the crafting table, or None for the player's inventory grid. Change it before it is sent to the menu. Guest state is not swapped in.
- **`native_trade_select(player, name, villager, index)`**: a player selected offer `index` in a villager's trading menu; `villager` is its entity id when known. The menu has updated and the player's state is not swapped in.
- **`native_hopper_pickup_item(dimension, pos, item, count, cancel)`**: a hopper at `pos` is about to pick up a dropped stack; set `cancel` to leave it in the world.
- **`native_weather_change(dimension, raining, thundering, cancel)`**: the server's `/weather` command is about to set
  its final rain and thunder states in `dimension`. Set `cancel` to keep the previous weather parameters, including
  their timers.
- **`native_thunder_change(dimension, thundering, cancel)`**: the server's `/weather` command is about to change
  thunder. Set `cancel` to keep the previous thunder state and timer.
- **`native_lightning_strike(player, pos, cause, cancel)`**: a weather, Channeling-trident, or `/summon` lightning bolt
  is about to be created. Set `cancel` to prevent the bolt and its effects; `cause` names the creation path.
- **`native_time_skip(dimension, from, to, cause, cancel)`**: the server clock is about to jump after enough players
  sleep or through `/time set`, `/time add`, or `/time set <marker>`. Set `cancel` to keep the old time.
- **`native_sleep_finished_time(dimension, current, new_time)`**: enough players slept through the night and the
  `advance_time` rule is on (NeoForge's `SleepFinishedTimeEvent`). `new_time` comes in as the next morning's day time;
  set another to wake everyone then. A time before `current` is refused and the morning stays; the check runs once after
  every mod, so a later mod sees the value an earlier one set. It runs before `native_time_skip`, which hears the jump
  to the time chosen here and may still cancel it; the players wake either way. Bundles need
  `"hooks": 308`.
- **`native_world_border_bounds_changed()`**: `/worldborder set` or `/worldborder add` changed the border size or started a timed size change. Bundles need `"hooks": 245`.
- **`native_world_border_center_changed()`**: `/worldborder center` moved the border center. It hears successful commands only. Bundles need `"hooks": 246`.
- **`native_world_game_rule_change(player, name, rule, value)`**: an operator changed a game rule through the server game rules screen; it is heard after the final value is applied. Bundles need `"hooks": 247`.
- **`native_spawn_change(pos, angle)`**: `/setworldspawn` moved the world's spawn point, after its position and rotation are applied. Bundles need `"hooks": 244`.
- **`native_portal_create(dimension, pos)`**: a Nether portal built for dimension travel or the End arrival platform was completed in `dimension`; `pos` is its lower corner or platform origin. Bundles need `"hooks": 286`.
- **`native_raid_spawn_wave(raid, pos, wave, raiders)`**: a raid wave is about to be spawned at `pos`; `raiders` is the count Horizon will spawn.
- **`native_raid_state_change(raid, pos, state)`**: a raid at `pos` was triggered or changed to `victory`, `loss`, or `stopped`.
- **`native_world_saved(wait)`**: the server completed its world save cycle; `wait` is true when chunk writes were flushed before returning.
- **`native_entities_load(dimension, chunk, count)`**: saved entity records for `chunk` finished loading into `dimension`.
- **`native_entities_unload(dimension, chunk, count)`**: entities in `chunk` left the active world and were queued for saving.
- **`native_game_event(event, pos)`**: a generic world game event was fired at `pos`, including events with no nearby vibration listeners.
- **`native_tab_complete(player, name, command, range, suggestions)`**: a guest's server completion request was evaluated; mods may add or filter suggestions. `range` is the replaced byte range. Guest state is not swapped in, and helpers that look through `self.lan` may find nothing while the guest packet is handled.
- **`native_broadcast_message(line, cancel)`**: a system broadcast line is about to be logged and sent to every player. A mod may change `line` or set `cancel` to drop the broadcast.
- **`native_whitelist_state_update(enabled)`**: the dedicated server's player whitelist was enabled or disabled.
- **`native_player_bundle_item_select(player, name, slot, index)`**: a server player selected an item in a bundle;
  `slot` is the packet's menu slot and `index` is the chosen item index.
- **`native_player_ready_arrow(player, name, arrow)`**: a server player released a drawn bow, with `arrow` naming
  the selected projectile item. The guest's state is not swapped in while the packet is handled.
- **`native_prepare_hopper_transfer(pos)`**: a hopper at `pos` begins a transfer attempt, after its cooldown and enabled checks and before pushing or pulling items.
- **`native_inventory_move_item(source, destination, item, cancel)`**: a hopper is about to move one item between containers (a composter's bone meal too). Set `cancel` to leave it in the source slot. A chest boat or minecart is reported at the block it is in.
- **`native_hopper_inventory_search(hopper, container, entity)`**: a hopper selects a block container or container entity while searching for items.
- **`native_vault_loot(pos, item)`**: a vault is about to eject a loot item, before the item entity spawns.
- **`native_world_border_bounds_change_finish()`**: a timed world border size change finished on the server, after the border reached its final size.
- **`native_player_launch_projectile(player, name, item)`**: a server player launched a snowball, egg, ender pearl,
  potion or trident. It fires after the projectile is created and only for guest players in a LAN world.
- **`native_entity_unleash(entity, kind)`**: a mob's leash was removed, before any lead drop is handled.
- **`native_entity_drop_item(entity, item, count, pos)`**: a mob is about to drop `count` of `item` at `pos`.
- **`native_item_despawn(entity, item, count, pos)`**: a dropped item entity reached its 6000-tick lifetime and is
  about to be removed.
- **`native_explosion_prime(center, radius)`**: a server explosion is about to begin at `center` with `radius`.
  It is listen-only, so the explosion proceeds as usual; it also fires on the single-player server.
- **`native_vehicle_damage(kind, damage, cancel)`**: a player is about to damage a boat or minecart. Change `damage`
  or set `cancel` to leave its damage and wobble unchanged; the attack still completes. `kind` is `boat` or the
  minecart type. Server-side.
- **`native_vehicle_destroy(kind, pos)`**: a boat or minecart is about to be destroyed at `pos`. TNT minecarts that
  prime instead of breaking are not heard. Server-side.
- **`native_vehicle_create(kind, pos)`**: a boat or minecart joined the level at `pos`, after being placed by a
  player or dispenser. Server-side.
- **`native_player_toggle_sneak(player, name, sneaking)`**: a guest's player input changed sneaking, after the hub applied the state. The guest's state is swapped in; only actual changes are reported.
- **`native_player_input(player, name, flags)`**: a loaded guest sent a `PlayerInput` packet, after its flags were applied to the hub. `flags` is the packet's bit field, and the guest's state is swapped in.
- **`native_player_arm_swing(player, name, hand)`**: a guest's arm swing was handled and their attack strength reset. The packet currently swings `mainhand`; the guest's state is swapped in.
- **`native_player_jump(player, name)`**: accepted guest movement made the server player jump from the ground, after movement simulation finishes. The guest's state is swapped in.
- **`native_player_kicked(player, name, reason)`**: a guest was kicked after the hub recorded the rendered disconnect reason and sent the disconnect. The guest's state is swapped in, unless the guest was already removed in the same poll (a kick and the disconnect arriving together): the hook still fires, without it.
- **`native_player_level_change(player, name, from, to)`**: a server player's experience change altered their level, after the new level was applied. The guest's state is swapped in when the player is a guest.

#### Server hooks for plugin ports: console and log

- **`native_console_line(source, line, cancel, replies)`**: a line the dedicated server's console is about to run, before
  any command sees it (vanilla's, the dedicated-only ones such as `stop`, and `native_perform_console_command`'s).
  `source` is `console` for a line typed on the console or sent by `horizon-server` as the console (its console, `cmd`,
  the panel, the `save-off` around its backups) and `rcon` for an RCON command; `line` is as given, a leading slash
  included. Set `cancel` and it does not run. Push to `replies` to answer: each line is logged as the console's
  feedback is, so it also reaches the RCON client or the `horizon-server` caller. There is no player (the console is
  level 4). An audit log's `ServerCommandEvent`/`RemoteServerCommandEvent`. Bundles need `"hooks": 463`.
- **`native_log_line(level, thread, message, line)`** (`horizon_client_lib::native_client_hooks`): a message was
  written to `logs/latest.log`, so a console bridge needs no tailing of the file. `level` is `INFO`, `WARN` or `ERROR`,
  `thread` the thread the line names, `message` the text and `line` what the file got (`[HH:mm:ss] [thread/LEVEL]:
  message`). It runs on the log's own writer thread after the file is written, never on the server or render thread,
  so a slow mod holds up no tick; it has no `Game`, and a line a mod logs from it comes back to it later (don't log
  each line you hear). Lines the flood limit leaves out are not heard, and a mod that panics in it is not called again.
  Bundles need `"hooks": 471`.

#### Server hooks for plugin ports: connection

What a proxy-fronted backend, a plugin channel or a login gate needs, which plugin ports reached with raw injections
before (#10461).

- **`native_handshake(address, host, intent, cancel, reason)`**: a free function in `horizon_client_lib::lan::conn`,
  heard on the connection's own thread for each network connection's handshake (`ClientIntentionPacket`), before the
  server acts on `intent` (1 status, 2 login, 3 transfer). `address` is the peer's (behind a proxy, the proxy's; the
  forwarded address comes later, with the login), `host` the address the client says it connects to, with whatever a
  proxy put after it (Floodgate's `\0^Floodgate^...` data). A mod may rewrite `host` (later mods see the new one; the
  server itself reads nothing from it), or set `cancel` to refuse the connection: a login or a transfer is disconnected
  with `reason` (a plain "Disconnected" if unset), a status ping is closed unanswered. The legacy `0xFE` ping and an
  integrated server's own player are not heard. There is no `Game` on this thread, so no helper works and `native_tell`
  reaches nobody; keep the hook short, the connection waits for it. Bundles need `"hooks": 466`.
  With proxy forwarding on (`forwarding.secret`, offline mode) the server reads the handshake's host up to 32767
  characters, as Paper does behind a proxy, so a Floodgate player's long data reaches the hook; without forwarding the
  limit stays vanilla's 255.
- **`native_custom_payload_from_client(player, channel, data)`**: a player's plugin channel message
  (`ServerboundCustomPayloadPacket` in play) on any channel that isn't Horizon's own (`horizon:mod_net`,
  `horizon:quiet_commands`, `horizon:lod`), which vanilla drops: a Simple Voice Chat client's `voicechat:*` requests,
  a proxy's or a client mod's channels. `data` is the client's bytes, checked by nothing. It runs in the tick after the
  message arrived, in arrival order, with the hub back and no player's state swapped in, so every helper works; it is
  heard from a dead player too. Every guest is heard (a single player world's own player sends none). The client side,
  a window's own plugin channels, is #6774. Bundles need `"hooks": 465`.
- Helper **`native_send_custom_payload_to_player(player, channel, data)`** (`host.send_custom_payload_to_player`):
  sends `data` on a plugin channel (`namespace:path`, not Horizon's own `horizon:` ones, at most 1 MiB as
  `ClientboundCustomPayloadPacket` takes) to one player's client, a vanilla client included, as Bukkit's
  `Player.sendPluginMessage`. It is queued and leaves at the tick's end, so it works from any server hook, a player's
  own included; `Ok` means queued, and a player who isn't on by then gets nothing. Unlike `native_net_send_to_player`
  nothing caps or frames it. An error on a client, and for a bad channel or size. Bundles need
  `"hooks": 504`.
- Helper **`native_player_permission(player)`** (`host.player_permission`): the permission level of a player who is on
  the server or joining it, 0 to 4, an operator's from `ops.json` above 0 (`PlayerList.getProfilePermissions`). It also
  works inside `native_player_login`, for the player that hook hears (their level is set by then), so a login gate that
  lets operators through needs no `ops.json` of its own. Inside a player's hook, with the hub out of reach, only that
  player is found; None for a player who isn't on, and on a client. Bundles need `"hooks": 485`.

#### Server hooks for plugin ports: players

- **`native_player_logging_out(player, name)`**: a player is leaving (lost connection, kick, quit), heard while they are
  still in their level and listed in `native_server_players`, before their menu closes and they are saved and removed
  (Bukkit's `PlayerQuitEvent`). Their state is not swapped in, so the helpers that take a UUID and `native_run_command`
  reach them. A death a mod causes here (`kill <name>`) happens at once where they stand, with its death message, drops
  and `native_player_died`, and they are saved dead. Once per placed player who leaves, before `native_player_logged_out`.
  Not heard when the server stops (`native_player_logged_out` tells then, with them still listed).
- **`native_player_hurt_by_player(attacker, attacker_entity, victim, victim_entity, source, damage_type, amount, canceled)`**:
  one player hurting another, both named by UUID and network entity id. `source` is `melee`, `sweep` or the projectile's
  entity type (`minecraft:arrow`, `minecraft:spectral_arrow`, `minecraft:snowball`, `minecraft:egg`, a mod's thrown type).
  Heard where `native_living_incoming_damage` is (the victim's `hurtServer` past its guards: the `pvp` rule and the teams
  allow it, the victim is not creative, a spectator or dead; a hit in the hurt cooldown is heard too) and just before it.
  Change `amount`, or set `canceled` and nothing of the hurt happens; v1 then hears nothing of it. A snowball or egg does
  no damage and vanilla's `Player.hurtServer` stops on zero: it is heard there with amount 0, and what a mod sets changes
  nothing. No `self`, like v1. Not heard: thorns, Channeling's lightning, fire, potion and tipped-arrow effects, thrown
  tridents (they hurt no players yet).
- **`native_player_spawn_location(player, name, dimension, position, rotation)`**: a joining player is about to be placed
  (Spigot's `PlayerSpawnLocationEvent`): after `native_player_login` and their saved data (or a newcomer's spawn), before
  the level and the world around them are chosen and sent. Change the dimension, feet position or yaw/pitch and they
  appear there, with no teleport afterwards; an unknown dimension or a non-finite value leaves that part as it was.
  No state is swapped in.
- **`native_item_picked_up_v2(player, stack, thrower, position, canceled)`**: an additive version of
  `native_item_picked_up` with the whole `ItemStack` (its components: enchantments, unbreakable, attribute modifiers, a
  custom name...), the item entity's thrower UUID (none for block and mob drops) and its position. Heard just before v1,
  which keeps its signature and firing point and hears nothing of a pickup canceled here.
- **`native_item_dropped_v2(player, stack, cancel)`**: an additive, cancellable version of `native_item_dropped`
  (NeoForge's `ItemTossEvent`) with the whole `ItemStack` thrown, heard just before v1, which keeps its signature and
  firing point. Set `cancel` and nothing is thrown: the stack goes back into the player's inventory (the held slot
  first; only what doesn't fit is thrown, unheard), and v1 hears nothing of it.
- **`native_creative_slot_v2(player, name, slot, stack, cancel)`**: an additive version of `native_creative_slot` with
  the whole `ItemStack` put in the slot (none for an emptied one), heard just before v1, which keeps its signature and
  firing point and hears nothing of a packet canceled here. The player's state is swapped in, as for v1.

#### Server hooks for plugin ports: world

Helpers for server mods that work on the levels as a whole (a pre-generator, a web map, a view distance tuner), on a
server only.

- **`native_force_chunk_in(owner, dimension, [x, z], hold)`** (`native_chunk_tickets.rs`): `native_force_chunk`
  ([Chunk tickets](#chunk-tickets)) for the level `dimension` (`minecraft:the_nether`), whichever level is in place. A
  level nobody is in, or one not open yet, opens at the next server tick and ticks while a ticket holds it, so a
  pre-generator walks the Nether and the End from any server hook (a console command, `native_level_tick_pre` of the
  Overworld) with nobody there. False for a dimension other than the Overworld, the Nether and the End. API
  413.
- **`native_world_border(dimension)`** (`native_world_border.rs`): the level's world border as the server enforces it,
  `[min_x, min_z, max_x, max_z]` in blocks (the previous size while it moves), for a web map's border line or a
  pre-generator's `worldborder` selection. Vanilla 26.3 keeps a border per level (`ServerLevel.getWorldBorder`, each
  level's `data/minecraft/world_border.dat`, `/worldborder` changing the level it runs in); Horizon keeps one for every
  level today, so the three levels answer the same bounds until that is fixed, and the helper then answers each level's
  own. None for another dimension. API 515.
- **`native_set_player_view_distance(uuid, distance)`** (`native_view_distance.rs`): limits one player's view distance
  to `distance` chunks (2 to 32), None gives it the server's again. The player's distance is the smallest of the
  server's, what its client asked for and the limit; only that player is told its radius (`SetChunkCacheRadius`), and
  from its next view update it is sent, keeps and tracks chunks and entities within it, the others as before. The
  server still loads its own view distance round every player, so the limit can only lower a player's distance (raise
  the server's and limit the others to give one player more). It lasts until the player leaves.
  `native_player_view_distance(uuid)` is the distance the player has now. Both answer false/None for a player not online
  and inside a guest's packet hook, where the hub is taken out: call them from a tick or console hook. API
  495.
- **`native_set_player_fake(uuid, fake)`** (`native_view_distance.rs`): marks the player as a fake player, a mod's bot
  with no client, or not. A fake player keeps only a small area loaded and ticking around it (its view distance plus
  one, at most the server's plus one), its packets are cut to the few a bot needs (position, entity motion, the chunk
  batch end, keep-alive, ping, combat kill and respawn), and it does not count for natural mob spawning. The mark lasts
  until the player leaves. False off a server, for a player not online, and inside a guest's packet hook: call it from a
  tick or console hook. API 605.
- **`native_expect_fake_player(name, fake)`** (`native_view_distance.rs`): says that a player named `name` (any case)
  will join as a fake player, or not any more. The player is fake from its first packet on, so its join sends and
  builds nothing a client-less player cannot read (no login records, recipe book, command tree, player list or
  advancements). False off a server. API 602.
- **`native_is_player_fake(uuid)`** (`native_view_distance.rs`): whether the player is a fake player, a mod's bot with no
  client (marked by `native_set_player_fake`, or announced by name with `native_expect_fake_player`, which makes it fake from
  its first packet on). False off a server and for a player not online; for a join hook that gets only a uuid. API
  603.
- **`native_chunk_snapshot(dimension, [x, z])`** (`native_chunk_snapshot.rs`): a `NativeChunkSnapshot` of a loaded
  chunk of a level that is open (in place or with nobody in it), None for a chunk not loaded (nothing is loaded for
  it). It shares the chunk's sections until the level changes them, so it is cheap to take on the server thread and
  can be read later on another one. It reads `block_state(x, y, z)` (a `BlockStateId`, for `native_state_block` and the
  state helpers), `surface(x, z)` (`WORLD_SURFACE`), `noise_biome(qx, qy, qz)` and `biome_cell(x, y, z)` (the quart
  cell vanilla's fuzzy biome zoom reads for a block, which can lie in a neighbouring chunk), `height()` and
  `has_ceiling()`, in the level's block coordinates; outside the chunk it answers None. To draw it:
  `native_block_map_color(state)` is `(MapColor id, MapColor.col)`, `native_biome_name(biome)` the biome's name and
  `native_biome_tint(biome, x, z)` its grass (with the dark forest and swamp modifiers), foliage, dry foliage and water
  colours (`BiomeColors`, after `native_biome_colors`, not blended). API 460.

#### Server hooks for plugin ports: permission nodes

Permission nodes beyond operator levels: a permissions mod (a LuckPerms port) answers, every other mod and the
server's command tree ask. Without such a mod every answer is the operator level, as before.

- **`native_permission_check(player, node, default_level, result)`**: whether a player has the permission node `node`
  (`essentials.tpa`, `minecraft.command.gamemode`), as Bukkit's `Permissible.hasPermission` and NeoForge's
  `PermissionAPI.getPermission` ask it. A permissions mod sets `result`; the first mod that sets it decides. Left None,
  the answer is the player's operator level against `default_level` (0 lets everyone). It is asked through `&self`
  (a provider reads what it knows) and often with the hub out of reach, for players on the server, joining or unknown
  to it, so answer from the mod's own data. Server-side. Bundles need `"hooks": 473`.
- Helper **`native_has_permission(player, node, default_level)`** (`host.has_permission`): asks
  `native_permission_check`, and when no mod answers, whether the player's operator level is `default_level` or above.
  The level is the server's for a player who is on or joining, the acting player's inside their hook; anyone else, and
  every player on a client, counts as level 0. Bundles need `"hooks": 473`.
- Helper **`native_player_permission_value(player, node)`** (`host.player_permission_value`): the node as the
  permissions mods set it, three-valued as Bukkit's `isPermissionSet` against `hasPermission`: Some(true), Some(false)
  (a mod set it to false, which is not the same as unset), or None when no mod answers `native_permission_check` (asked
  with a `default_level` of 0, no operator fallback). A mod applies its own default and parent-node inheritance on None.
  The operator level is `native_player_permission(player)`. Works for players on the server or not. Server-side.
  Bundles need `"hooks": 631`.
- **`native_player_commands_sent_v2(player, name, names, names_hidden)`**: an additive version of
  `native_player_commands_sent`, heard right after v1 at the same times (join, a change of level, a resend) with
  `names` as v1 left it. `names_hidden` lists the root commands an operator of level 4 would get that this player's
  level hides (`gamemode`, `op`, a mod's command above their level). Move one into `names` and the player's tree has it
  with everything under it, as an operator of its level gets it (Paper's `PlayerCommandSendEvent` after a permission
  check passed), so their client completes and highlights it; take a name out of `names` to hide it, as with v1. Only
  the tree changes: running a granted vanilla command still needs its level (`native_player_command_level`). Mods are
  asked in order and share both lists. Bundles need `"hooks": 477`.
- **`NativeCommand::permission(node)`** (`NativeCommand::new("tpa <player:player>").permission("essentials.tpa")`): a
  mod's command that a permission node decides, as Bukkit's `plugin.yml` `permission:`. Before the server puts it in a
  player's tree and before it runs a player's typed one, it asks `native_has_permission(player, node, level)` with
  the command's level as the default, so a permissions mod grants it to a player below that level or refuses it to an
  operator; without one the level decides, as for a command without a node. `native_player_commands_sent_v2` then
  hears a command its node refuses in `names_hidden` and one it grants in `names`. A command's node is the first one
  its usages give. A dedicated server's console runs every registered command without asking. Bundles need
  `"hooks": 462`.
- **`native_player_command_level(player, name, command, level)`**: the operator level one typed command of a player
  runs at, before the server checks it (Paper's `VanillaCommandWrapper`, which asks `minecraft.command.<name>` instead
  of the level). `level` comes in as the player's own; raise it to grant this command (`/gamemode` at 2 for a player of
  level 0) or lower it to refuse one; above 4 counts as 4. `command` is the line without its slash. The command then
  runs at that level: a vanilla one through its own checks, a mod's against its level (one with a permission node asks
  the node, with this level as the fallback). Heard after `native_player_command` for the commands vanilla's and the
  mods' dispatch run; not for the chat commands the server answers itself (`/me`, `/msg` and its aliases, `/say` from
  a player allowed it), `/list`, the chat filter's and Horizon's `/tempban` commands, nor a mod's quiet commands, and a LAN world without Allow Commands still refuses its guests' vanilla
  commands. The hub is out of reach and no player's state is swapped in: answer from the mod's own data. Server-side.
  Bundles need `"hooks": 476`.
- Helper **`native_resend_commands(player)`** (`host.resend_commands`): sends a player their command tree again, with
  `native_player_commands_sent`, its v2 and the permission nodes of the mods' commands asked anew, as Bukkit's
  `Player.updateCommands`: after a permissions mod changed what they may use (a `/lp` edit, a sync from another
  server). The tree goes out in the next server tick, so it works from any server hook, the player's own included
  (calling it from the commands-sent hooks themselves would send the tree every tick). True when the player is on;
  false for anyone else and on a client. Bundles need `"hooks": 499`.

#### Running commands from mods

- `native_run_command_captured(line) -> NativeCommandResult`: `native_run_command` with the answer kept (vanilla's
  `CommandSourceStack.withCallback` on a source that keeps what it is sent). `line` (a leading slash allowed) runs as
  the console; a command a mod registered goes to the mods (`native_perform_console_command`), anything else to
  vanilla's. `ran` says it ran without failing, `result` is vanilla's result value (the sum of the successful results,
  one per fork of `execute as @a run ...`; 0 for a failed `execute if entity`; 1 for a mod's command a mod ran), and
  `output` holds the lines it answered with, which reach no one else, operators included. Where `native_run_command`
  runs (no player swapped in); inside a player's hook nothing runs. Bundles need `"hooks": 502`.
- `native_run_command_as(player, line) -> NativeCommandResult`: `line` runs as that player typing it would (Bukkit's
  `Player.performCommand`): their source, operator level and position, so `/gamemode creative` for a level-0 player
  is refused with vanilla's error; a command a mod registered goes to the mods when their level allows it. The answer
  is sent to them as their own command's, and the same lines come back in `output`. `native_player_command` doesn't
  hear it. Where `native_run_command` runs, or inside that player's own hook (`native_perform_command`: the lines
  join its feedback); for anyone else, or a player not on the server, nothing runs. Bundles need
  `"hooks": 501`.
- `native_run_mod_command(line) -> bool`: `native_run_command` that reaches the mods' commands too (Bukkit's
  `Bukkit.dispatchCommand` from the console): a command a mod registered goes to the mods
  (`native_perform_console_command`), anything else to vanilla's. True when a mod took it or vanilla's ran without
  failing; no output. Bundles need `"hooks": 503`.
- `native_command_info(label) -> Option<NativeCommandInfo>`: the server's command `label` names (Bukkit's
  `CommandMap.getCommand`), a leading slash allowed: one a mod registered, else vanilla's with an alias resolved (`tp`
  gives `teleport`, whose `aliases` hold `tp`). `usages` are the mod's registered usages, or vanilla's one line as
  `/help` gives it; `level` is the operator level a player needs, `vanilla` whether vanilla has it. None when no
  command has that name. Bundles need `"hooks": 461`.
- Argument kinds that ask the server, for `native_register_commands`' usages: `word?` (a word) and `text?` (the rest
  of the line, last) are sent with vanilla's `ask_server` suggestions, so the player's client asks the server for
  their completions instead of completing nothing; `player_name` is a word that does the same, any name accepted
  (offline ones too, unlike `player`), and the server offers the online players' names matching it.
  **`native_tab_complete_v2(player, name, label, arguments, range, suggestions)`** hears every completion request
  right after `native_tab_complete`, the command split as Bukkit's `TabCompleter` gets it: `label` is its first word,
  `arguments` the words after it split at each space, the last the one being typed (empty after a space). For a
  command a mod registered, `range` is that word, and for `player_name` `suggestions` already holds the names; push
  yours over `range`. No player's state is swapped in and the server's players are out of reach
  (`native_server_players` gives none). Bundles need `"hooks": 510`.

#### Server hooks for plugin ports: Movement and teleports

Hooks before players and projectiles move, so a port can stop or redirect a move instead of sending the player back a
tick later with a command.

- **`native_player_move(player, name, from, to, yaw, pitch, on_ground, cancel)`**: a server player's position or look
  packet that changed either and passed the server's checks (moved too quickly, moved wrongly, the anticheat), before
  it is applied (Bukkit's `PlayerMoveEvent`). `from` and `to` are the feet before and after. Set `cancel` and the
  player is sent back to `from`; change `to` and they are put there instead (not checked against the blocks) and their
  client gets a teleport there. Not heard: riders, sleepers, moves while a teleport waits for its answer, teleports.
  Once a move, inside the packet's handling: keep it cheap. No `self`, no state swapped in. Server-side.
- **`native_player_teleport(player, name, cause, from_dimension, from, to_dimension, to, relative, cancel)`**: a server
  player is about to be teleported, before their client hears it (Bukkit's `PlayerTeleportEvent`, NeoForge's
  `EntityTeleportEvent`). `cause` is `command`, `ender_pearl`, `chorus_fruit`, `spectate` (the spectator teleport
  menu), `dismount` or `unknown` (another server move of the player). `to_dimension` differs from `from_dimension` only
  for a `command`, `spectate` or `unknown` (a mod's `native_teleport_player`) teleport into another level. Change `to` and they go there instead; set `cancel` and
  they stay where they are. Within a level `relative` is the position packet's flags: a mod may set or clear the look and motion bits (8, 16,
  32, 64, 128, 256; 0x1f8 keeps a pearl's speed as 26.2 did), the position bits are ignored; into another level it is
  not used. Not heard: portals and End gateways (their own hooks), respawns, a rider its vehicle carries, waking from a
  bed, a move the server's checks send back, the spectator camera. No `self`, no state swapped in. Server-side.
- **`native_player_respawn_position(player, name, dimension, pos, yaw, pitch)`**: a dead player is about to respawn
  (NeoForge's `PlayerRespawnPositionEvent`, Bukkit's `PlayerRespawnEvent.setRespawnLocation`), before they are placed:
  their respawn point (a bed or anchor block's top, a forced point, else the world spawn) and the look they get (0, 0).
  Left alone, the point is resolved as vanilla's is (the spot beside the bed, the world spawn's free column). Change the
  dimension or position and they are placed exactly there, no bed or anchor used; an unknown dimension or a non-finite
  value leaves that part as it was. On a LAN host's world the level stays the Overworld. No state swapped in.
- **`native_trample(dimension, entity, kind, pos, fall_distance, cancel)`**: a player or mob (`entity` its network id,
  `kind` its type) that fell `fall_distance` onto the farmland at `pos` is about to turn it to dirt, past vanilla's
  chance and checks (NeoForge's `FarmlandTrampleEvent`, Bukkit's `PlayerInteractEvent` `PHYSICAL` and
  `EntityInteractEvent`). Set `cancel` and the farmland and its crop stay; the fall damage is taken either way. Pressure
  plates and tripwires are `native_player_physical_trigger`. No `self`. Server-side.
- **`native_projectile_world_border(dimension, entity, kind, pos, velocity, collide)`**: a thrown projectile (pearl,
  snowball, egg, potion...) inside the world border whose next step would end outside it, before 26.3 stops it on the
  border like a wall (MC-310476). A mod may change `pos` and `velocity`, or set `collide` false and it flies through
  this tick as before 26.3. Arrows and tridents, which Horizon does not stop at the border, are not heard. No `self`.
  Server-side.

#### Server hooks for plugin ports: Inventories

- **`native_player_launch_projectile_v2(player, name, stack, cancel)`**: a guest is about to throw `stack` (a snowball,
  an egg, an ender pearl, a mod's thrown item), heard where `ProjectileItem.use` creates the projectile, before it
  exists (Bukkit's `ProjectileLaunchEvent` for a player). Set `cancel` and nothing is thrown, used or put on cooldown,
  and `native_player_launch_projectile` (v1, which keeps its signature and firing point) hears nothing of it; the
  client, which took the item at once, gets its inventory back. The guest's state is swapped in. Not heard: bows,
  crossbows, tridents, splash and lingering potions, experience bottles, wind charges and fireworks.
- **`native_inventory_click(player, name, window, slot, button, mode, cancel)`**: a guest's click in their own
  inventory (`window` 0) or a vanilla menu they have open (a chest, a furnace, a crafting table, a station, a merchant,
  a mount), heard before it is applied (Bukkit's `InventoryClickEvent`), with the packet's `slot` (the window's
  numbering, -999 outside), `button` and `mode` (`ClickType`'s id: 0 pickup, 1 quick move, 2 swap, 3 clone, 4 throw,
  6 pickup all). Drags (mode 5) are heard once, at their end, by `native_inventory_drag`; a mod's own menus are `native_menu_clicked`'s; spectators are not
  heard. Set `cancel` and nothing moves: the click is handled as one that does nothing, so the client's predicted
  slots and cursor are answered with the server's. No state is swapped in; the helpers that take a UUID reach them.
- **`native_inventory_drag(player, name, window, kind, slots, cancel)`**: a guest's drag (quick craft) in their own
  inventory or a vanilla menu ends, heard before it is applied (Bukkit's `InventoryDragEvent`): `kind` 0 spreads the
  cursor evenly (left button), 1 puts one in each slot (right), 2 clones (middle, creative), and `slots` are the window
  slots the client dragged over, in order (the menu still skips those that cannot take the stack). Set `cancel` and
  nothing moves: the drag ends empty and the client's predicted slots and cursor are answered with the server's.
- **`native_block_place(player, name, stack, pos, state, cancel)`**: a guest is about to place `stack` as the block
  `state` at `pos`, heard where `BlockItem.place` has found the position and the state and checked that the block
  survives there unobstructed, before anything is set (Bukkit's `BlockPlaceEvent`, NeoForge's
  `BlockEvent.EntityPlaceEvent`). Set `cancel` and nothing is placed or used (a failed placement, so the hand does not
  swing); the client sees the blocks and its inventory as they are, and `native_place_block` hears nothing. The guest's
  state is swapped in. `state` is `getStateForPlacement`'s, before the stack's `block_state` component; finding it asks
  `native_state_for_placement` and `native_can_survive` once more. Unlike claiming `native_use_item_on`, the block's
  own use (a door, a chest) and the item's other uses are left alone.
- Helper `native_set_inventory(uuid, kind, slots, selected) -> Result<(), String>`: replaces a player's whole
  inventory (`Player`: 41 slots in `native_inventory`'s numbering) or ender chest (`Ender`: 27), as Bukkit's
  `PlayerInventory.setContents` does: live when they are on the server, else in their file, never while they log in,
  are dead or while a player's state is swapped in (call it from `native_server_tick`). For `Player`,
  `selected` (0..9) becomes the held hotbar slot, the cursor and the 2x2 crafting grid are emptied, and a player on
  the server is sent the whole inventory at once. Err for a wrong number of slots.
- Helpers `native_set_inventory_slot(uuid, kind, slot, expected, new) -> Result<Option<ItemStack>, String>` and
  `native_take_stacks(uuid, slots) -> Result<Vec<ItemStack>, String>`, checked like `native_move_stack` and live or in
  the file like it: the first puts `new` in one slot (armour and the off hand too), which must still hold exactly
  `expected`, and returns what was there (Bukkit's `setItem`); the second takes the stacks out of the given `Player`
  slots, each of which must still hold exactly its stack, all or none (a stake, a trade, a fine with enchanted gear).
  A player's screen follows with the tick's sync.
- **`native_player_inventory_changed(player, stacks)`**: a player's inventory changed, heard at the moment vanilla's
  `InventoryChangeTrigger` is checked (the per-tick check found another item or count in a slot), before any criterion
  is looked at and whether or not any advancement uses `inventory_changed`. `stacks` are the 41 slots in
  `native_inventory`'s `Player` numbering: 0..36 the main inventory (0..9 the hotbar), 36..40 the armour (feet, legs,
  chest, head), 40 the off hand. It also fires at a player's first check after joining, so a mod sees what they log in
  with. The player's state is swapped in. Heard for the single player, LAN guests and a dedicated server's players,
  never on a client of a remote server: what quest, collection and "obtain an item" mods need instead of polling
  every inventory. Bundles need `"hooks": 579`.

#### Server hooks for plugin ports: login and profiles

What a skin, login or profile mod needs, which plugin ports reached with raw injections or their own `usercache.json`
parsing before (#10461).

- **`native_player_pre_login(player, name, address, defer)`**: a free function in `horizon_client_lib::lan::conn`,
  heard on the connection's own thread for a player logging in, once the session service or the proxy has named them
  and before the server's own checks (Bukkit's `AsyncPlayerPreLoginEvent`). `address` is theirs (behind a proxy, the
  forwarded one). A mod that needs time, a permissions or inventory load from a database, or may refuse, pushes a
  `Receiver<Result<(), Text>>` onto `defer` and answers on it from any thread: `Ok` lets the player on, `Err` refuses
  them with that reason. The login waits for every answer, 20 s at most in all (a vanilla client gives up after 30 s
  of silence); an answer that doesn't come by then, or whose sender is dropped, refuses with a plain "Disconnected".
  Mods' loads run side by side. The integrated server's own player is not heard, and there is no `Game` on this
  thread. Bundles need `"hooks": 487`.
- **`native_player_profile(player, name, properties)`**: a free function in `horizon_client_lib::lan::conn`, heard on
  the connection's own thread once the server has let a player in (name, whitelist, bans and player limit checked) and
  before it sends `LoginFinished` (Velocity's `GameProfileRequestEvent`). `properties` are the profile's, as the session
  service (online mode) or the proxy (forwarding) gave them, none in offline mode. A mod may change, add or remove them,
  within `GameProfile`'s limits (names up to 64 characters, signatures up to 1024): the player's own client and every
  player list get the new ones, so a skin mod sets a stored `textures` here before anyone sees the old one. The
  integrated server's own player is not heard. There is no `Game` on this thread, so no helper works; the hook may block
  for a lookup, the login waits for it (a vanilla client gives up after 30 s of silence). Bundles need
  `"hooks": 488`.

- Helpers **`native_set_profile_property(player, name, value)`** (`host.set_profile_property`) and
  **`native_refresh_player(player)`** (`host.refresh_player`): a skin changed while the player is on (Paper's
  `Player.setPlayerProfile`, then `hidePlayer`/`showPlayer`). The first sets the property `name` of an online player's
  profile to `value` (its value and signature, None unsigned) or removes it with None; every player list entry of
  theirs carries it from then on, but nothing is sent. False on a client and past `GameProfile`'s limits (64, 32767
  and 1024 characters). Where the hub is out of reach (inside a player's own hook) the change applies at the tick's
  end. The second shows the player again, at the end of their level's pass: everyone who sees them gets their list
  entry anew and their entity again, and the player is respawned in place into the level they are in, all state kept,
  so their own client draws the new skin too (a dead player only to the others). Both work from any server hook,
  `native_server_tick` included; true does not mean the player was found. Bundles need
  `"hooks": 497`.

- Helpers **`native_player_textures(uuid)`** (`host.player_textures`) and **`native_player_head(uuid)`**
  (`host.player_head`): the `textures` property of a player who is on, its value and signature as their player list
  entry carries it (Paper's `Player.getPlayerProfile`), and a `minecraft:player_head` stack of any player (Bukkit's
  `SkullMeta.setOwningPlayer`) whose `minecraft:profile` holds their id, plus their name and skin when they are on,
  so it shows the skin at once (a profile of an id alone the client resolves itself). Where the hub is out of reach
  (inside a player's own hook) the textures are None and the head carries the id only. Bundles need
  `"hooks": 494`.

- Helpers **`native_profile_by_name(name)`** (`host.profile_by_name`) and **`native_profile_by_uuid(uuid)`**
  (`host.profile_by_uuid`): a player the server knows, by name in any case (their id and name as written) or by id
  (their name): one who is on, else one in a dedicated server's profile cache, `usercache.json` (vanilla's
  `GameProfileCache`, Bukkit's `getOfflinePlayer` without its network lookup). They ask no profile service and make
  up no offline player; None when nobody is found. Bundles need `"hooks": 496`.

#### Server hooks for plugin ports: player state

Helpers that read a player's state and write it back, what a mod keeping players in step across servers (a HuskSync
port) needs. They reach the server's own players, as `native_server_players` lists them: inside a player's turn (their
hook, their command) only that player, outside one (`native_server_tick`, `native_menu_clicked`) everyone on. A write
goes through the player's turn as `/effect` and `/attribute` do, and their client follows with the tick's sync. Nothing
calls them without a mod.

- Helper **`native_player_compound(uuid)`**: the player's compound as the server would save it now
  (`players/data/<uuid>.dat`, vanilla's `ServerPlayer.saveWithoutId`), with every key Horizon keeps without modelling
  it. None inside a player's turn (their live state is out of their record) and for a player not on.
  Bundles need `"hooks": 478`.
- Helpers **`native_player_food(uuid)`** and **`native_set_player_status(uuid, health, food, xp)`**: the food level,
  saturation and exhaustion (`FoodData`); and health, food `(level, saturation, exhaustion)` and experience
  `(level, progress, total)` set, each when given, as Bukkit's `setHealth`, `setFoodLevel`, `setSaturation`,
  `setExhaustion`, `setLevel`, `setExp` and `setTotalExperience`. Health is capped at the player's maximum; 0 or less
  kills them as `/kill` does, with the death message "<player> died" (Bukkit's `setHealth(0)`). The mod's food is not
  heard by `native_food_level_changed`, except inside the player's own turn. False for a player not reached, a dead one,
  a value that is not a number, a negative level or total, or a progress outside 0 to 1.
  Bundles need `"hooks": 492`.
- Helpers **`native_effects(uuid)`**, **`native_add_effect(uuid, effect)`**, **`native_remove_effect(uuid, id)`** and
  **`native_set_effects(uuid, effects)`**: a player's effects as `NativeEffect { id, amplifier, duration, ambient,
  particles }` (`MobEffectInstance`; duration -1 is infinite); one given as `LivingEntity.addEffect` gives it (Bukkit's
  `addPotionEffect`: a weaker or shorter one of the kind waits under it), one taken off as `/effect clear` takes it, or
  all replaced (a saved player's potion effects restored). The client gets `UpdateMobEffect` with the ambient and
  particle flags. Horizon keeps no hidden-icon flag on an effect, so the icon always shows. Unknown effects and a
  duration of 0 or below -1 are refused. Bundles need `"hooks": 479`.
- Helpers **`native_attribute(uuid, attribute)`**, **`native_set_attribute_base(uuid, attribute, base)`** and
  **`native_set_attributes(uuid, saved)`**: an attribute's base value and its value with every modifier (Bukkit's
  `AttributeInstance.getBaseValue` and `getValue`); its base set as `/attribute .. base set` does; a saved
  `attributes` list (the compound's) loaded as `AttributeMap.load` reads a player's file. A lower `max_health` caps the
  health, and the client gets `UpdateAttributes`. An id without a namespace is `minecraft:`'s.
  Bundles need `"hooks": 474`.
- Helpers **`native_player_persistent_data(uuid)`** and **`native_set_player_persistent_data(uuid, data)`**: the
  compound mods keep on a player (NeoForge's `Entity.getPersistentData()`, Bukkit's `PersistentDataContainer`), saved in
  their file as `NeoForgeData`; an empty compound when there is none, and
  setting an empty one takes it away. Mods share it: keep your keys under your namespace.
  Bundles need `"hooks": 486`.

#### Server hooks for plugin ports: Moving players and vehicles

Helpers that move a server player or a vehicle from a mod, so a port no longer runs `execute in .. run tp` from the
console (which waits for the next tick and does nothing inside a player's hook).

- Helper **`native_teleport_player(uuid, dimension, pos, yaw, pitch)`** (`host.teleport_player`): teleports a server
  player to `pos` in `dimension` (`minecraft:the_nether`), facing `yaw` and `pitch`, as `/tp` does (Bukkit's
  `Player.teleport`): they leave their vehicle, lose their motion and fall, and their client is told. Callable from any
  server hook. Within their level it is done at once when they are the acting player or nobody's state is swapped in
  (a tick hook); inside another player's hook it is done after the players' packets of the server tick (the same tick
  when the hook heard a packet). Into another level it happens when the tick's level teleports run, as a command's.
  True when done or queued; false for a player not on the server or dead (where that can be seen), another dimension
  than the three, a position outside `/tp`'s bounds, an angle that is not finite, and on a client. Server-side.
  Bundles need `"hooks": 511`.
- Helper **`native_teleport_vehicle(dimension, id, pos)`** (`host.teleport_vehicle`): teleports the boat, minecart or
  mob a rider sits on (a horse, a pig) with entity id `id`, of the level in place, to `pos` with its riders, as `/tp`
  does a vehicle (Bukkit's `Vehicle.teleport`): they stay seated and move by the same offset, and a server player riding
  it is told, as their client drives the vehicle itself. Only where nobody's state is swapped in (a tick hook); inside
  `native_vehicle_moved_v2` change `to` instead. False inside a player's hook, for no such vehicle here, another
  dimension, a position outside `/tp`'s bounds, and on a client. Server-side. Bundles need
  `"hooks": 512`.
- **`native_vehicle_moved_v2(player, dimension, vehicle, kind, from, to, cancel)`**: an additive version of
  `native_vehicle_moved`, heard right after v1 at the same times and with the same state swapped in, and also for a mob
  a server player rides (a horse, a pig, a strider), which v1 does not hear. `vehicle` is the entity id and `dimension`
  its level. Set `cancel` and the vehicle is put back at `from` with its riders; change `to` and it is put there instead
  (not checked against the blocks; a value that is not finite is ignored), as Bukkit's `VehicleMoveEvent` with
  `Vehicle.teleport`. Either way its riders' clients are told, and the next move is heard from there. Mods are asked in
  order and share `to` and `cancel`. Server-side. Bundles need `"hooks": 514`.
- Helpers **`native_mount(entity, vehicle)`**, **`native_dismount(entity)`** and **`native_eject_passengers(vehicle)`**
  (`host.mount`, `host.dismount`, `host.eject_passengers`), by entity id in the level in place: seat a server player or
  a mob on a boat, minecart or mob, as `/ride <entity> mount <vehicle>` does with its checks (neither rides already, no
  loop, a free seat, minecarts carry players only); take one off what it rides, as `/ride <entity> dismount`, landing
  beside the vehicle; or take every server player and mob off a vehicle (Bukkit's `addPassenger`, `leaveVehicle`,
  `eject`). `native_entity_mount` may refuse each. Inside a player's hook only the acting player and mobs are reached.
  True when it rides, got off, or one got off; false otherwise and on a client. Server-side. Bundles need
  `"hooks": 472`.
- Helpers **`native_vehicle_of(entity)`** and **`native_passengers_of(entity)`** (`host.vehicle_of`, `host.passengers_of`),
  by entity id in the level in place: the boat, minecart or mob a server player or mob sits on (Bukkit's `getVehicle`), and
  the server players and mobs riding a boat, minecart or mob, in vanilla's `getPassengers` order (the controlling
  passenger first; players, then mobs). Read-only: they change nothing. None or empty when nothing rides, for an entity
  not reached (inside another player's hook only the acting player and mobs are), and on a client. Server-side. Bundles
  need `"hooks": 642`.
- Helper **`native_set_flight(uuid, allow, flying)`** (`host.set_flight`): lets a server player fly in any game mode
  (`allow`) or not, and sets whether they fly now (`flying`), as Bukkit's `Player.setAllowFlight` and `setFlying`
  (`Abilities.mayfly` and `flying`); their client is told, and the server's flying check no longer kicks them. A player
  who may not fly does not fly; creative and spectator players fly whatever `allow` is. Kept until their game mode
  changes, they respawn or they leave (vanilla sets it for the mode then); not saved. Done at once where nobody's state
  is swapped in, else after the players' packets of the server tick. True when done or queued; false for a player not
  on the server (where that can be seen) and on a client. Server-side. Bundles need `"hooks": 507`.
- Helper **`native_respawn_player(uuid)`** (`host.respawn_player`): respawns a dead server player as their client's
  Respawn click does (`ClientCommand` `PERFORM_RESPAWN`; Spigot's `player.spigot().respawn()`): at their bed, anchor or
  the world spawn, with the respawn hooks, without waiting for the click. Done after the players' packets of the server
  tick (the same tick from a packet's hook or the tick's start, else the next). True when queued; false for a player
  seen not on the server or alive (inside another player's hook nobody else can be seen: it is queued and does nothing
  for a living player), and on a client. Server-side. Bundles need `"hooks": 500`.

#### Server hooks for plugin ports: Reading players

What a server mod reads about a player besides their position and health. These helpers find the acting player
(whose state a player's hook swaps in) and every player the hub holds; inside another player's hook, where the hub
is out of reach, only the acting player is found and the others read as None. Server-side: on a client they give
None.

- **`NativeServerPlayer`**, as `native_server_players()` and `native_acting_player()` give it, also carries
  `latency` (the round-trip in ms, as the player list shows it), `protocol`, `xp_level` and `xp_progress`,
  `on_ground`, the abilities the client is told (`flying`, `may_fly`, `invulnerable`, `walk_speed`, `fly_speed`),
  `sneaking`, `eye_height` in the current pose (1.62 standing, 1.27 crouching), `selected_slot`, and the `nickname`
  and `hidden` level server mods set (Bukkit's `getPing`, `isOnGround`, `getAllowFlight`, `isFlying`,
  `isSneaking`, `getEyeHeight`, `getDisplayName`). For the acting player, `latency`, `nickname`, `hidden` and
  `address` read 0 or None while the hub is out of reach. Bundles need `"hooks": 506`.
- **`NativeServerPlayer.fake`** is true for a fake player, a mod's bot with no client (`native_set_player_fake`), so a
  mod can leave out what only a client would show. Bundles need `"hooks": 604`.
- Helper **`native_player_client(uuid) -> Option<NativeClientInfo>`** (`host.player_client`): the client's brand,
  language, the view distance it asks for, chat visibility, main hand and particle setting, the chunks the server
  sent it and its wire bytes in and out (Paper's `getClientBrandName`, `locale`, `getClientViewDistance`). It
  reads the hub, so it is None inside a player's hook while the hub is out of reach, the acting player's too.
  Bundles need `"hooks": 475`.
- Helpers **`native_player_stats(uuid)`**, **`native_player_stat(uuid, stat_type, key)`** and
  **`native_set_player_stats(uuid, stats)`** (`host.player_stats`, ...): the player's statistics as vanilla's
  `players/stats/<uuid>.json` holds them (`{"stats": {"minecraft:custom": {"minecraft:jump": 3}}, "DataVersion":
  ..}`), one counter of them (0 when never counted; `custom`, `jump` mean `minecraft:custom`,
  `minecraft:jump`), and their replacement from that JSON (a HuskSync restore). Replaced statistics are saved with
  the player and all go to the client at its next request; scoreboard objectives that count one are not told.
  **`native_player_advancements(uuid)`** and **`native_set_player_advancements(uuid, advancements)`** do the same
  for `players/advancements/<uuid>.json` (each advancement's criteria with their dates, and `done`); advancements
  this server lacks are dropped as when the file loads, and the client gets every advancement's progress again
  with the player's next tick (one newly done shows its toast, as a grant would, with no rewards). The setters say whether the player was found. Bundles need
  `"hooks": 489`.
- Helpers **`native_max_players()`** (`max-players`, as `/list` shows it; None while the hub is out of reach),
  **`native_server_dir()`** (a dedicated server's folder, absolute; None elsewhere) and **`native_log_file()`** (the
  `logs/latest.log` this run writes, absolute; None when it logs to the terminal only), as Bukkit's
  `Server.getMaxPlayers` and a plugin's server folder. Bundles need `"hooks": 505`.
- Helpers **`native_held_stack(uuid)`** and **`native_player_equipment(uuid, slot)`** (`host.held_stack`,
  `host.player_equipment`): the whole stack, components included, in a player's main hand or `EquipmentSlot`, from
  any server hook and for any player, not only the acting one (Bukkit's `getEquipment().getItemInMainHand()` and
  `getItem(slot)`). None for an empty slot and for `Body` and `Saddle`. Bundles need `"hooks": 480`.

#### Server hooks for plugin ports: per-player feedback

What Bukkit's and Adventure's `Player` methods send one player, which plugin ports built as raw packets through
`native_send` before: a sound, particles, a title, boss bars and the compass target. Only
that player is sent anything; the world, its `/bossbar` bars and the other players are untouched. NeoForge has no
per-player helpers for these (a NeoForge mod sends `ServerPlayer.connection.send` itself), so none is named.

- Helper **`native_play_sound_to(uuid, sound, category, at, volume, pitch)`** (`host.play_sound_to`): a
  `ClientboundSoundPacket` to that player alone, as Bukkit's `Player.playSound`, at `at` in their level or, with None,
  where they stand when it leaves. `category` is a `SoundSource` name (`master`, `music`, `record`, `weather`, `block`,
  `hostile`, `neutral`, `player`, `ambient`, `voice`, `ui`). As for `GameLevel::native_play_sound`, an unknown
  `minecraft:` event is dropped and another namespace's event goes by name (a client hears it if its resources define
  it). No range applies: their client fades it with distance as any sound. It leaves at the tick's end, so it works
  from any server hook, a player's own included; true means queued. False on a client, for a bad name, category,
  position, volume or pitch, and for a player who isn't on (inside another player's hook, where the players are out of
  reach, that player then gets nothing). Bundles need `"hooks": 540`.
- Helper **`native_send_particles_to(uuid, options, state, at, spread, speed, count, force)`**
  (`host.send_particles_to`): particles for that player alone, as vanilla's `ServerLevel.sendParticles(player, ...)`
  (`/particle ... normal|force <player>`) and Bukkit's `Player.spawnParticle`: sent when `at` is within 32 blocks of
  the centre of their block, 512 with `force`, which also sets the packet's `overrideLimiter` (the client draws them
  whatever its Particles option says). Options and their checks are `GameLevel::native_send_particles`'s: block
  particles need `state`, item, trail and vibration particles aren't sent, a mod's own type
  (`ParticleOptions::named`) reaches only a Horizon client. Queued as `native_play_sound_to`, with the same answers.
  Bundles need `"hooks": 547`.
- Helper **`native_title(uuid, title, subtitle, fade_in, stay, fade_out)`** (`host.title`): a title and subtitle in
  the middle of that player's screen, as `/title` and Adventure's `showTitle`: the times (in ticks; vanilla's default
  is 10, 70, 20), the subtitle and the title, in that order, replacing a title shown before; an empty subtitle shows
  none. Sent at once, as `native_tell`: also inside a player's hook for the acting player, whose packets go after the
  hook. False on a client, for a negative time, and for a player who isn't on or can't be reached from here (another
  player inside a player's hook). Bundles need `"hooks": 555`.
- Helper **`native_boss_bar(uuid, id, bar)`** (`host.boss_bar`): shows that player the boss bar `id` (a UUID the mod
  picks; a random one keeps clear of the world's) as a `NativeBossBar` (`name`, `progress` 0 to 1, `color` and
  `overlay` as `BossBarColor` and `BossBarOverlay` ordinals, the `darken`, `music` and `fog` flags), changes it, or
  takes it away with None, as Bukkit's `BossBar.addPlayer`/`removePlayer`. The game's boss bar update sends it with the
  world's bars (`ServerBossEvent`'s packets: a change is one packet, a new bar is added under the mod's earlier ones,
  after the `/bossbar` bars and before a boss's). By UUID: kept for a player who is off, who sees it again when they
  come back, until a mod takes it away or the server stops. Works from any server hook. False on a client, and for a
  progress outside 0 to 1 or a colour or overlay `BossEvent` hasn't. Bundles need `"hooks": 517`.
- Helper **`native_set_compass_target(uuid, target)`** (`host.set_compass_target`): points that player's compass at
  `Some((dimension, pos))`, or at the world spawn again with None, as Bukkit's `Player.setCompassTarget`. The
  `ClientboundSetDefaultSpawnPositionPacket` they are sent carries the target in place of the world spawn, now and
  every time the server sends the world spawn again (a respawn, a trip to another level), until a mod sets it again or
  they leave. A compass points there only in that dimension (elsewhere it spins, as for the world spawn; a Horizon client so far
  keeps only an Overworld target, `remote.rs`); a lodestone
  compass and a recovery compass keep their own targets. Queued as `native_play_sound_to`, with the same answers; also
  false for a dimension that isn't a namespaced id or a position a block position can't carry. Bundles need
  `"hooks": 520`.

#### Server hooks for plugin ports: packets

What packet-level plugins (ProtocolLib, PacketEvents, TAB), plugin channel listeners, resource pack plugins and
`/kick` replacements need, which plugin ports reached with raw packets and injections before.

- **`native_server_packet(player, packet, cancel)`**: each play packet the server is about to send one guest's
  client, decoded (`ClientboundPlayPacket`), in the order they leave; a broadcast is heard once for each player it
  reaches, so a mod can answer each viewer differently (ProtocolLib's and PacketEvents' outgoing listeners; there is no
  NeoForge or Bukkit event). Change `packet` and that client gets the changed packet, encoded again; set `cancel` and
  it gets nothing. It is heard only after a mod called the helper **`native_watch_server_packets()`** (any thread,
  any time; it cannot be switched off again): until then the server neither decodes nor copies what it sends, and each
  packet costs one atomic load. Not heard: the configuration phase, the disconnect a kick sends, bytes that don't decode
  (they go out unchanged, as does every packet no mod changed), and a player who isn't a guest (single player's own). It
  runs where the packet is sent, on whichever server thread sends it, with no `self` and no player's state swapped in, once for
  each packet and player: keep it cheap. Bundles need `"hooks": 548`.
- **`native_entity_data_to(viewer, entity, values)`**: the typed shortcut for one packet, the entity data
  (`ClientboundSetEntityDataPacket`) the server is about to send the guest `viewer` for the entity with network id
  `entity`. `values` are its `(index, serializer id, value)` entries (`EntityDataValue`, `SynchedEntityData.DataValue`)
  in the order sent; change them and only that viewer gets the change, so one viewer can see a player glow (OR `0x40`
  into the `Byte` of entry 0) or another custom name, while the server and every other viewer keep the real values.
  Heard just before `native_server_packet` hears the same packet, with the changed values. Switched on by the helper
  **`native_watch_entity_data()`**, which decodes this packet alone (the other packets keep going out undecoded).
  Every entity's data reaches a client in this packet, a newly tracked one's first values included; an entity whose
  values are all defaults is tracked without one, as in vanilla, so there is nothing to change until a value does.
  The same threads and limits as `native_server_packet`. Bundles need `"hooks": 521`.
- **`native_player_register_channels(player, name, channels, register)`**: a guest's client registered plugin channels
  (`minecraft:register`, `register` true) or unregistered them (`minecraft:unregister`), NUL-separated names as Bukkit's
  and Fabric's clients and proxies send them; vanilla reads neither message (Bukkit's `PlayerRegisterChannelEvent` and
  `PlayerUnregisterChannelEvent`). `channels` are the names that changed, after Horizon applied them. What a client
  registered during its configuration, before it joined, is heard once as one registration right after
  `native_player_logged_in`, with the hub in place; a message in play is heard as it is handled, from a dead player too,
  with the player's state swapped in, so `native_tell` answers them, and then goes on to wherever it went before. Horizon
  keeps at most 128 names of at most 256 bytes, printable ASCII with a `:`; a name past that, or one that changes
  nothing, is not heard. The helper **`native_player_channels(player)`** lists the channels a player has, in the order
  registered (Bukkit's `Player.getListeningPluginChannels`); it reads the hub, so it is empty inside a player's own
  hook. Bundles need `"hooks": 541`.
- Helpers **`native_send_resource_pack(player, id, url, sha1, prompt, required)`** and
  **`native_pop_resource_pack(player, id)`**: a mod's own resource packs for one player's client, as Bukkit's
  `Player.addResourcePack` and `removeResourcePack(s)`. The push (`ClientboundResourcePackPushPacket`) carries the
  mod's own `id`, a UUID it picks, so it can take that pack away again and tell its answers from other packs' with
  `native_resource_pack_status_v2`; `sha1` is the pack's lowercase hex hash (empty: unchecked), `prompt` an extra line
  on the client's prompt, `required` how the prompt words it. A pack whose id the client already has is replaced.
  As in vanilla, a player who declines is kicked only when the server's own pack (`require-resource-pack`) is
  required, whatever pack they declined; anything else is the mod's to decide. `native_pop_resource_pack(player, None)`
  takes away every server pack the client has. Both send as `native_send` does (from `native_server_tick` and from the
  player's own hooks) and are false when the player isn't on, on a client, and for an empty URL, a URL past 32767
  characters or a hash past 40. Bundles need `"hooks": 545`.
- **`native_resource_pack_status_v2(player, name, pack, status)`**: an additive version of
  `native_resource_pack_status` with the answered pack's id (`ServerboundResourcePackPacket.id`, Bukkit's
  `PlayerResourcePackStatusEvent.getID`), so a mod hears its own pack's answers apart from every other pack's;
  `status` is the same `Action` number. In play it is heard just before v1, with the player's state swapped in. The
  answers to the server's own pack (`resource-pack` in `server.properties`), given during the configuration and never
  heard by v1, are heard here in order once the player has logged in, right after `native_player_logged_in`, with the
  hub in place; a player disconnected for declining a required pack never logs in. v1 keeps its signature and firing
  point. It only hears. Bundles need `"hooks": 544`.
- Helper **`native_kick(player, reason)`**: disconnects a player with `reason`, a whole `Text` in its colours and
  translations (Adventure's `Player.kick(Component)`; `/kick` takes one line of plain text), as `/kick` does: the
  client shows it, the log's "lost connection" line names it and `native_player_kicked` hears it. The kick happens at
  the tick's end, with the mod messages, so it works from any server hook, the kicked player's own included. True when
  it is queued; false on a client, in a world nobody else can join, and for a player who isn't on when the hub can tell
  (a player gone by the tick's end isn't kicked either). Bundles need `"hooks": 528`.

#### Server hooks for plugin ports: vanish and per-viewer visuals

What a vanish, a per-player hologram or a Tab list plugin needs, which plugin ports reached with raw packets or
injections before.

- **`native_living_change_target(dimension, entity, kind, target, target_mob, cancel)`**: a mob's target changed
  (NeoForge's `LivingChangeTargetEvent`, Bukkit's `EntityTargetEvent`): mob `entity` of type `kind` now targets the
  player `target` (a UUID) or the mob `target_mob`, or lets go (both None). Heard once per change, after the mob's
  tick that made it (its target goals or brain; a creative player it drops), or, when another mob's tick changed it
  (`HurtByTargetGoal`'s alert), once every mob has ticked. Set `cancel` and the mob keeps the target it had, and what
  its tick did to the new one (a melee hit, a sting, a sonic boom) is not dealt; it may pick the same target on a
  later tick, which is heard again, so a vanish mod cancels every time the target is a vanished player. A target that
  is neither a player nor a mob is not heard. No `self` (the mobs are out of the game while they tick), so a mod
  decides from its own state. Server-side. Bundles need `"hooks": 530`.
- Helper **`native_hide_entity_from(viewer, entity, hidden)`** (`host.hide_entity_from`): hides the entity with
  network id `entity` (a mob, a display, another player's `NativeServerPlayer.entity_id`) from the player `viewer`
  alone, or shows it again, as Bukkit's `Player.hideEntity`/`showEntity` (`hidePlayer` for a player). From the next
  tick the viewer's client stops tracking it, so it disappears there, and once shown it is paired again as it is then;
  a hidden player also leaves the viewer's Tab list, and the viewer doesn't read them leave. Everyone else still sees
  it; `native_hide_player` stays the way to hide a player from permission levels. Kept while the viewer is on, for that
  id: forgotten when they leave, and a player who rejoins has another id (hide them again from
  `native_player_logged_in`). Outside a player's hook; false there, when the viewer isn't on and for their own entity.
  Bundles need `"hooks": 523`.
- Helpers **`native_client_entity_spawn(viewer, kind, pos, yaw, nbt)`**, **`native_client_entity_set(viewer, id, nbt)`**,
  **`native_client_entity_move(viewer, id, pos, yaw)`**, **`native_client_entity_remove(viewer, ids)`** and
  **`native_reserve_entity_id()`** (`host.client_entity_spawn`...): an entity shown to one player's client alone, a
  hologram or a board only its owner sees (Bukkit has no API for it; plugins send the packets). Spawn sends the add and
  its synched data, encoded from `nbt` as `/summon` reads it: all of a text, item or block display's (`text`,
  `background`, `billboard`, `transformation`, `item`, `block_state`, `teleport_duration`...), and for any type
  `CustomName`, `CustomNameVisible`, `Invisible`, `Glowing`, `Silent` and `NoGravity`; other types' own data is not
  read, and a player is refused. It returns the entity's id, one no server entity takes. No level holds it: it is not
  simulated, saved, tracked or seen by anyone else, and the client forgets it when the player changes dimension or
  leaves. `set` gives it the data `nbt` gives its type, what `nbt` leaves out back at its default (pass the whole of what
  it shows); `move` teleports it (a display eases over its `teleport_duration`); `remove` takes away those of `ids` that
  are this viewer's client-side entities and leaves other ids alone. `native_reserve_entity_id()` is an id from the
  server's own counter for a mod's own packets. Outside a player's hook; None or false there, when the viewer isn't on,
  for an unknown type and for an id that isn't theirs. Bundles need `"hooks": 519`.
- Helper **`native_set_list_name(uuid, name)`** (`host.set_list_name`): styled text (colours, a rank prefix) for that
  player in everyone's Tab list in place of their nickname or name, as Bukkit's `Player.setPlayerListName`; None shows
  the nickname or name again. Only the Tab list (chat keeps `native_set_display_name`'s or the nickname). It is sent at
  once and with every entry the lists get of them later (they join, a vanish ends, `native_hide_entity_from` shows them
  again), and kept by UUID until a mod sets it again or the server stops, also if they leave and come back, so a Tab
  plugin needs no per-tick diff. Outside a player's hook; false there and when they aren't on. Bundles need
  `"hooks": 552`.
- Helper **`native_set_display_name(uuid, name)`** (`host.set_display_name`): styled text for that player in chat in
  place of their nickname or name, as Bukkit's `Player.setDisplayName`: their chat messages, `/me`, `/msg` both ways
  and the lines that they joined or left; None shows the nickname or name again. Their team's prefix, suffix and colour
  still go around it; hovering it gives the real name, which `/tell` is suggested with and which stays in the log,
  commands and selectors. The Tab list keeps `native_set_list_name`'s or the nickname. Kept by UUID until a mod sets
  it again or the server stops. Outside a player's hook; false there and when they aren't on. Bundles need
  `"hooks": 551`.
- Helper **`native_set_name_tag(uuid, tag)`** (`host.set_name_tag`): a plain name over that player's head for everyone
  else in place of theirs, a nickname above the head; None shows their name again. A client puts a player's profile
  name there, so the tag is the profile name of their list entries: 1 to 16 ASCII letters, digits or symbols, no
  spaces, no colours (a team's formatting applies only while the team lists the shown name). The other players'
  clients get their entry and entity again at once (a blink), and their Tab list shows the tag unless a list name or
  nickname covers it; their own client reads it from their next join. Chat, commands, selectors, the log and the
  server's events keep the real name. Kept by UUID until a mod sets it again or the server stops. Outside a player's
  hook; false there, when they aren't on and for a tag that isn't one. Bundles need `"hooks": 553`.

#### Server hooks for plugin ports: NPCs

Helpers for server mods that spawn and drive NPCs (an NPC library, guards that fight, a stage that places actors),
in `crates/horizon-client/src/game/native_npcs.rs`, on a server only (on a client they do nothing and the readers give
nothing). Each names its level by `dimension` (`minecraft:the_nether`) and reaches any level the server has loaded,
not only the one in place: that level is put in place for the call and the one that was is put back, as `execute in`
does, so they work from `native_server_tick` too. A level the server hasn't loaded is refused, and a level nobody is
in has only the chunks something holds loaded. While a level ticks its mobs they are out of it, so from a hook inside
that tick the other mobs of that level read as gone.

- **`native_spawn_mob(dimension, kind, pos, &NativeSpawn)`** and **`native_summon(dimension, kind, pos, nbt)`**
  ([Mobs](#mobs), [Entities](#entities)) spawn into any loaded level, as `execute in <dimension> run summon` does,
  where they were refused for any level but the one in place. Their other refusals stay. API 535.
- **`native_entity_by_uuid(uuid)`** finds the entity with that UUID in any loaded level, as Bukkit's
  `Server.getEntity(UUID)` does: `(dimension, id, kind)`, so a stage plugin that keeps its actors by UUID reaches them
  wherever they are, without `/tp`, `/data` or `/kill` by UUID on the console. A server's player is found too
  (`minecraft:player`, in its level), except while a guest's packet is handled, where the hub is taken out.
  **`native_merge_entity_nbt(dimension, id, nbt)`** is `native_entity_merge_nbt` ([Entity and level
  handles](#entity-and-level-handles)) for any loaded level, as `execute in <dimension> run data merge entity` does,
  with the same refusals (a player, a kind Horizon can't reload yet, data that changes nothing). API
  536.
- **`native_find_entity_by_uuid(uuid)`** is `native_entity_by_uuid` from a `&self` context, so a mod can make many
  lookups from a read-only hook: the same `(dimension, id, kind)`, for a server's player and for the entities of the
  level in place. A level not in place is read only by swapping it in, which needs `&mut`, so those stay with
  `native_entity_by_uuid`. None on a client.
- **`native_entity_nbt_in(dimension, id)`** is the entity's data as `native_entity_nbt` gives it, in any loaded level
  (`/execute in <dimension> run data get entity`, without `id`): what a mod reads after finding an entity by UUID in
  another level. None on a client, for a level not loaded, and for no such entity. API 639.
- **`native_level_markers(dimension)`** lists the `minecraft:marker` entities of a loaded level as `(id, position,
  data)`: the `data` compound is what `/data get entity <marker> data` shows (empty for a marker saved without data).
  It reads only the level's list of kept entities (displays, interactions, markers), not every entity, so it is the
  cheap way to find markers used as anchors, beside filtering `native_level_entities`. Empty on a client and for a
  level not loaded. Bundles need `"hooks": 632`.
- **`native_set_entity_rotation(dimension, id, yaw, pitch, head_yaw)`** turns an entity without moving it, as
  `/rotate` does (`Entity.forceSetRotation`): its yaw and pitch in degrees (pitch clamped to -90..90), with its body and
  head; `Some(head_yaw)` then turns a mob's head alone (`setYHeadRot`), so an NPC looks over its shoulder. It takes
  mobs (mannequins too), armor stands, boats, minecarts, evoker fangs, and displays and the other entities Horizon
  keeps without simulating them; a player is turned by teleporting them. A mob whose AI runs turns back by its own
  look and body control on its next tick: for one that looks around, use `native_mob_look_at` each tick, or spawn it
  with `no_ai`. False for no such entity, another kind, or an angle that isn't finite. API 537.
- **`native_mob_navigate(dimension, mob, to, reach, speed)`** walks a mob with the engine's pathfinding, as
  `PathNavigation.moveTo(x, y, z, reach, speed)` does: round walls and through doors as its own navigation allows (a
  walker's target is the ground below `to`), done within `reach` blocks, at `speed` times its movement speed. True
  when a path was found within the mob's follow range. **`native_mob_follow(dimension, mob, target, speed)`** walks it
  to where an entity of its level stands now (`moveTo(entity, speed)`; a player by their entity id): call it again as
  the target moves, as vanilla's follow and attack goals do. **`native_mob_stop(dimension, mob)`** stops the walk
  (`stop`), **`native_mob_navigating(dimension, mob)`** says whether one is in progress (`isInProgress`: false once
  it arrived, got stuck or was stopped), and **`native_mob_look_at(dimension, mob, at)`** turns its head towards a
  point at its own head speeds for its next tick (`LookControl.setLookAt`), its body following. These drive the mob
  as vanilla's goals do, so its own goals may take the navigation or the look over on a later tick: an NPC library
  asks again each tick, or gives its mobs goals of their own (`native_register_goals`). A mob with `no_ai` keeps a path
  but doesn't walk it, and doesn't turn its head (`native_set_entity_rotation` turns it). False for no such living mob
  or a value that isn't finite. API 539.
- **`native_mob_attack(dimension, mob, target)`** has a mob strike an entity with its own attack, as
  `Mob.doHurtTarget` does: its attack damage, held weapon and enchantments, knockback, the species' extras (a cave
  spider's poison, a husk's hunger) and its attack sound, dealt as any mob's hit is, so the damage hooks hear it.
  `target` is a living mob or a server's player of the same level, by entity id; the player is hit as when a mob that
  sees them strikes in its tick, whichever player the mob sees. There is no reach or sight check and no swing, as in
  vanilla, where the attack goal checks and swings first. True when the blow was dealt (whether it hurt, a shield or
  creative mode aside, is the damage hooks' to tell); false for no such living mob or target, the mob itself, and a
  player while a guest's packet is handled, where the hub is taken out. API 538.

#### Server hooks for plugin ports: damage, hands, fishing and redstone

`native_living_incoming_damage_v3(victim, victim_uuid, kind, attacker, attacker_uuid, direct, damage_type, amount,
cancel)` is `native_living_incoming_damage` with who the hit names (NeoForge's `LivingIncomingDamageEvent`, Bukkit's
`EntityDamageEvent`). It fires everywhere v1 fires (players, mobs and the dragon; melee, projectiles, the environment),
right after v1 with the amount v1 left: change `amount` or set `cancel`, which does what a v1 cancel does; a hit v1
canceled is not asked. `victim_uuid` is a player victim's UUID (a player is known from their first turn on the server;
a mob's UUID is looked up by its id). `attacker` is the causing entity where v1 has 0 for it: a mob's melee names the
mob, a player's melee the player, and a source naming no entity falls back to the synchronous attack in progress, as
`native_living_knockback_v2` does; `attacker_uuid` is a player attacker's UUID and `direct` the direct entity (the
arrow). Each is none when Horizon does not know it, and an attacker that is the victim itself is none. No `self`, like
v1: the victim is borrowed out of the game during the hit, so the entity helpers can't be called here; a mod keeps
what it needs and acts in its next tick hook. Bukkit's damage modifier breakdown (armour, blocking, resistance,
absorption as separate values) is not offered: the reductions are applied after this hook. Bundles need "hooks":
531.

`native_interact_entity_v2(entity, kind, hand, stack, cancel)` is `native_interact_entity` with the hand that clicks
(`Hand::Main` or `Hand::Off`, NeoForge's `PlayerInteractEvent.EntityInteract.getHand`). It is asked right after the
unchanged v1 hook, once per hand vanilla tries: the main hand first, the off hand only when the main hand's turn did
nothing. Setting `cancel` claims the click, as v1's `claimed` does. A mod that claims the main hand's click hears one
event per click, as Bukkit's `PlayerInteractEntityEvent` for `EquipmentSlot.HAND`. A click v1 claimed is not asked
again. Bundles need "hooks": 525.

`native_right_click_block_v2(pos, face, hand, cancel)` is `native_right_click_block` with the hand that clicks
(NeoForge's `PlayerInteractEvent.RightClickBlock.getHand`), asked right after the unchanged v1 hook, once per hand
tried. A canceled click does what a v1 cancel does (neither the block nor the item acts, the hand doesn't swing, the
other hand gets no turn); a click v1 canceled is not asked again. A protection mod refuses an off-hand flint and steel
or bone meal by its hand. Bundles need "hooks": 546.

`native_player_fish_v2(player, name, state, hand, cancel)` is `native_player_fish` with the rod's hand: the cast and the
reel-ins (states 0 and 2 to 4) name the hand the rod is used with, as Bukkit's `PlayerFishEvent.getHand`; a bite
(state 1) names the hand holding the rod, the main hand first. It is asked right after v1 with the same `cancel`; one
v1 set is not asked again. A stasis or power rod in the off hand is seen, and read from that hand's stack. Bundles
need "hooks": 542.

`native_fishing_hook_landed(player, hook, dimension, block, face)` hears a server player's fishing bobber (`hook`, its
entity id) touching a block, as Bukkit's `ProjectileHitEvent` for a `FishHook`: once each tick the flying bobber meets
a block on its way (`face` the side it meets), and once each tick it lies on one (on the ground as the tick begins or
ends: the block under its centre, `face` `[0, 1, 0]`). It is heard at the end of the bobber's tick with the bobber in
place, so a mod that powers the button or plate it rests on can also take it away with `native_remove_entity`. A
bobber in water, one hooked to an entity and a LAN host's own bobber are not heard. It only hears: the bobber moves as
it would. The player's state is swapped in, the other players are out of reach. Bundles need "hooks":
522.

`native_fishing_medium(dimension, hook, position, owner, medium)` lets a bobber fish outside water: lava, the void
below a skyblock island, a cloud. Each tick a bobber that is flying or floating, not in water and not hooked to an
entity asks the mods whether its spot (`position`, its feet) is a fishing medium; `owner` is where the player who cast
it stands. The first mod to answer `Some(NativeFishingMedium)` makes it one, for this tick:

- `surface`: the height it floats at. In the block of the surface or below, the bobber is in the medium as it would
  be in a water block: a flying bobber stops there, then bobs on the surface, waits for a fish, bites and is reeled in
  as in still water. Above it, it falls as usual.
- `loot_table`: what a catch from there rolls, with the fishing context vanilla's table gets (luck, the open-water
  flag, which is true in a medium). None rolls `minecraft:gameplay/fishing`. The bobber keeps the last medium's table
  until it lands in water.
- `lure_ticks`: the wait before a fish comes, drawn from `min..=max` and less the rod's Lure (vanilla draws 100 to 600
  ticks). None keeps vanilla's.

It is asked on the server, with the owner's state swapped in, and on every client that draws the bobber (`owner` None
for another player's whom the client doesn't see), so that both move it alike, as vanilla ticks the hook on both
sides: answer from the world and `owner` alone, the same on both. A mod that isn't on a client leaves it predicting a
fall that the server's updates pull back. `horizon-ctl status` shows the hook's `loot_table`. Water is never asked
about, and no answer keeps vanilla. Bundles need "hooks": 578.

`native_remove_entity(dimension, id)` also removes projectiles (arrows, tridents, snowballs, eggs, ender pearls,
fireballs, wind charges, llama spit, shulker bullets, firework rockets) as `/kill` does, and a server player's fishing
bobber, as `Entity.discard` (Bukkit's `Entity.remove`): an arrow cleanup, or a power rod's bobber taken once its
button is held. A bobber can be removed for the player in place (inside their hooks, `native_fishing_hook_landed`
included), and outside a guest's turn for any player of the level; it ends as a bobber that goes by itself, and the
rod takes no wear. False for another dimension or no such entity. Bundles need "hooks": 543.

`native_block_redstone(dimension, pos, old, new)` (a `GameLevel` hook) is asked when a button, lever or pressure plate
is about to change its signal from `old` to `new`, 0 to 15, as Bukkit's `BlockRedstoneEvent`: a button pressed by a
hand or an arrow or let go by its release tick, a lever pulled, a plate's weight checked. Change `new` and the
component takes that signal (clamped to 0 to 15; an on-or-off component is on for anything above 0). One equal to
`old` keeps it as it is, without a sound: a held button's release is scheduled again and asked again, a plate held on
is checked again, a lever stays. A mod holds a power rod's button or plate on with no 1-tick gap. Not heard yet:
redstone wire, torches, repeaters, comparators, observers, detector rails, tripwire hooks, daylight detectors and the
other sources. Server-side. Bundles need "hooks": 516.

`native_hurt_entity(dimension, target, amount, damage_type, attacker)` hurts a mob or a player of the level `dimension`
by `amount` of `damage_type` (`minecraft:mob_attack`), from `attacker` (a mob's or a player's entity id, the causing
and direct entity, as vanilla's `damageSources().mobAttack(attacker)`; none for no entity). It goes the way
`/damage <target> <amount> <damage_type> by <attacker>` goes: armour, effects, invulnerability time, knockback away from
the attacker, death and its message, and the damage hooks hear it. It returns whether the target was hurt: false for
an invulnerable, dead or creative target, another dimension, an entity that is not a mob or a player, an attacker that
isn't there, or an amount that is not a finite number. An NPC's own hit needs no `damage` command and no UUID lookup.
Bundles need "hooks": 524.


#### Server hooks for plugin ports: Menus, items and maps

- Helper `native_item_from_snbt(snbt) -> Option<ItemStack>`: the stack an SNBT item compound stands for, as `/give`
  and the data packs write one (`{id:"minecraft:diamond_sword",count:1,components:{"minecraft:damage":5}}`), read as
  vanilla's `TagParser` and `ItemStack.CODEC` do (Bukkit's `ItemFactory.createItemStack`). None when the text is not
  one compound (whitespace around it aside), for air, and for an item the world does not have. `ItemStack::from_nbt`
  reads a stack from a compound without asking the registry, and `mod_api::parse_snbt` is the SNBT reader itself (the
  tag and the bytes it read), so a mod that keeps items as text needs neither the game's internal `item_data` module
  nor a `summon` command to make them. Bundles need `"hooks": 527`.
- Helpers `native_new_map(dimension, colors) -> Option<i32>` and `native_paint_map(map_id, colors) -> bool`: maps a
  server mod draws that the world keeps (Bukkit's `Server.createMap` and a `MapCanvas` drawn for everyone). `colors`
  is the 128 x 128 picture, row by row, each byte a `MapColor` id times 4 plus its shade, as `native_send_map` takes.
  `native_new_map` makes a locked map (as a cartography table locks one: no exploring draws over it, no player
  markers) of the level `dimension`, scale 0, under the world's next map id, so it never meets an id the game hands
  out; give a `minecraft:filled_map` with that `minecraft:map_id`. `native_paint_map` replaces the whole picture of a
  map the world made or saved (false otherwise); an unlocked map is still drawn over where its holders explore. Both
  are saved with the world's maps (`data/minecraft/maps/<id>.dat`) and sent like vanilla map data to whoever carries
  the map or sees it framed, players who join later included, so the mod never resends them. None and false on a
  client of a server. Bundles need `"hooks": 534`.
- Helpers `native_map_colors(map_id) -> Option<Vec<u8>>` and `native_map_meta(map_id) -> Option<(u8, String, bool)>`: the
  picture (16384 bytes, row by row, as `native_paint_map` takes it) and the scale, dimension and locked flag of a map the
  world keeps. A map is read from its loaded copy or its saved file, which is not loaded by the read. None for a map the
  world never made or saved, and on a client of a server. No hook is needed.
- Helper `native_open_menu_with_input(uuid, rows, title, items, input) -> Option<i32>`: a server menu (see "Server
  menus") whose slots in the range `input` hold items as a chest's do (Bukkit's custom inventory with storage slots):
  the player puts stacks in and takes them out by vanilla's click rules (pickup, split, shift-click from their
  inventory, number keys, drags, double-click), while the menu's other slots stay the mod's to show and their clicks
  still reach `native_menu_clicked`, moving nothing. `items` fills both; `native_set_menu_items` changes only the shown
  slots. None for a player not here, or `input` empty or past the rows' slots. Helpers `native_menu_contents(uuid)`
  (every slot of the player's open server menu, input slots included; None while none is open) and
  `native_menu_cursor(uuid)` (the stack on the cursor; inside `native_menu_clicked`, the one the player held as they
  made that click, so a kit editor can copy it into a slot).
- **`native_menu_input_closed(uuid, window, slots)`**: a menu with input slots closed, for any reason (the player, the
  server, a mod, another menu, death, leaving), heard before `native_menu_closed` for the same window. `slots` are
  its input slots as the player left them: take what the mod keeps, and what stays goes back to the player as
  vanilla's `AbstractContainerMenu.clearContainer` gives a crafting grid back (into the inventory, the rest dropped;
  all dropped at them when they left), so no item is lost or made. It runs where the menu closed, so the hub may be
  out of the game's hands and a player's state swapped in: answer through `slots`. Bundles need
  `"hooks": 533`.
- Helper `native_open_inventory_menu(viewer, rows, title, (target, kind), layout) -> Option<i32>`: another player's
  inventory (`Player`, 41 slots in `native_inventory`'s numbering) or ender chest (`Ender`, 27) in a server menu, for
  an operator's `/invsee` (Bukkit's `openInventory(target.getInventory())`). Menu slot `k` shows the target's slot
  `layout[k]`; a `None` (or a slot past the layout) stays the mod's to show with `native_set_menu_items`, its clicks
  heard by `native_menu_clicked`. Clicks on the shown slots move the target's own items by vanilla's click rules, as in
  a chest (pickup, split, shift-click, number keys, drags, double-click, throw), so a split stack really splits; an
  armour or off-hand slot takes any stack, as that Bukkit view does. A target on the server is read before each click
  and each tick and changed in the same step, so their own screen follows with the tick's sync and nothing is copied
  or lost; a target away is their file (`players/data/<uuid>.dat`), read again before each click and written after one
  that changes it (a click whose write fails is taken back whole). The menu closes once the target leaves, dies or,
  away, comes back. None for a viewer not here, the target being the viewer, logging in, dead or never seen here, a
  layout longer than the rows' slots, a slot past the inventory or shown twice, and inside a player's turn. A mixin
  helper, not on `Host` (it takes `NativeInventoryKind`). Bundles need `"hooks": 526`.

#### Server hooks for plugin ports: world generation

For a mod that generates terrain of its own (a custom world generator, a far lands border, a void world with climate
biomes) in some chunks or all of them, as Bukkit's `ChunkGenerator` does.

- **`native_set_biomes(dimension, entries)`** (`native_biome_colors/native_biome_edit.rs`): sets biome cells in the loaded chunks of the
  level being ticked, as `/fillbiome` does. `entries` is a list of `([qx, qy, qz], biome id)`: a cell is 4x4x4 blocks
  and its position is the block coordinate shifted right by 2 (`qy` counts from world height 0, so it may be
  negative). The columns changed are saved and their biomes sent to the players who have them
  (`ChunkMap.resendBiomesForChunks`). Returns `Ok(cells changed)`: a cell that already holds the biome, a cell in a
  chunk not loaded and a cell outside the column's height are left out. Returns `Err` with nothing changed for a
  `dimension` that is not the level being ticked, a client of a server, or a biome id the level doesn't have
  (`minecraft:plains`, or a data pack's own). API 624.
- **`native_world_seed()`** (`native_world_seed.rs`): the world's seed as `/seed` prints it (`WorldOptions.seed`,
  Bukkit's `World.getSeed`), the one every level generates from, for a preview that draws the terrain a chunk will get.
  None off a server: a client only knows a hash of its server's seed. API 556.
- **`native_chunk_generate(dimension, chunk, seed, flat, chunk_out, generate)`**
  (`horizon_world::worldgen::native_generate`): chunk `[x, z]` of the level `dimension` is about to get its terrain,
  after its biomes and structure starts and before the noise (Bukkit's `ChunkGenerator.generateNoise`, vanilla's
  `NOISE` status); `flat` says the level is superflat. To build it, set its blocks with `chunk_out.set_block(x, y, z,
  state)` (all air until then; `chunk_out.registry()` names the states) and set `generate` to `NativeGenerate::Replaced
  { surface, carvers, structures, decorations }`: vanilla then adds only what the flags ask for, the dimension's surface
  rule, its carvers (on a superflat level too), the structures, the biomes' features (Bukkit's `shouldGenerateSurface`,
  `shouldGenerateCaves`, `shouldGenerateStructures`, `shouldGenerateDecorations`). Left at `Vanilla` with no block set,
  the chunk is vanilla's, byte for byte; `chunk_out.set_biomes(pick)` (what decoration, the saved chunk and the players
  see; the surface rule and structure placement keep reading the biome source) and `chunk_out.clear_structures()` work
  with either answer. Mods are asked in turn. It runs on the generation workers, for several chunks at once on several
  threads and never on the server thread, so no `Game` helper works there; a chunk can be asked again (a proto chunk
  saved unfinished, the neighbours of a finished one, the far terrain's decoration), so answer the same each time. Also
  where single player and the far terrain generate. API 518.
- **`native_lod_column(dimension, column, seed, flat, column_out)`** (`horizon_world::worldgen::native_generate`): the
  far terrain samples block column `[x, z]` where no chunk is generated yet, from vanilla's noise and surface rule
  (`GeneratorWorker::lod_grid`, `surface_sample`). A mod whose `native_chunk_generate` builds that chunk calls
  `column_out.replace(spans, biome)` with the column's blocks, runs `(lowest y, highest y + 1, state)` in any order, and
  the biome its top shows (None keeps vanilla's): the far terrain then draws them, its top, the water right over it and
  the spans a cliff shows, until the chunk itself is generated. Left alone, the column is vanilla's. It is asked for each
  far column, many per tile, on the far terrain's workers, several at once: answer from the terrain function alone,
  without building the chunk. Like vanilla's, a far column shows no carver's caves. API 532.

#### Server hooks for plugin ports: diagnostics

What a diagnostics or profiler port (Bukkit's `getTPS`, spark, PacketEvents' counters) reads. Each costs nothing
while no mod asks for it.

- **`native_tick_times()`** and **`native_tps()`**: the last 100 server tick durations in nanoseconds, laid out as
  vanilla's `MinecraftServer.getTickTimesNanos` (the tick numbered `n` at index `n % 100`, 0 where none was recorded
  yet; sort a copy for percentiles, `native_tick_stats` has the average), and the ticks a second over the last one,
  five and fifteen minutes (Bukkit's `Server.getTPS`), measured between the ends of the recorded ticks and averaged
  once a second as a load average is. They start at 20; an idle server reads 20, a sprint more and a server that
  falls behind less. Bundles need `"hooks": 554`.
- **`native_level_stats(dimension)`**, **`native_level_entities(dimension)`** and
  **`native_level_block_entities(dimension)`**: what an open level of a server holds (`minecraft:the_nether`; None or
  empty on a client and for a level that isn't open), as Bukkit's `World.getLoadedChunks`, `getForceLoadedChunks`,
  `getEntities` and `Chunk.getTileEntities` give it. `NativeLevelStats` counts the loaded chunks, the forced ones
  (`/forceload` and `native_force_chunk`), the block entities and those that tick, and the entities (players
  included) and block entities by type, the most first. The entity list gives each entity's id (what events and
  `native_entity` take), type and position; the block entity list each one's position and type: the vanilla block
  entity type of its block (`minecraft:barrel`, `minecraft:mob_spawner`), else the block's name. A level not in place
  is swapped in for the count and back, as `/execute in` does. The players are the hub's, so none are counted inside
  a player's own hook, where the hub is taken out. Each walks every entity or block entity: call them once a second
  or less, not every tick. Bundles need `"hooks": 529`.
- **`native_server_packets(player, packets, outbound)`** and **`native_count_packets(on)`**: the play packets each
  player's connection carries, by type, as PacketEvents' send and receive events count them. Nothing is kept until a
  mod calls `native_count_packets(true)` (one switch for the server, shared by every mod; off drops what is kept, and
  a packet costs one relaxed load); while on, once a server tick after `ServerTickEnd` each player's packets since the
  last tick arrive, those sent (`outbound`) and those received, each as (packet id, body bytes) in order. The id is
  `horizon_protocol::ids::play`'s as the server sent or read it (before an older client's renumbering; keep-alives
  and the disconnect included), the bytes are id and payload before compression and encryption. A tick stalled for
  long keeps at most 65536 packets each way. Bundles need `"hooks": 549`.
- **`native_server_tick_timed(wall_ms, cpu_ms, spans)`**, **`native_spans(on)`**, **`native_mob_cost_sampling(on)`**
  and **`native_take_mob_costs()`**: a profiler by tick step instead of by stack, as spark's. After each pass of the
  server tick (paused ones too) while its spans are recorded, the hook gets the pass's wall time, the server thread's
  CPU time in it (None where the system doesn't tell) and its spans (`TickSpan`: name, wall and CPU milliseconds,
  depth; summed by name in the order first seen, `tick_mobs` inside `tick`). A dedicated server records them anyway
  (its slow ring), so there the hook fires every tick; elsewhere it fires while `native_spans(true)`,
  `HORIZON_SPIKES` or a performance report wants them: one switch shared by all, on while anyone wants it, which costs
  two clock reads a step on every thread. `native_mob_cost_sampling(true)` times every mob's own tick (riders inside)
  by level and type until it is turned off, apart from `horizon-report`'s sampling, so both can run;
  `native_take_mob_costs()` takes the sums (`NativeMobCost`), costliest first. For a lighter profile, sample one tick
  in N: turn it on in `ServerTickStart` and off after. Call both from server hooks: the sampling belongs to the
  server thread. Bundles need `"hooks": 550`.

- **`native_perform_command(player, command, ran)`**: a player typed a registered command (without its slash) and
  their level allows it, whether the world allows commands or not. The mod parses the line and sets `ran`. The
  player's state is swapped in for the call, as for their other actions, so `native_player` reads them; a
  registered command no mod takes answers as vanilla's unknown command.
- **`native_player_swap_hands(player, name, cancel)`**: a player pressed the swap-hands key (not a spectator), heard
  before the main and off hand items swap. Set `cancel` and they stay where they are; the player's client, which swapped
  them at once, gets its slots back. The player's state is swapped in, so `native_acting_player`, `native_tell` and the item
  helpers reach them. Not heard: the same swap from an open inventory (hovering a slot and pressing the key) and the host of a LAN world.
- Helpers: `native_server_players()` (uuid, name, level, dimension, position and game mode of each player on);
  `native_tell(uuid, text)`, a system line to one player, also the acting player inside their hook;
  `native_broadcast(text)`, to every player and the server's
  log; `native_server_event(name, uuid, message)`, an `announce` event on a dedicated server's event stream, which the Discord
  bridge relays when `[discord] events` lists `announce`; `native_feedback(text)`, the command's answer to the player whose command it is; `native_world_dir()`, the
  open world's folder, where a server mod keeps its data in a folder of its own so the world's backups carry it.
- **`native_item_damaged(player, name, item, amount, cancel)`**: an item of a player is about to lose durability, on a
  server (`ItemStack.hurtAndBreak`: tools, armour, shield, rod, elytra), after Unbreaking has had its say. Change
  `amount` (0 leaves the item whole) or set `cancel` for no wear. Not heard for a creative player or when Unbreaking
  saved the item, nor for a mob's armour, a dispenser's tool or the host of a LAN world. The player's state is swapped
  in, so `native_acting_player`, `native_tell` and the item helpers reach them.
- Items (API 9), of the acting player (inside `native_perform_command`, whose player is swapped in): `native_held()`
  (the main hand's item, count, and whether it is plain), `native_count_items(item)`, `native_carried_items()`,
  `native_take_items(item, count)` (the main hand first, then as `Inventory.add` orders slots),
  `native_room_for(item)` and `native_give_items(item, count)` (only where it fits: nothing is dropped). They look at
  the 36 inventory slots and the off hand, never the armour, and only at **plain** stacks: no damage and no
  component (name, enchantment...), so a shop never takes a worn or enchanted item for a new one. Each returns how
  many it counted, took or gave; the player's screen follows as for any change to their inventory.
- `native_give_stack(uuid, stack)`, a whole stack (damage, name, enchantments... kept) to any player from any server
  hook, as `Inventory.add` places it (onto equal stacks, then empty inventory slots); what did not fit comes back, all
  of it for a player not here, dead, or swapped out by another player's hook. Nothing is dropped: an auto-pickup
  leaves the rest on the ground, a machine keeps it.
- Experience: `native_experience_orbs(dimension, min, max)`, the orbs in a box (id, position, `value`, and `count`
  for merged orbs), read before a mod cancels an orb in `native_entity_join_level`; and `native_give_experience(uuid,
  points)`, points given to any player as touching an orb gives them: Mending repairs their damaged items first, the
  rest goes to their experience, with the level-up sound; and `native_give_levels(uuid, levels)`, levels added or,
  negative, taken off (`Player.giveExperienceLevels`, as the anvil and the enchanting table charge; below 0 the level
  and points are 0; no level-up sound). Both return false for a player not here, dead, or swapped out by another
  player's hook, and where the server's player list is busy (some world ticks hold it): give first, and cancel the orb
  only when the give returned true. A merged orb is worth `value` × `count`.
- **`native_player_totem(player, name)`**: a totem of undying saved a player whose hurt would have killed them, on a server
  (`checkTotemDeathProtection`), heard just before it is used up. Listen only: the health state machine decides before a
  mod is asked, so a mod cannot veto it. The player's state is swapped in, so `native_acting_player`, `native_tell` and the
  item helpers reach them. Not heard: the host of a LAN world.
- API 21: `native_acting_player()`, the player whose state is swapped in (inside `native_perform_command`,
  `native_right_click_block`, `native_use_item_on`, `native_destroy_block`...; None elsewhere);
  `native_run_command(line)`, a command run as the server's console (level 4, no output), only where no player is
  swapped in (`native_tick`, a menu hook): a teleport to another level (`execute in minecraft:the_nether run tp Alex 0
  70 0`) lands at the next tick's start. `native_server_players()` and the acting player carry `health` (0 once dead).
  `native_action_bar(uuid, text)`, a line over a player's hotbar, which also reaches the acting player inside their
  hook.
- **`native_food_level_changed(player, name, from, to, cancel)`**: the food level of a player is about to change, on a
  server (eating, hunger and saturation effects, exhaustion, starving), before the new level reaches them. Change `to`
  (0 to 20) or set `cancel` to leave it at `from`, saturation included. It is heard once for what one turn of the
  player changed, not for the full bar a respawn gives, the food a player joins with, or the host of a LAN world. The
  player's state is swapped in, so `native_acting_player`, `native_tell` and the item helpers reach them.
- **`native_right_click_block(pos, face, canceled)`** (API 21): a player's right click on a block, before the block's
  own use and the item's (`ServerPlayerGameMode.useItemOn`), once per hand. A mod that sets `canceled` stops both,
  without a swing; the player's client is sent the block back. With `native_destroy_block` it keeps players out of
  claimed land.
- **`native_block_spread(dimension, kind, from, to, canceled)`** and **`native_piston_move(dimension, piston,
  direction, push, destroy, canceled)`** (API 22, on `GameLevel`, like the block observers): a fluid flowing or fire
  spreading (`kind` `fluid` or `fire`) from `from` into `to`, and a piston moving blocks once its structure resolved.
  Canceled, the block at `to` stays as it is, and the piston moves nothing. Fluids ask on every flow: answer from
  memory, fast.
- **`native_block_ignite(dimension, player, pos, cause, cancel)`** (on `GameLevel`): fire is about to be placed.
  It covers fire spread, lava's random fire placement, lightning strikes, and dispenser flint-and-steel. Set `cancel`
  to leave the position unchanged. Direct player flint-and-steel/fire-charge placement is not covered.
- **`native_block_burn(dimension, pos, cancel)`** (on `GameLevel`): fire's burnout check is about to remove the block at
  `pos`. Set `cancel` to keep it.
- **`native_dragon_egg_form(dimension, pos, cancel)`** (on `GameLevel`): the first Ender Dragon fight is about to place
  its egg at `pos`. Set `cancel` to leave the block unchanged.
- **`native_fluid_level_change(dimension, pos, fluid, from, to, cancel)`** (on `GameLevel`): flowing `water`, `lava`
  or a mod fluid (by its id, `ruby:honey`) at `pos` is about to change level. Levels run from 0 through 8. Set `cancel` to keep the old level.
- **`native_leaves_decay(dimension, pos, cancel)`** (on `GameLevel`): a non-persistent leaf block at distance 7 is
  about to decay. Set `cancel` to keep it.
- **`native_inventory_block_start(player, name, pos, cancel)`**: a guest is about to open an inventory supplied by a
  block at `pos`. Set `cancel` to leave it closed. The guest's state is not swapped in.
- **`native_moisture_change(dimension, pos, from, to, cancel)`** (on `GameLevel`): farmland at `pos` is about to
  change moisture. Set `cancel` to keep its old moisture.
- **`native_sculk_bloom(dimension, pos)`**: after a sculk catalyst at `pos` takes experience, its block has entered
  the blooming state.
- **`native_sponge_absorb(dimension, pos)`** (on `GameLevel`): a sponge at `pos` has absorbed water and become wet.
- **`native_target_hit(dimension, pos, strength)`**: a projectile hit the target block at `pos`, which now has the
  reported redstone output `strength`.
- **`native_vault_change_state(dimension, pos, state)`**: the vault block at `pos` changed its state to `state`
  (`inactive`, `active`, `unlocking`, or `ejecting`).
- **`native_block_failed_dispense(dimension, pos, item)`**: a dispenser at `pos` failed to dispense `item`.
- **`native_beacon_state_change(dimension, pos, active)`**: the beacon at `pos` gained or lost its complete beam
  pyramid; `active` says whether it is active now.
- **`native_anvil_damaged(dimension, pos, from, to)`**: the anvil at `pos` took damage, changing from `from` to
  `to`; `to` is air when it broke.
- **`native_block_break_progress_update(player, name, pos, progress)`**: a guest's crack animation at `pos` reached
  progress stage 0 through 9.
- **`native_block_lock_check(player, name, pos, locked)`**: a guest checks the lock on the container at `pos`;
  `locked` starts with vanilla's result and may be changed. The guest's state is not swapped in.
- **`native_block_fade(dimension, pos, from, to, cancel)`** (on `GameLevel`): ice or snow melts, or coral dies. `from`
  and `to` are block state IDs. Set `cancel` to keep the old block.
- **`native_block_form(dimension, pos, from, to, cancel)`** (on `GameLevel`): precipitation is about to place ice or
  snow, or add a snow layer at `pos`. `from` and `to` are block state IDs. Set `cancel` to keep the old state.
- **`native_note_play(dimension, player, pos, instrument, note, cancel)`** (on `GameLevel`): a note block is about to
  sound when punched, used, or powered. Set `cancel` to silence it; when used to tune it, the note remains unchanged.
- **`native_bell_ring(dimension, player, pos, cancel)`** (on `GameLevel`): a bell is about to ring. Set `cancel` to
  stop the ring and its raider reveal.
- **`native_player_physical_trigger(dimension, player, pos, block, cancel)`** (on `GameLevel`): a player stands in a
  pressure plate (wooden, stone, weighted) or a tripwire `block` at `pos` and would set it off (Bukkit's
  `PlayerInteractEvent` with `Action.PHYSICAL`). Asked each time the block counts what is inside it, before the
  effect, for every player in it. Set `cancel` and that player is not counted: the plate stays unpowered and the wire
  untripped for them, while another player or mob still sets it off. Farmland and turtle eggs a player lands on are
  `native_trample` and `native_entity_change_block_v2`; redstone ore, big dripleaf and sculk are not heard yet.
- **`native_cauldron_level_changed(dimension, player, pos, from_level, to_level, reason, cancel)`** (on `GameLevel`):
  precipitation, dripstone, or a burning player being extinguished is about to change a cauldron's level. Set `cancel`
  to leave it as it was.
- **`native_random_tick(ticks, dimension, pos, state, handled)`** (API 33, on `GameLevel`): a block `tickChunk`
  random-ticks, before its own `randomTick`. Change the block with the level's `native_set_block_property(ticks, pos,
  key, value, flags)`; set `handled` to skip the block's own random tick.
- **`native_tick_block(ticks, dimension, pos, state, handled)`** (API 34, on `GameLevel`): a due scheduled tick of a block
  that is still there (`ServerLevel.tickBlock`), before its own `tick`; the same contract as `native_random_tick`.
- **`native_explosion_detonate(dimension, center, blocks)`** (API 22): the positions an explosion is about to break,
  before any is hit; take out the ones to spare. Entities are hurt as before.
- **`native_explosion_knockback(dimension, center, blocks, entity, knockback_velocity)`**: a server explosion's
  calculated velocity, before it is added to the entity. Replace `knockback_velocity` to change the push;
  `entity` is None for a player, whom `native_acting_player` names. The player packet uses the same replacement. Wind bursts are heard too.
  Bundles need `"hooks": 374`.
- **`native_fill_bucket(stack, pos, canceled)`** and **`native_attack_entity(entity, kind, pos, canceled)`** (API 22):
  a player's bucket at a block, and their blow on an entity, with the acting player swapped in. Canceled, the bucket
  does nothing and the blow does no harm. A fluid or mob bucket also asks about the block beside the clicked face
  whenever that block is not its target, since a block that refuses the fluid passes it there; the fluid may still
  stay in the target.

#### Server hooks for plugin ports: player bots

Helpers for server mods whose bots are server players driven by the mod (a fake client that sends its own moves, as a
PvP bot plugin's), in `crates/horizon-client/src/game/native_bots.rs`, on a server only. Nothing runs unless a mod
calls them, and neither changes the player or the level. API 454.

- **`native_simulate_player_move(uuid, NativePlayerInput) -> Option<NativeSimulatedMove>`** plays one tick of the
  server player's own movement from a bot's keys, with the client movement code the server's movement prediction
  replays a client tick with (`LocalPlayer.aiStep` and `Player.travel`): `forward` and `strafe` (-1..1; a value in
  between is a stick's partial push), `jump`, `sneak` (slower, held at edges), `sprint` (only with a forward input),
  `yaw` and `pitch` in degrees. Gravity, friction, block speed factors, stepping up, swimming and climbing follow. It
  starts where the player is now, with `velocity` when given, else the server's copy: the server doesn't keep a bot's
  momentum, so pass back the `velocity` of the last answer each tick. The answer is the `position`, `velocity`,
  `on_ground`, `horizontal_collision` and `sprinting` after the tick; the player stays where they are until the mod
  moves them (its bot's move packet, `native_teleport_player`). Entities don't block it (a boat, a shulker). None off
  a server, for a player in another level than the one in place, a guest out of reach inside another player's hook
  (the hub is taken out), dead, a spectator or riding, and for an input that is not a number.
- **`native_collision_boxes(dimension, aabb) -> Vec<[f64; 6]>`** lists the block collision boxes that share volume
  with `aabb` (`[min_x, min_y, min_z, max_x, max_y, max_z]`) in the level `dimension`, in world space, as
  `Level.getBlockCollisions(null, aabb)` gives them under the empty collision context (a stair gives two boxes, a
  fence its post): for a bot's own collision checks and pathfinding. Entities and the world border aren't in it.
  Empty off a server, for another level than the one in place, a box whose corners are not numbers or out of order,
  and a box over about a million blocks.

### Mobs

API 37, `crates/horizon-client/src/game/native_mobs.rs`: helpers for server mods, for the level in place, on a server only (on a
client they do nothing, and the readers give nothing). Each takes the `dimension` (`minecraft:overworld`) of the level it means, as
`native_set_block` does, and is refused when another level is in place. The per-level tick hooks (API 23) run with a defined level in
place; `native_server_tick` does not define one, so name the dimension and expect a refusal there.

- `native_spawn_mob(dimension, kind, pos, &NativeSpawn)` spawns a mob and returns its id, in any loaded level since API
  535 ([NPCs](#server-hooks-for-plugin-ports-npcs)). `NativeSpawn` holds an optional name and health, and the
  `no_ai`, `invisible`, `silent`, `invulnerable` and `no_gravity` flags; its default is a plain mob. The mob is finalized as a natural spawn
  is, and kept when its chunk unloads. Nothing is spawned outside `/summon`'s bounds, in a chunk that isn't loaded, or a monster on peaceful.
- `native_mob(dimension, id)` and `native_mobs(dimension)` read the living mobs: id, dimension, kind, position, yaw, health and maximum health.
- `native_teleport_mob(dimension, id, pos)` (as `/tp`: it dismounts and drops its path), `native_push_mob(dimension, id, velocity)` and
  `native_set_mob_health(dimension, id, health)` change one, and answer false for no such mob or a value that isn't finite. Health must be
  above 0.
- `native_kill_mob(dimension, id)` kills as `/kill` does, with its loot; `native_remove_mob(dimension, id)` takes the mob away with no death
  and no loot (it is not saved either).
- While the game ticks its mobs they are out of the level: from a hook inside that tick, a spawned mob joins after the tick, and the others
  read as gone.

### Mob goals

**`horizon_client_lib::mobs::ai::native_register_goals(mob)`** (vanilla's `Mob.registerGoals`; a trait mod's
`Hooks::register_goals(event)`, with `event.mob`) is asked for every new mob, after its species added its goals and before its
saved data is read. `mob.ai.goals` is vanilla's `goalSelector`, `mob.ai.targets` its `targetSelector`. A mod's living type
([mods.md](mods.md)) comes with its template's goals, a zombie's attacks, strolls and targets: clear them to replace them, as a
class whose `registerGoals` skips `super`'s, or add to them. A villager acts through its brain, which goals don't replace:
a type from the villager template still works, sleeps and trades as a villager, next to the goals a mod adds. For a
neutral mob whose goals are all the mod's, copy `minecraft:wandering_trader`. A goal is a
Rust type implementing
`horizon_client::mobs::goals::Goal` (a bundle compiles into the game, so nothing is loaded at run time): its `flags` (`MOVE`,
`LOOK`, `JUMP`, `TARGET`), `can_use`, `can_continue`, `start`, `tick` and `stop` run by priority and flag as vanilla's
`GoalSelector` runs them. The vanilla goals in that module (`Float`, `Stroll`, `LookAtPlayer`, `MeleeAttack`, `NearestTarget`
and the rest) are there to reuse.

```rust
use horizon_client::mobs::{Mob, goals::{self, Cx, Goal}};
use horizon_client::mod_api::{Hooks, RegisterGoalsEvent};

/// Walks the mob back to its post when it is more than 4 blocks away.
#[derive(Clone, Debug)]
struct ToPost { post: [f64; 3] }
impl Goal for ToPost {
    fn flags(&self) -> u8 { goals::MOVE }
    fn can_use(&mut self, m: &mut Mob, _: &mut Cx) -> bool {
        let [x, y, z] = self.post;
        let (dx, dy, dz) = (m.position.x - x, m.position.y - y, m.position.z - z);
        dx * dx + dy * dy + dz * dz > 16.0
    }
    fn can_continue(&mut self, m: &mut Mob, _: &mut Cx) -> bool { !m.ai.nav.is_done() }
    fn start(&mut self, m: &mut Mob, cx: &mut Cx) { m.nav_move_to(cx.w, self.post, 1.0); }
    fn box_clone(&self) -> Box<dyn Goal> { Box::new(self.clone()) }
}

struct Guards;
impl Hooks for Guards {
    fn register_goals(&self, event: &mut RegisterGoalsEvent<'_>) {
        if event.mob.kind.id == "example:guard" {
            let ai = &mut event.mob.ai;
            ai.goals.clear();
            ai.targets.clear();
            ai.goals.add(0, goals::Float);
            ai.goals.add(1, ToPost { post: [0.5, 64.0, 0.5] });
        }
    }
}
cargo build -p cargo-horizon-mod
# From crates/horizon-mod-wasm/tests/fixture, with ROOT set to the repository:
"$ROOT/target/debug/cargo-horizon-mod" horizon-mod build --wasm --catalog points.json
"$ROOT/target/debug/cargo-horizon-mod" horizon-mod wasm-std "$ROOT/target/horizon-wasm-std.wasm" --catalog points.json
# From the repository root:
cargo test -p horizon-mod-wasm --features listener-fixture --test native_listener collect_positions_copy_in_and_out_natively_as_in_the_real_guest -- --exact
HORIZON_WASM_FIXTURE="$PWD/crates/horizon-mod-wasm/tests/fixture/target/horizon-mod/wasm_fixture.hmod" HORIZON_WASM_STD="$PWD/target/horizon-wasm-std.wasm" cargo test -p horizon-mod-wasm --features listener-fixture --test real_member real_member -- --ignored --exact
```

### Victim ownership and hot path

One immutable accepted-hit record follows the mob or player body; there is no
separate death-weapon table. Mob reconstruction moves it through the existing
non-Clone vehicle transfer token, along with the processed death attempt; the
real `DeathTime` NBT codec preserves the original removal deadline. A full player
teleport restore moves the record through the existing body-replacement boundary,
and respawn discards it.
Each body stores an optional shared pointer; snapshots share the immutable
record rather than copying its source name and stack. Records are captured only
for listened fatal deaths/loot or accepted projectiles needed by `loot_drops`.
Without those listeners, no accepted record or saved weapon is allocated.
Armor's existing deferred event carries the actual cause, direct entity and
position as scalars; synchronous equipment wear borrows the paused hit's name.
The attack scope retains a projectile's weapon only when incoming damage,
death or loot listeners need it. No idle-tick sweep or second source map is added.

Prepared-resource Game regressions live in
`game::native_hooks::death_source_tests`; the player cases additionally require
loopback sockets. `game::lan_actions::guest_body::tests::accepted_hit_moves_through_teleport_restore_but_not_respawn`
covers the ownership transfer. These are source regressions, not a record of a
completed run.

The clean-room contract references are the public
[NeoForge 26.3 LivingDeathEvent Javadoc](https://aldak0.ru/javadoc/26.3.x/net/neoforged/neoforge/event/entity/living/LivingDeathEvent.html)
and [ArmorHurtEvent Javadoc](https://aldak0.ru/javadoc/26.3.x/net/neoforged/neoforge/event/entity/living/ArmorHurtEvent.html),
plus the pinned game's `LivingEntity.die`, `LivingEntity.tickDeath` and
`DamageSources` factories. NeoForge's LGPL source and patches were not read.

## Prebuilt SDK layout

The v4 prebuilt archive separates target artifacts (`deps/`) from host proc-macro
libraries (`host/`). Cargo builds with an explicit `--target <rustc host triple>`,
so host build dependencies cannot introduce a second metadata variant into the
target SDK. Only host proc-macro dynamic libraries are shipped, not their host
dependency metadata. Consumers pass both folders as `-L dependency` paths; named
extern and facade lookups use only `deps/`. `native/` still holds native linker
inputs. The identity manifest hashes files in all three folders.

Target compilation retains `-C prefer-dynamic` (and `-C rpath` on macOS), while
Cargo's explicit target keeps those flags off host proc-macros. Multiple versions
of a target dependency are legitimate: uniqueness is required only when selecting
a named extern or publishing an internals facade, never across the whole SDK.

To rebuild and prove the archive with the launcher's pinned Rust toolchain active:

```sh
python3 tools/prebuilt_game.py build prebuilt
python3 tools/prebuilt_game.py smoke prebuilt
python3 tools/prebuilt_game.py exports prebuilt
python3 tools/native_dylib_probe.py --sdk prebuilt --output target/native-dylib-probe
```

For a server archive, add `--server` to the build command. Use a fresh
`CARGO_TARGET_DIR` when collecting a clean-build proof; all commands must use the
same compiler as `launcher/src/rust_toolchain.rs`.

## Upstream service contracts carried by this sync

The upstream additions below describe behavior to preserve in the typed `Host` and
handle API, not additional `native_*` entry points or a bundle API counter. Use the
book and generated API reference for the available signatures; pending hook fragments
must be converted to the hook catalog before being advertised as available.

- Tamed-mob death messages are offered after construction and only when an online
  owner would receive them with `show_death_messages` enabled. Listeners may replace
  or suppress the message in dispatch order. The server-only named-mob log is not
  a second player-message hook.
- Dedicated-server settings describe the live listener: account authentication,
  bound port and whether Velocity forwarding is active. They never expose the
  forwarding secret, and are unavailable before listening or on integrated/LAN
  servers. Secure-profile enforcement and hardcore are not part of this read.
- Vehicle and passenger reads use entity handles (`entity_vehicle` and
  `entity_passengers` on `Host`), not parallel numeric-ID helpers. Passenger order
  follows vanilla, with the controlling passenger first. These are read-only.
- UUID lookup from a read-only context must not swap levels. A mutable lookup can
  reach other loaded levels; entity NBT reads there must refuse unloaded levels,
  missing entities and client-side use rather than load a level implicitly.
- Map reads return the 128 × 128 row-major color bytes and scale, dimension and
  locked metadata. Reading saved data must not install a loaded map as a side
  effect; unknown maps and client-side world reads return no value.
- Living-entity sight uses eye-to-eye COLLIDER shapes, no fluids, the source's
  collision context and vanilla's 128-block limit. Unknown entities, different
  levels and unavailable server levels must not report visibility. The typed
  `living_has_line_of_sight` and `level_line_of_sight` reads replace raw-ID helper
  proposals; the point-to-point form has no entity collision context.
- Camera submersion (main's `native_camera_submersion(camera, fog_type)`, API 646): a client
  listener runs when the camera's fluid submersion type is evaluated
  (`Camera.getFluidInCamera`), before fog and underwater screen overlays are prepared.
  Setting `fog_type` to `FogType::Atmospheric` clears underwater, lava and powder-snow fog
  and overlays (Freecam uses it). Hook entry: `camera_submersion` v1, client, `listen`.
- Client entity update (main's `native_client_entity_update(viewer, id, nbt) -> bool`,
  `host.client_entity_update`): like `native_client_entity_set` for a few fields. It reads
  `nbt` as `set` does but sends only the data of the keys `nbt` has (`transformation` sends
  its four parts); a key at its default value is sent, so a field can go back to it. A
  display's `start_interpolation` goes with it, so a changed transformation eases over
  `interpolation_duration`. `set` still sends every field. False when `id` isn't this
  viewer's client-side entity, when the viewer isn't on, and outside a player's hook.

These are contract notes, not runtime verification of the converted services.

## Living absorption

Living handles expose `absorption(host)` on both sides and
`set_absorption(host, amount)` on the server. Resolution follows health: armor
stands, dragons and stale handles return `None` on reads. A non-finite write
returns `false` and changes nothing; accepted amounts clamp to
`0..=max_absorption` for that entity. Player and mob writes roll back with the
transaction. This is the absorption amount, not the absorption status effect.
