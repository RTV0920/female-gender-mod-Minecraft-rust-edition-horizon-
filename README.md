# Female Gender Mod for Horizon

A native Rust port of **Female Gender Mod 5.0.0** (Fabric, Minecraft 26.3) to
[Horizon](https://github.com/horizon-client), the Rust Minecraft client. It
ports the avatar settings, the spring physics, the UV layouts and the
Fabric network protocol, so a Horizon player and a Java player on the same
Fabric server see each other's settings.

**Port version 5.1.0** adds the in-game **Character Personalization**
screen: press `H` during play (never on the title screen, the pause menu or
another screen) for the original Customization, Breast Physics and
Miscellaneous tabs and their sliders and toggles. The key is rebindable under
Options → Mods. It keeps 5.0.1's Fabric Registry Sync support, so it can join
LAN worlds and integrated servers whose synced registries contain modded
entries (for example FGM's client-only `female_gender_mod:female_hurt` sound
event).

Upstream: <https://github.com/WildfireMC/FemaleGenderMod>
License: **LGPL-3.0-only** (see `LICENSE`; the original jar and its license
are preserved in `reference/`).

## What is ported

Everything the Fabric mod stores and syncs:

| Setting | Values / default |
| --- | --- |
| Gender | female / male / other (default male, as upstream) |
| Bust size | 0.0 – 0.8 (default 0.6) |
| Cleavage | 0.0 – 0.1, rotation 0 – 10° (default 0) |
| Offsets X / Y / Z | X -1.0 – 1.0, Y -1.0 – 1.0, Z -1.0 – 0.0 (default 0) |
| Breast physics | on by default |
| Uniboob | on by default |
| Bounce intensity | 0.0 – 0.5 (default 0.333) |
| Momentum / floppiness | 0.25 – 1.0 (default 0.75) |
| Female hurt sounds | on by default |
| Voice pitch | 0.8 – 1.2 (default 1.0) |
| Show breasts in armor | on by default |
| UV layouts | 4 layouts × 5 directions, full in-game editor with hidden faces |
| Client: disable rendering | off |
| Client: armor physics override | off |
| Client: armor tooltip stat | on |
| Client: disable sound replacement | off |
| Client: sync toast | on |
| Client: player-list mode | Mod UI only / Tab list open / Always |
| Client: hide own contributor tag | off |
| Cloud: enabled / automatic / server / verbosity | off / off / default / default |

The original `armor_data` values are ported for every chestplate the upstream
ships (leather, chain, iron, gold, diamond, netherite, turtle, elytra and the
modded names it lists): whether it covers, hides, its physics resistance and
tightness.

## Requirements

- Horizon built on `81ae4a7d8f27d38e5cabbdcda0a06b2c4b000045`
- Horizon launcher 0.2.37 or newer
- The `horizon` instance in the launcher

Native mods run with your rights and are not sandboxed, so the launcher asks
for `--trust-native-code`.

## Build

```sh
horizon-launcher native-build \
  /home/roman/rust/minecraft-rust-mods/female_gender_mod/mixins/female_gender_mod \
  --instance horizon --trust-native-code
```

The bundle manifest is `mixins/female_gender_mod/horizon-mixin.toml`; the
package metadata for sharing is `horizon-native.json` at the repository root.
The launcher builds the pinned Horizon revision, applies the mixins, compiles
the game and makes the result the version the instance plays.

## Use

1. Start the `horizon` instance.
2. Press **H** while playing to open **Character Personalization**. The three
   tabs carry the original sliders and toggles; **Done**, **Esc** or **H**
   close it. It does not open from the title screen, the pause menu, the
   inventory, chat or any other screen.
3. Open **Options → Mods → Female Gender Mod** for the full page: gender, the
   UV editor, cloud and client settings, and the key bind.
4. Change gender, size, physics, sounds, armor, UVs or client settings. Every
   value is saved immediately in the mod's data folder (`config` key) and the
   avatar config is resent to the server when it changed.
5. Join a Fabric server with the original mod. Horizon players appear to Java
   players with the same geometry and settings, and Java players' configs are
   received and drawn by Horizon.
6. Paper/Purpur servers with TheReizi's **Female Gender Plugin 1.0.0** also
   work: the port speaks that plugin's legacy `wildfire_gender:*` channels in
   addition to the 5.0.0 Fabric channels, so Horizon players sync with each
   other through the plugin. The plugin itself does not implement the 5.0.0
   `female_gender_mod:*` handshake, so Java clients running the original 5.0.0
   mod still cannot sync through it.

## Interoperability

The port speaks the original Fabric protocol:

- configuration phase: `minecraft:register`, `c:version`, `c:register`,
  `fabric:registry/sync` → `fabric:registry/sync/complete`, and
  `female_gender_mod:clientbound/hello` → `serverbound/hello` (protocol 2);
- play phase: `female_gender_mod:serverbound/sync` and
  `female_gender_mod:clientbound/sync` with `AvatarConfig.COMPACT_STREAM_CODEC`.

The registry map is acknowledged without remapping, like the original client
does for a local server. This is safe here because FGM's only extra synced
entry is appended to the sound registry, so vanilla raw IDs stay unchanged.

It also speaks the pre-5.0.0 channels used by TheReizi's Paper plugin:
`wildfire_gender:send_gender_info` and `wildfire_gender:sync`, with the
plugin's V5 payload (UUID plus the full config and UV layouts).

The server keeps the per-player config and rebroadcasts it to every player
tracking that player, so one sync on join plus one after each local change is
enough. Rust players therefore show up for Java clients, and Java players'
physics and settings show up in Rust. The port sends both protocol families on
each sync; a Fabric server ignores the legacy payload and the Paper plugin
ignores the 5.0.0 payload.

## Limitations

- The custom female hurt sound **assets** are not registered: Horizon's native
  audio API has no way to add a sound event from a mod. The setting is stored
  and synced, and "Disable Sound Replacement" is kept, but no custom sound
  plays in this port.
- The celestialfault cloud sync API is not implemented. All cloud settings are
  stored, but "Sync Now" and "Delete Cloud Profile" are inert.
- The original's render-toggle keybind is not bound. Its config key is the
  open-menu key (`H` by default, rebindable under Options → Mods): the
  in-game screen carries the Customization, Physics and Misc sliders and
  toggles, while gender, the UV editor, cloud and client settings stay on the
  Options → Mods page.
- Armor borrows the chestplate's own equipment texture: whenever a chestplate
  covers the breasts and the layer is visible, the breast boxes are drawn
  with `entity/equipment/humanoid/<material>` and the original default armor
  UVs (64×32), so the bulge looks like part of the chestplate. This follows
  the chestplate's coverage, not the armor physics override, so the override
  never makes a covered layer fall back to the player's skin. Leather keeps
  its dye tint. Custom resource-pack `armor_data` texture overrides and armor
  trims are not read; if the atlas has no matching equipment texture, the
  layer stays hidden under the armor instead of drawing skin through it. The
  physics resistance/tightness of the chestplate is still applied.

## Layout

```
horizon-native.json                 package metadata
LICENSE                             LGPL-3.0
README.md                           this file
PORTING.md                          implementation notes
mixins/female_gender_mod/
  horizon-mixin.toml                bundle manifest
  fgm.rs                            game hooks: settings page, in-game screen, physics
  frame.rs                          per-frame screen open/close helper
  fgm_remote.rs                     play registration and sync
  fgm_net.rs                        configuration handshake flush
  fgm_render.rs                     breast and jacket geometry
  *.rs                              small injected helper functions
  core/                             fgm_core crate
    src/config.rs                   settings, defaults, text persistence
    src/net.rs                      Fabric codecs and config queue
    src/physics.rs                  BreastPhysics / BothBreasts port
    src/state.rs                    runtime state, gating, sync bookkeeping
    src/ui.rs                       pages, buttons and click actions
    src/menu.rs                     in-game screen model and layout
reference/                          original jar contents (decompiled) and license
```
