//! The mod's runtime state: the local settings, the avatar configs learned
//! from the server, the physics of every rendered player and the page stack of
//! the settings screen.
//!
//! The game, network and render hooks all run on one thread; the mutex is
//! there because a hook's signature asks for `&mut` and another hook (the
//! network queue) is entered from the net thread. Every critical section is
//! short and no closure passed to [`with`] may call another [`with`].

use crate::config::{AvatarConfig, ClientConfig, deserialize, serialize};
use crate::menu::{Menu, Outcome, Widget};
use crate::physics::{Armor, BothBreasts, BothValues, EntityMotion};
use crate::ui::Ui;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// What the play phase still has to send for this connection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayActions {
    /// Register our Fabric channels with `minecraft:register`.
    pub register: bool,
    /// Send our own `AvatarConfig` to the server.
    pub sync: bool,
}

/// The values the render layer needs for one player.
#[derive(Clone, Debug)]
pub struct RenderInfo {
    pub config: AvatarConfig,
    pub values: BothValues,
    pub disable_rendering: bool,
    pub armor_physics_override: bool,
}

impl RenderInfo {
    /// `GenderLayer.isChestplateOccupied`: the chestplate hides physics unless
    /// the client override is on.
    pub fn chestplate_occupied(&self, armor: Armor) -> bool {
        armor.covers && !self.armor_physics_override
    }

    /// `GenderArmorLayer.isLayerVisible`: a covering chestplate is always
    /// drawn with the armour's texture. The physics override changes the
    /// physics and the `show_in_armor` gate, not what visually covers the
    /// breasts, so this stays true while `chestplate_occupied` is false.
    pub fn armor_layer_visible(&self, armor: Armor) -> bool {
        armor.covers
    }

    /// `GenderLayer.setupRender`: whether the layer draws at all.
    pub fn visible(&self, armor: Armor) -> bool {
        if self.disable_rendering || armor.hides {
            return false;
        }
        if !self.config.show_in_armor && self.chestplate_occupied(armor) {
            return false;
        }
        self.base_size() >= 0.02
    }

    /// `breastSize` before the extra size below is added; the original hides
    /// the layer when this is under 0.02.
    pub fn base_size(&self) -> f32 {
        let b = self.values.left.breast_size;
        if b > 0.7 { b } else { (b * 1.5).min(0.7) }
    }

    /// `GenderLayer.breastSize`: the size the rotations and translations use.
    pub fn breast_size(&self) -> f32 {
        let b = self.values.left.breast_size;
        self.base_size() + 0.5 * (b - 0.7).abs() * 2.0
    }

    pub fn z_offset(&self) -> f32 {
        0.0625 - self.values.left.breast_size * 0.0625
    }

    pub fn outward_angle(&self) -> f32 {
        (self.config.breasts.cleavage * 100.0).round().min(10.0)
    }

    /// `bounceEnabled`: physics on and the chestplate does not fully stop it.
    pub fn bounce_enabled(&self, armor: Armor) -> bool {
        self.config.breasts.physics
            && (!self.chestplate_occupied(armor) || armor.resistance < 1.0)
    }

    /// `breathingAnimation`: the original animates only when the armour does
    /// not fully resist it.
    pub fn breathing(&self, armor: Armor, breathing: bool) -> bool {
        (self.armor_physics_override || armor.resistance <= 0.5) && breathing
    }
}

/// Everything one connection has learned, plus the settings that outlive it.
#[derive(Clone, Debug)]
pub struct State {
    pub config: AvatarConfig,
    pub client: ClientConfig,
    pub ui: Ui,
    /// The in-game character personalization screen (`H`).
    pub menu: Menu,
    pub loaded: bool,
    /// UUID -> avatar config, as `clientbound/sync` delivered it.
    pub remotes: HashMap<u128, AvatarConfig>,
    /// Entity id -> UUID for the players currently in the level.
    pub entity_uuids: HashMap<u32, u128>,
    pub own_uuid: Option<u128>,
    pub physics: HashMap<u32, BothBreasts>,
    /// The local config changed since the last `serverbound/sync`.
    pub dirty: bool,
    pub joined_synced: bool,
    pub registered_play: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            config: AvatarConfig::default(),
            client: ClientConfig::default(),
            ui: Ui::default(),
            menu: Menu::default(),
            loaded: false,
            remotes: HashMap::new(),
            entity_uuids: HashMap::new(),
            own_uuid: None,
            physics: HashMap::new(),
            dirty: false,
            joined_synced: false,
            registered_play: false,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets everything that belongs to one server connection, keeping the
    /// stored settings and the open settings page.
    pub fn reset_connection(&mut self) {
        self.remotes.clear();
        self.entity_uuids.clear();
        self.own_uuid = None;
        self.physics.clear();
        self.joined_synced = false;
        self.registered_play = false;
        self.dirty = false;
    }

    pub fn set_own(&mut self, uuid: u128) {
        self.own_uuid = Some(uuid);
        // The server never sends us our own config, but an old copy from a
        // previous session with the same UUID must not shadow the local one.
        self.remotes.remove(&uuid);
    }

    /// Replaces the player id -> UUID map. Physics for entities that left the
    /// level is dropped with it.
    pub fn set_entity_map(&mut self, players: impl IntoIterator<Item = (u32, u128)>) {
        self.entity_uuids = players.into_iter().collect();
        self.physics.retain(|id, _| *id == 0 || self.entity_uuids.contains_key(id));
    }

    /// What `fgm_remote::poll` has to send after this frame. Registration and
    /// the first sync happen once per connection; a local edit schedules
    /// another sync.
    pub fn play_actions(&mut self, joined: bool) -> PlayActions {
        let mut out = PlayActions::default();
        if !joined {
            return out;
        }
        if !self.registered_play {
            self.registered_play = true;
            out.register = true;
        }
        if self.dirty || !self.joined_synced {
            self.dirty = false;
            self.joined_synced = true;
            out.sync = true;
        }
        out
    }

    /// The writer was not ready (a reconfiguration started between the check
    /// and the write): undo the matching part of [`Self::play_actions`] so the
    /// next frame tries again.
    pub fn play_failed(&mut self, actions: PlayActions) {
        if actions.register {
            self.registered_play = false;
        }
        if actions.sync {
            self.joined_synced = false;
            self.dirty = true;
        }
    }

    /// A `clientbound/sync` packet. Our own UUID is ignored: the settings page
    /// and the local config are authoritative for ourselves.
    pub fn apply_remote(&mut self, uuid: u128, config: AvatarConfig) {
        if self.own_uuid == Some(uuid) {
            return;
        }
        self.remotes.insert(uuid, config);
        // A fresh config for an entity that is already mapped: its physics
        // keeps running, the render layer reads the new values next frame.
    }

    pub fn config_for_entity(&self, id: u32) -> Option<&AvatarConfig> {
        if id == 0 {
            return Some(&self.config);
        }
        self.entity_uuids
            .get(&id)
            .and_then(|uuid| self.remotes.get(uuid))
    }

    pub fn has_entity(&self, id: u32) -> bool {
        id == 0 || self.entity_uuids.contains_key(&id)
    }

    /// `BothBreastsPhysics.tick`: called once per tick for every player the
    /// client knows a config for. The physics runs even while the render layer
    /// is hidden, as in the original; the config gates only the drawing.
    pub fn tick_physics(&mut self, id: u32, motion: &EntityMotion, armor: Armor) {
        let Some(config) = self.config_for_entity(id).cloned() else {
            return;
        };
        let override_armor = self.client.armor_physics_override;
        let physics = self
            .physics
            .entry(id)
            .or_insert_with(|| BothBreasts::new(id));
        physics.update(motion, &config, armor, override_armor);
    }

    /// The interpolated values for `fgm_render`. `None` when the player has no
    /// config or its gender never draws breasts.
    pub fn render_info(&self, id: u32, partial: f32) -> Option<RenderInfo> {
        let config = self.config_for_entity(id)?.clone();
        if !config.gender.can_have_breasts() {
            return None;
        }
        let values = self
            .physics
            .get(&id)
            .map(|physics| physics.values(partial))
            .unwrap_or_default();
        Some(RenderInfo {
            config,
            values,
            disable_rendering: self.client.disable_rendering,
            armor_physics_override: self.client.armor_physics_override,
        })
    }
}

/// The process-wide state. Hooks run on the game thread; the network thread
/// only queues packets, so no hook holds this across a frame.
static STATE: Mutex<Option<State>> = Mutex::new(None);

fn lock() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

pub fn with<R>(f: impl FnOnce(&State) -> R) -> R {
    let mut guard = lock();
    let state = guard.get_or_insert_with(State::default);
    f(state)
}

pub fn with_mut<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = lock();
    let state = guard.get_or_insert_with(State::default);
    f(state)
}

/// The stored `config` setting, once. A missing key means first run: the
/// defaults stand and `first_time_load` stays true.
pub fn load(raw: Option<&str>) {
    with_mut(|state| {
        if state.loaded {
            return;
        }
        state.loaded = true;
        let Some(raw) = raw.filter(|text| !text.is_empty()) else {
            return;
        };
        let (config, client) = deserialize(raw);
        // A stored config that matches the defaults needs no sync beyond the
        // one every connection sends anyway.
        state.dirty |= config != AvatarConfig::default();
        state.config = config;
        state.client = client;
    });
}

pub fn save_text() -> String {
    with(|state| serialize(&state.config, &state.client))
}

pub fn loaded() -> bool {
    with(|state| state.loaded)
}

/// Clears the connection data when the player no longer is on a server (the
/// game keeps the `Remote` only while connected).
pub fn reset_if_active() {
    with_mut(|state| {
        if state.own_uuid.is_some()
            || state.joined_synced
            || state.registered_play
            || !state.entity_uuids.is_empty()
        {
            state.reset_connection();
        }
    });
}

/// A reconfiguration starts play again: the channels and the config have to
/// be sent a second time. A fresh connection has nothing to reset yet.
pub fn reset_if_joined() {
    with_mut(|state| {
        if state.joined_synced || state.registered_play {
            state.reset_connection();
        }
    });
}

pub fn labels() -> Vec<String> {
    with(|state| {
        state
            .ui
            .buttons(&state.config, &state.client)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    })
}

/// Applies one button of the open page. Returns true when a value changed, so
/// the caller knows to write the `config` key.
pub fn click(index: usize) -> bool {
    with_mut(|state| {
        let before = state.config.clone();
        let changed = state.ui.click(index, &mut state.config, &mut state.client);
        if state.config != before {
            state.dirty = true;
        }
        changed
    })
}

/// Asks the game for the character personalization screen on its next tick.
/// The key press that asked for it does not also reach the screen that opens.
pub fn menu_request_open() {
    with_mut(|state| state.menu.request_open());
}

pub fn menu_take_open_request() -> bool {
    with_mut(|state| state.menu.take_open_request())
}

pub fn menu_visible() -> bool {
    with(|state| state.menu.open)
}

/// The panel's widgets for the current tab at this GUI size, for drawing and
/// hit-testing.
pub fn menu_widgets(width: f32, height: f32) -> Vec<Widget> {
    with(|state| {
        state
            .menu
            .widgets(&state.config, &state.client, width, height)
    })
}

pub fn menu_open() {
    with_mut(|state| state.menu.show());
}

pub fn menu_close_requested() -> bool {
    with(|state| state.menu.close_requested)
}

/// The left button went down over the panel. True in `changed` means the
/// caller has to write the `config` key; `close` means the panel asked to
/// close.
pub fn menu_press(x: f32, y: f32, width: f32, height: f32) -> Outcome {
    with_mut(|state| {
        let before = state.config.clone();
        let before_client = state.client.clone();
        let outcome = state
            .menu
            .press(x, y, &mut state.config, &mut state.client, width, height);
        if state.config != before {
            state.dirty = true;
        }
        Outcome {
            changed: outcome.changed
                || state.config != before
                || state.client != before_client,
            close: outcome.close,
        }
    })
}

/// The pointer moved with the left button down. The values change in memory;
/// the next release or close marks them stored.
pub fn menu_drag(x: f32, width: f32, height: f32) -> bool {
    with_mut(|state| {
        state
            .menu
            .drag_at(x, &mut state.config, &mut state.client, width, height)
    })
}

/// The left button came up. True when the drag changed a stored value.
pub fn menu_release() -> bool {
    with_mut(|state| {
        let changed = state.menu.release();
        if changed {
            state.dirty = true;
        }
        changed
    })
}

/// A key over the panel. Every key is taken by the screen; this handles the
/// close keys and Tab.
pub fn menu_key(key: &str, close_key: &str) -> Outcome {
    with_mut(|state| {
        let outcome = state.menu.key(key, close_key);
        if outcome.changed {
            state.dirty = true;
        }
        outcome
    })
}

/// Closes the panel. True when a drag left values that were never stored.
pub fn menu_close() -> bool {
    with_mut(|state| {
        let changed = state.menu.hide();
        if changed {
            state.dirty = true;
        }
        changed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Gender;
    use crate::physics::MotionPose;

    fn motion() -> EntityMotion {
        EntityMotion {
            position: [0.0, 64.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            body_yaw: 0.0,
            previous_body_yaw: 0.0,
            limb_position: 0.0,
            limb_speed: 0.0,
            pose: MotionPose::Standing,
            swinging: false,
            swing_right_arm: true,
            age: 0,
        }
    }

    #[test]
    fn first_join_registers_and_syncs_once() {
        let mut state = State::new();
        assert_eq!(state.play_actions(false), PlayActions::default());
        let first = state.play_actions(true);
        assert_eq!(
            first,
            PlayActions {
                register: true,
                sync: true
            }
        );
        assert_eq!(state.play_actions(true), PlayActions::default());
        state.dirty = true;
        assert_eq!(
            state.play_actions(true),
            PlayActions {
                register: false,
                sync: true
            }
        );
        assert_eq!(state.play_actions(true), PlayActions::default());
    }

    #[test]
    fn reconnecting_starts_over() {
        let mut state = State::new();
        state.play_actions(true);
        state.reset_connection();
        assert_eq!(
            state.play_actions(true),
            PlayActions {
                register: true,
                sync: true
            }
        );
    }

    #[test]
    fn remote_config_is_mapped_through_the_entity_list() {
        let mut state = State::new();
        let mut config = AvatarConfig::default();
        config.gender = Gender::Female;
        config.breasts.bust_size = 0.5;
        state.set_entity_map([(7, 0xabc)]);
        state.apply_remote(0xabc, config.clone());
        assert_eq!(state.config_for_entity(7), Some(&config));
        assert!(state.has_entity(7));
        state.set_entity_map([(8, 0xdef)]);
        assert_eq!(state.config_for_entity(7), None);
        // The config itself is cached, as the Java mod's 15-minute cache does.
        state.set_entity_map([(7, 0xabc)]);
        assert_eq!(state.config_for_entity(7), Some(&config));
    }

    #[test]
    fn own_uuid_never_shadows_local_settings() {
        let mut state = State::new();
        state.set_own(0xabc);
        let mut remote = AvatarConfig::default();
        remote.gender = Gender::Female;
        state.apply_remote(0xabc, remote);
        assert_eq!(state.config_for_entity(0), Some(&state.config));
        assert!(state.remotes.is_empty());
    }

    #[test]
    fn render_gates_follow_armor_and_size() {
        let mut state = State::new();
        state.config.gender = Gender::Female;
        state.config.breasts.bust_size = 0.6;
        state.config.breasts.physics = true;
        state.tick_physics(0, &motion(), Armor::NONE);
        // The first tick only records the starting position; the values are
        // still the defaults and the size gate hides the layer.
        let info = state.render_info(0, 0.0).unwrap();
        assert!(!info.visible(Armor::NONE));
        for _ in 0..40 {
            state.tick_physics(0, &motion(), Armor::NONE);
        }
        let info = state.render_info(0, 0.0).unwrap();
        assert!(info.visible(Armor::NONE));
        assert!(info.values.left.breast_size > 0.0);
        let covering = Armor {
            covers: true,
            hides: false,
            resistance: 1.0,
            tightness: 1.0,
        };
        assert!(info.visible(covering));
        assert!(!info.bounce_enabled(covering));
        state.config.show_in_armor = false;
        let info = state.render_info(0, 0.0).unwrap();
        assert!(!info.visible(covering));
        state.client.armor_physics_override = true;
        let info = state.render_info(0, 0.0).unwrap();
        assert!(info.visible(covering));
        assert!(info.bounce_enabled(covering));
    }

    #[test]
    fn covering_chestplate_keeps_the_armor_layer_with_physics_override() {
        let mut state = State::new();
        state.config.gender = Gender::Female;
        state.config.breasts.bust_size = 0.6;
        state.config.breasts.physics = true;
        state.client.armor_physics_override = true;
        let covering = Armor {
            covers: true,
            hides: false,
            resistance: 1.0,
            tightness: 1.0,
        };
        for _ in 0..40 {
            state.tick_physics(0, &motion(), covering);
        }
        let info = state.render_info(0, 0.0).unwrap();
        assert!(info.visible(covering));
        // The override disables the armour's physics effect...
        assert!(!info.chestplate_occupied(covering));
        // ...but the worn chestplate still covers the layer visually.
        assert!(info.armor_layer_visible(covering));
        // Without a covering chestplate the skin layer is drawn.
        assert!(!info.armor_layer_visible(Armor::NONE));
    }

    #[test]
    fn male_and_disabled_rendering_hide_the_layer() {
        let mut state = State::new();
        assert!(state.render_info(0, 0.0).is_none());
        state.config.gender = Gender::Female;
        state.client.disable_rendering = true;
        let info = state.render_info(0, 0.0).unwrap();
        assert!(!info.visible(Armor::NONE));
    }
}
