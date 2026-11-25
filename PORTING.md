# Porting notes

This document records how the Fabric **Female Gender Mod 5.0.0** was mapped
onto Horizon at revision `81ae4a7d8f27d38e5cabbdcda0a06b2c4b000045`
(`horizon-mixins` `ddd9700b21997c95d95ac6ee5ecafe3830cf622a`), and where the
port deliberately differs.

## Architecture

The bundle is `mixins/female_gender_mod/`:

| Module | Target file | Purpose |
| --- | --- | --- |
| `fgm` | `crates/horizon-client/src/game.rs` | settings page, in-game screen, once-a-tick physics |
| `fgm_remote` | `crates/horizon-client/src/game/remote.rs` | play registration and sync |
| `fgm_net` (`pub`) | `crates/horizon-client/src/net.rs` | configuration handshake flush |
| `fgm_render` | `crates/horizon-client/src/mobs/render.rs` | breast and jacket geometry |

All settings, codecs, physics and UI logic live in the `fgm_core` crate
(`core/`), which has no dependencies. The injected modules only read Horizon
state and hand values to `fgm_core`.

The injected helper functions are one function per file (`list.rs`,
`settings.rs`, `click.rs`, `tick.rs`, `frame.rs`, `key.rs`, `screen_*.rs`,
`remote_poll.rs`, `remote_packet.rs`, `net_register.rs`, `net_brand.rs`,
`render.rs`). They have no state of their own.

## In-game screen

Pressing the open-menu key (`H` by default) while playing opens a character
personalization panel with the original three tabs. Horizon cannot add a
screen of its own, so the panel uses the Options screen as its host: the key
opens Options with the pointer released, `native_screen_render_post` draws an
opaque panel over it, and the screen input hooks take every click, wheel step
and key while it is up. `Menu` in `fgm_core::menu` is the model and layout;
`fgm.rs` is the thin game side. Opening is deferred to the next frame, so the
key that asks for the panel never reaches the Options screen, and it is
refused on the title screen, the pause menu, chat, containers, server dialogs
and every other non-play screen. The same key, Escape and Done close it and
return to the world with the pointer captured.

## Settings

`fgm_core::config` mirrors the original JSON model:

- `AvatarConfig`: `gender`, `breasts`, `uvs`, `sounds`, `show_in_armor`.
- `Breasts`: `x_offset`, `y_offset`, `z_offset`, `bust_size`, `cleavage`,
  `physics`, `uniboob`, `bounce_multiplier`, `floppiness`.
- `Sounds`: `hurt`, `voice_pitch`.
- `ClientConfig`: `first_time_load`, `show_toast`, `armor_stat`,
  `player_list_mode`, `hide_own_contributor_tag`, the four cloud fields and
  the three overrides (`armor_physics_override`, `disable_rendering`,
  `disable_sound_replacement`).

Defaults are the original ones, including `gender = MALE` (the mod starts
without breasts, as upstream). The values are stored under the mod's
`config` key in Horizon's native data folder as a compact `key=value;` text,
not JSON; it round-trips every setting, including hidden UV faces
(`key=hidden;`).

Ranges are clamped exactly as the original (`Breasts.STREAM_CODEC`
`ByteBufCodecs` ranges): X/Y in `[-1, 1]`, Z in `[-1, 0]`, bust in `[0, 0.8]`,
cleavage in `[0, 0.1]`, bounce in `[0, 0.5]`, floppiness in `[0.25, 1]`,
voice pitch in `[0.8, 1.2]`.

## Wire format

Channels:

```
female_gender_mod:clientbound/hello
female_gender_mod:serverbound/hello
female_gender_mod:clientbound/sync
female_gender_mod:serverbound/sync
wildfire_gender:sync                 (TheReizi plugin, clientbound)
wildfire_gender:send_gender_info     (TheReizi plugin, serverbound)
```

`AvatarConfig.COMPACT_STREAM_CODEC` is reproduced byte for byte:

1. `bool` present; `false` means the male default and the payload ends.
2. gender ordinal as a VarInt (`FEMALE=0`, `MALE=1`, `OTHER=2`).
3. six `f32` big-endian values (X, Y, Z, bust, cleavage), `bool` physics and
   — only when physics is on — `bool` uniboob, `f32` bounce, `f32` floppiness.
4. four UV layouts. Each is a VarInt entry count followed by entries of
   `VarInt direction` (east 0, west 1, down 2, up 3, north 4) and four
   VarInts `x1, y1, x2, y2`. Absent faces are `None`; the default layouts
   keep their all-zero `UVQuad.UNUSED` faces, so the encoding size matches the
   Java server.
5. `bool` hurt sound, `f32` voice pitch, `bool` show-in-armor.

`clientbound/sync` is a 16-byte big-endian UUID followed by the compact
config; `serverbound/sync` is the compact config alone.

The legacy `wildfire_gender:*` payload is the plugin's V5 format: a 16-byte
big-endian UUID, the gender ordinal, the full (non-compact) breast/sound
fields in the plugin's order, and four UV-layout maps. It has no presence
boolean and no separate serverbound/clientbound difference; the UUID is
always present. The port writes all five faces of every legacy layout, using
the default quad for a deliberately hidden face, because the plugin fills
missing entries with the mod defaults. Incoming legacy layouts may be empty
(the 1.0.0 plugin sends them that way); the port fills those with the upstream
defaults.

### Fabric handshake

Horizon has no Fabric API, so the bundle performs the common handshake itself:

1. The server sends `minecraft:register` (NUL-separated channels) and a ping
   during configuration. `receive_brand` queues our `minecraft:register`
   reply listing `female_gender_mod:clientbound/hello`, `c:version` and
   `c:register`.
2. `c:version` → reply `[1, 1]` (one version, protocol 1).
3. `c:register` for `"play"` → reply with the received version, `"play"` and
   the four play channels (5.0.0 sync plus the legacy plugin pair). This is
   what lets the server's `ServerPlayNetworking.canSend` succeed for the
   tracker broadcasts.
4. `clientbound/hello` → reply `serverbound/hello` with VarInt protocol `2`.
   The server stores version 2 on the connection and only then registers the
   play receivers.

The replies are queued on the net thread and written by the game thread from
`Game::poll_remote` while `Session.configuring` is set. The writer lock is
held across the `configuring` check, so a reply can never leak into play or
into a later server; `configure` clears the queue when a new configuration
starts. Once joined, the bundle also sends the legacy
`minecraft:register` play registration (all four sync channels) as a fallback
for servers that track players early.

The local config is sent once after join and again after every local change
(`State::play_actions` / `play_failed`). Both protocol families are sent in
the same frame; the Paper plugin ignores the 5.0.0 channel and a Fabric server
ignores the legacy channel. The server rebroadcasts the config to every
tracker. Incoming packets on either family are mapped from UUID to entity id
through `Remote.level_players`; the local player is always entity id 0 and
uses the local settings, never the server's copy.

## Physics

`fgm_core::physics` is a line-by-line port of `BreastPhysics` and
`BothBreastsPhysics`:

- the vertical/horizontal springs, targets, velocity damping, bounce
  rotation, breast-size interpolation and the deterministic
  `LegacyRandom`-like RNG;
- pose handling (standing, crouching, sleeping), walk animation, swing
  detection and the armor resistance/tightness factors;
- `Armor::from_chest` carries the original `armor_data` values.

Vehicle-specific impulses (boats, minecarts, horses, pigs, striders) are not
ported; everything else follows the original formulas. Physics runs every
tick for the local player and every visible player, whether or not the layer
is currently drawn, matching the original's separate physics update.

## Rendering

The original `GenderLayer` builds a five-face box (four by five by three
pixels) per breast with `WildfireModelRenderer` and draws it after the body.
Horizon draws players through `mobs::render::render_with`; the bundle hooks
the second (normal) `render_visible` call with an `AFTER_CALL` injection and
appends the box vertices to the same `Vec<Vertex>`.

The port reproduces:

- the body matrix from `base_transform` + the `body` part matrix with the
  same `anim::state`/`Poses` inputs Horizon's own body pass uses;
- offsets (`i * 0.0625`, `0.05625 + j * 0.0625`, `zOffset - 0.125 + k *
  0.0425`), the uniboob shifts, bounce translation/rotation, breathing,
  `-35° * size` tilt, the outward Y rotation (positive left, negative right)
  and the `0.9995` X scale;
- the exact Java face order and vertex quads, the face-index rule (`None`
  shifts later faces, `UVQuad.UNUSED` consumes an index but draws nothing),
  the local UV corners and the 64×64 texture scale;
- the jacket overlay (`translate(0, 0, -0.015); scale(1.05)`) on the two
  overlay layouts;
- hurt/flash overlay, body tint, and entity OIT marking for the translucent
  player render type;
- Java's `round(value, 1)` (`floor(value * 10 + 0.5) / 10`) for the offsets,
  which differs from Rust's `round` on negative halves.

Armor: `GenderArmorLayer` is approximated in the same append pass. When a
chestplate covers the breasts, the boxes use the chestplate's own equipment
texture (`entity/equipment/humanoid/<material>`) with
`IBreastArmorTexture.DEFAULT_UVS` (a 64×32 texture) and the armor layer's
extra transform (`+0.001` x per side, `+0.015` y, `-0.015` z, `1.05` x).
That choice follows `armor.covers` / `GenderArmorLayer.isLayerVisible`; the
client armor physics override only changes the physics and the
`show_in_armor` gate, so it cannot switch a covered chestplate back to the
skin texture.
Leather is tinted with the same `armor_tint` the chestplate uses. If the
atlas has no matching equipment texture (a custom armor asset), the layer
stays hidden under the chestplate rather than drawing skin through it.
Resource-pack `armor_data` texture overrides and armor trims are not read;
the physical behavior and the chestplate's shield values
(`isChestplateOccupied`, resistance, tightness, "show in armor") match.

Mannequins have their own config type upstream and are not part of the player
sync, so only `minecraft:player` entities use this layer.

## UI

Horizon exposes a native mod's settings as a flat list of labels with click
indices. `fgm_core::ui` keeps a page stack:

```
Root
├─ Appearance (bust, cleavage, X/Y/Z offsets, reset)
├─ Breast Physics (physics, uniboob, intensity, momentum, reset)
├─ Sounds (hurt sounds, pitch, disable replacement, reset)
├─ Armor (show in armor, armor physics override, armor tooltip, reset)
├─ UV Layouts → Layout → Face (X1/Y1/X2/Y2 ± 1, visible/hidden, resets)
├─ Client Settings (rendering, toast, first-time flag, player list,
│                    contributor tag, sound replacement, reset)
├─ Cloud Sync (enabled, automatic, server, verbosity)
└─ Reset (per section, UVs, everything)
```

Every numeric value has separate `(-)` and `(+)` rows, so a click never wraps
through a range. Labels and click handling come from the same `buttons`
function, so an index can never refer to a different row than the one shown.

`Reset Everything` rebuilds the page stack and restores all defaults;
`Reset Client Settings` keeps `first_time_load`, because that flag marks the
original first-run tutorial and resetting it would show it again.

## Not ported

- **Custom hurt sound assets.** Horizon's native API has no way to register
  a sound event and asset from a mod, so the original female hurt sounds
  cannot play. The `hurt`/`voice_pitch` settings and the
  `disable_sound_replacement` override are still stored, synced and exposed.
- **Cloud sync.** The celestialfault API (`/v2` profiles, fetch/delete) is
  not implemented; the enabled/automatic/server/verbosity settings are kept
  and marked unavailable in the page.
- **Client UI extras.** The player-list gender entry (`player_list_mode`), the
  armor tooltip stat (`armor_stat`), the sync toast (`show_toast`) and the
  contributor tag (`hide_own_contributor_tag`) are stored and exposed, but
  this port has no player-list panel, armor tooltip hook or toast renderer, so
  they do not change anything on screen.
- **Keybinds.** The original binds a config key and a render toggle. The
  config key is ported as the open-menu key (`H` by default, rebindable under
  Options → Mods); the render toggle is not bound. Horizon has no API for a
  mod's own screen, so the panel rides the Options screen and draws over it.
- **First-run tutorial and toasts.** The sync toast and first-time flag are
  stored but no custom onboarding screen is drawn.

## Source of truth

The original jar was unpacked into `reference/` (decompiled with CFR and its
assets) and was used for every default, range, codec, matrix and armor value.
The LGPL-3.0 license is preserved as `LICENSE`; the upstream project is
WildfireMC's Female Gender Mod, and this port is a derivative work under the
same license.
