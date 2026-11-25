//! The in-game character personalization screen (the open-menu key, `H` by
//! default): the Fabric mod's three customization tabs with its sliders and
//! toggles.
//!
//! Horizon has no custom screen of its own for native mods, so the game side
//! draws this over the Options screen and takes its input. This module is the
//! model and the layout only: which widget a tab has, where it is, and what a
//! click, a drag or a key changes in the stored config.

use crate::config::{AvatarConfig, ClientConfig};

/// The original `female_gender_mod.appearance_settings.title`.
pub const TITLE: &str = "Character Personalization";
pub const PANEL_WIDTH: f32 = 272.0;
pub const PANEL_HEIGHT: f32 = 182.0;

/// One tab of the customization screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Customization,
    Physics,
    Misc,
}

impl Tab {
    pub fn next(self) -> Self {
        match self {
            Self::Customization => Self::Physics,
            Self::Physics => Self::Misc,
            Self::Misc => Self::Customization,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Customization => "Customization",
            Self::Physics => "Breast Physics",
            Self::Misc => "Miscellaneous",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Customization => 0,
            Self::Physics => 1,
            Self::Misc => 2,
        }
    }
}

/// Every slider the three tabs show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderId {
    Bust,
    Separation,
    Height,
    Depth,
    Rotation,
    Bounce,
    Floppiness,
    Pitch,
}

impl SliderId {
    /// The range the original `ConfigKey` declares.
    pub fn range(self) -> (f32, f32) {
        match self {
            Self::Bust => (0.0, 0.8),
            Self::Separation | Self::Height => (-1.0, 1.0),
            Self::Depth => (-1.0, 0.0),
            Self::Rotation => (0.0, 0.1),
            Self::Bounce => (0.0, 0.5),
            Self::Floppiness => (0.25, 1.0),
            Self::Pitch => (0.8, 1.2),
        }
    }

    /// The value one pixel of the track moves, in the config's units.
    fn step(self) -> f32 {
        match self {
            Self::Bust => 0.001,
            Self::Separation | Self::Height | Self::Depth => 0.05,
            Self::Rotation => 0.01,
            Self::Bounce => 0.005,
            Self::Floppiness | Self::Pitch => 0.01,
        }
    }

    pub fn value(self, config: &AvatarConfig, client: &ClientConfig) -> f32 {
        let _ = client;
        match self {
            Self::Bust => config.breasts.bust_size,
            Self::Separation => config.breasts.x_offset,
            Self::Height => config.breasts.y_offset,
            Self::Depth => config.breasts.z_offset,
            Self::Rotation => config.breasts.cleavage,
            Self::Bounce => config.breasts.bounce_multiplier,
            Self::Floppiness => config.breasts.floppiness,
            Self::Pitch => config.sounds.voice_pitch,
        }
    }

    pub fn fraction(self, config: &AvatarConfig, client: &ClientConfig) -> f32 {
        let (min, max) = self.range();
        ((self.value(config, client) - min) / (max - min)).clamp(0.0, 1.0)
    }

    fn set_fraction(
        self,
        fraction: f32,
        config: &mut AvatarConfig,
        _client: &mut ClientConfig,
    ) -> bool {
        let (min, max) = self.range();
        let raw = min + fraction.clamp(0.0, 1.0) * (max - min);
        let snapped = ((raw / self.step()).round() * self.step()).clamp(min, max);
        let slot = match self {
            Self::Bust => &mut config.breasts.bust_size,
            Self::Separation => &mut config.breasts.x_offset,
            Self::Height => &mut config.breasts.y_offset,
            Self::Depth => &mut config.breasts.z_offset,
            Self::Rotation => &mut config.breasts.cleavage,
            Self::Bounce => &mut config.breasts.bounce_multiplier,
            Self::Floppiness => &mut config.breasts.floppiness,
            Self::Pitch => &mut config.sounds.voice_pitch,
        };
        assign(slot, snapped)
    }

    /// The original slider message (`WardrobeSlider…`, `Slider…`).
    pub fn label(self, config: &AvatarConfig, client: &ClientConfig) -> String {
        match self {
            Self::Bust => format!(
                "Breast Size: {}%",
                (config.breasts.bust_size * 125.0).round() as i32
            ),
            Self::Separation => format!(
                "Separation: {}",
                (config.breasts.x_offset * 10.0).round() as i32
            ),
            Self::Height => format!(
                "Height: {}",
                (config.breasts.y_offset * 10.0).round() as i32
            ),
            Self::Depth => format!(
                "Depth: {}",
                (config.breasts.z_offset * 10.0).round() as i32
            ),
            Self::Rotation => format!(
                "Rotation: {}°",
                (config.breasts.cleavage * 100.0).round() as i32
            ),
            Self::Bounce => format!(
                "Intensity: {}%",
                (config.breasts.bounce_multiplier * 300.0).round() as i32
            ),
            Self::Floppiness => format!(
                "Momentum: {}%",
                (config.breasts.floppiness * 100.0).round() as i32
            ),
            Self::Pitch => {
                let _ = client;
                format!("Pitch: {}%", (config.sounds.voice_pitch * 100.0).round() as i32)
            }
        }
    }

    /// The original disables the physics sliders while physics is off and the
    /// pitch slider while hurt sounds are off.
    fn enabled(self, config: &AvatarConfig) -> bool {
        match self {
            Self::Bounce | Self::Floppiness => config.breasts.physics,
            Self::Pitch => config.sounds.hurt,
            _ => true,
        }
    }
}

/// Every toggle the three tabs show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToggleId {
    Physics,
    /// The original's `Dual-Physics`, the inverse of `breasts.uniboob`.
    DualPhysics,
    ArmorPhysics,
    HurtSounds,
    /// `Hide In Armor`, the inverse of `show_in_armor`.
    HideInArmor,
    ArmorStat,
}

impl ToggleId {
    pub fn value(self, config: &AvatarConfig, client: &ClientConfig) -> bool {
        match self {
            Self::Physics => config.breasts.physics,
            Self::DualPhysics => !config.breasts.uniboob,
            Self::ArmorPhysics => client.armor_physics_override,
            Self::HurtSounds => config.sounds.hurt,
            Self::HideInArmor => !config.show_in_armor,
            Self::ArmorStat => client.armor_stat,
        }
    }

    fn set(self, on: bool, config: &mut AvatarConfig, client: &mut ClientConfig) -> bool {
        let slot = match self {
            Self::Physics => &mut config.breasts.physics,
            Self::DualPhysics => &mut config.breasts.uniboob,
            Self::ArmorPhysics => &mut client.armor_physics_override,
            Self::HurtSounds => &mut config.sounds.hurt,
            Self::HideInArmor => &mut config.show_in_armor,
            Self::ArmorStat => &mut client.armor_stat,
        };
        let value = match self {
            Self::DualPhysics | Self::HideInArmor => !on,
            _ => on,
        };
        if *slot == value {
            return false;
        }
        *slot = value;
        true
    }

    fn enabled(self, config: &AvatarConfig) -> bool {
        match self {
            Self::DualPhysics | Self::ArmorPhysics => config.breasts.physics,
            _ => true,
        }
    }

    pub fn label(self, config: &AvatarConfig, client: &ClientConfig) -> String {
        match self {
            Self::Physics => format!(
                "Breast Physics: {}",
                enabled(ToggleId::Physics.value(config, client))
            ),
            Self::DualPhysics => format!(
                "Dual-Physics: {}",
                yes_no(ToggleId::DualPhysics.value(config, client))
            ),
            Self::ArmorPhysics => format!(
                "Armor Physics: {}",
                enabled(ToggleId::ArmorPhysics.value(config, client))
            ),
            Self::HurtSounds => format!(
                "Female Hurt Sounds: {}",
                enabled(ToggleId::HurtSounds.value(config, client))
            ),
            Self::HideInArmor => format!(
                "Hide In Armor: {}",
                enabled(ToggleId::HideInArmor.value(config, client))
            ),
            Self::ArmorStat => format!(
                "Show Armor Tooltip: {}",
                enabled(ToggleId::ArmorStat.value(config, client))
            ),
        }
    }
}

/// What a widget is and what clicking it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
    Tab(Tab),
    Slider(SliderId),
    Toggle(ToggleId),
    Done,
}

/// One clickable row as the game draws and hit-tests it.
#[derive(Clone, Debug)]
pub struct Widget {
    pub kind: WidgetKind,
    /// `[x, y, width, height]` in GUI pixels.
    pub rect: [f32; 4],
    pub label: String,
    pub enabled: bool,
    /// A toggle's value; false for other widgets. The label already carries
    /// it, but the game colours an on toggle differently.
    pub on: bool,
    /// The slider's handle position, 0 to 1; zero for other widgets.
    pub fraction: f32,
}

impl Widget {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let [rx, ry, rw, rh] = self.rect;
        x >= rx && y >= ry && x < rx + rw && y < ry + rh
    }
}

/// The result of a click or a key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// A stored value changed; the caller has to save and sync.
    pub changed: bool,
    /// The screen should close.
    pub close: bool,
}

/// The customization screen: its tab, whether it is open, and the slider being
/// dragged. It outlives an opening, so the player comes back to the same tab.
#[derive(Clone, Debug)]
pub struct Menu {
    pub tab: Tab,
    pub open: bool,
    pub close_requested: bool,
    open_requested: bool,
    drag: Option<SliderId>,
    drag_changed: bool,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            tab: Tab::Customization,
            open: false,
            close_requested: false,
            open_requested: false,
            drag: None,
            drag_changed: false,
        }
    }
}

impl Menu {
    /// The panel's rectangle in GUI pixels, centered in the window.
    pub fn panel(width: f32, height: f32) -> [f32; 4] {
        let x = ((width - PANEL_WIDTH) * 0.5).round().max(0.0);
        let y = ((height - PANEL_HEIGHT) * 0.5).round().max(0.0);
        [x, y, PANEL_WIDTH, PANEL_HEIGHT]
    }

    /// The widgets of the current tab, in the order they are drawn.
    pub fn widgets(
        &self,
        config: &AvatarConfig,
        client: &ClientConfig,
        width: f32,
        height: f32,
    ) -> Vec<Widget> {
        let panel = Self::panel(width, height);
        let mut out = Vec::with_capacity(12);
        let tab_width = 84.0;
        let tab_gap = 4.0;
        let tabs_width = tab_width * 3.0 + tab_gap * 2.0;
        let tabs_x = panel[0] + (panel[2] - tabs_width) * 0.5;
        for (index, tab) in [Tab::Customization, Tab::Physics, Tab::Misc]
            .into_iter()
            .enumerate()
        {
            out.push(Widget {
                kind: WidgetKind::Tab(tab),
                rect: [
                    tabs_x + index as f32 * (tab_width + tab_gap),
                    panel[1] + 28.0,
                    tab_width,
                    20.0,
                ],
                label: tab.label().into(),
                // The original leaves the current tab's button inactive.
                enabled: tab != self.tab,
                on: false,
                fraction: 0.0,
            });
        }

        let left = panel[0] + 10.0;
        let width = panel[2] - 20.0;
        let half = (width - 4.0) * 0.5;
        let right = left + half + 4.0;
        let row = |index: usize| panel[1] + 58.0 + index as f32 * 24.0;
        let slider = |id: SliderId, rect: [f32; 4]| Widget {
            kind: WidgetKind::Slider(id),
            rect,
            label: id.label(config, client),
            enabled: id.enabled(config),
            on: false,
            fraction: id.fraction(config, client),
        };
        let toggle = |id: ToggleId, rect: [f32; 4]| Widget {
            kind: WidgetKind::Toggle(id),
            rect,
            label: id.label(config, client),
            enabled: id.enabled(config),
            on: id.value(config, client),
            fraction: 0.0,
        };
        match self.tab {
            Tab::Customization => {
                out.push(slider(SliderId::Bust, [left, row(0), width, 20.0]));
                out.push(slider(SliderId::Separation, [left, row(1), half, 20.0]));
                out.push(slider(SliderId::Height, [right, row(1), half, 20.0]));
                out.push(slider(SliderId::Depth, [left, row(2), half, 20.0]));
                out.push(slider(SliderId::Rotation, [right, row(2), half, 20.0]));
            }
            Tab::Physics => {
                out.push(toggle(ToggleId::Physics, [left, row(0), width, 20.0]));
                out.push(toggle(ToggleId::DualPhysics, [left, row(1), width, 20.0]));
                out.push(slider(SliderId::Bounce, [left, row(2), half, 20.0]));
                out.push(slider(SliderId::Floppiness, [right, row(2), half, 20.0]));
                out.push(toggle(
                    ToggleId::ArmorPhysics,
                    [left, row(3), width, 20.0],
                ));
            }
            Tab::Misc => {
                out.push(toggle(ToggleId::HurtSounds, [left, row(0), width, 20.0]));
                out.push(slider(SliderId::Pitch, [left, row(1), half, 20.0]));
                out.push(toggle(ToggleId::HideInArmor, [left, row(2), width, 20.0]));
                out.push(toggle(ToggleId::ArmorStat, [left, row(3), width, 20.0]));
            }
        }
        out.push(Widget {
            kind: WidgetKind::Done,
            rect: [
                panel[0] + panel[2] - 10.0 - 64.0,
                panel[1] + 156.0,
                64.0,
                20.0,
            ],
            label: "Done".into(),
            enabled: true,
            on: false,
            fraction: 0.0,
        });
        out
    }

    /// Asks the game to open the screen on its next tick. The key that asked
    /// for it therefore does not also reach the Options screen that opens.
    pub fn request_open(&mut self) {
        self.open_requested = true;
    }

    pub fn take_open_request(&mut self) -> bool {
        std::mem::take(&mut self.open_requested)
    }

    /// The game opened the Options screen for this panel.
    pub fn show(&mut self) {
        self.open = true;
        self.close_requested = false;
        self.drag = None;
        self.drag_changed = false;
    }

    /// Forgets the panel. True when a drag left values that were never saved.
    pub fn hide(&mut self) -> bool {
        self.open = false;
        self.close_requested = false;
        self.drag = None;
        std::mem::take(&mut self.drag_changed)
    }

    /// The left button went down. Starts a drag, flips a toggle, switches the
    /// tab or asks to close.
    pub fn press(
        &mut self,
        x: f32,
        y: f32,
        config: &mut AvatarConfig,
        client: &mut ClientConfig,
        width: f32,
        height: f32,
    ) -> Outcome {
        let mut changed = self.end_drag();
        if !self.open {
            return Outcome { changed, close: false };
        }
        let Some(widget) = self
            .widgets(config, client, width, height)
            .into_iter()
            .find(|widget| widget.contains(x, y))
        else {
            return Outcome { changed, close: false };
        };
        if !widget.enabled {
            return Outcome { changed, close: false };
        }
        match widget.kind {
            WidgetKind::Tab(tab) => self.tab = tab,
            WidgetKind::Done => {
                self.close_requested = true;
                return Outcome {
                    changed,
                    close: true,
                };
            }
            WidgetKind::Toggle(id) => {
                let on = !id.value(config, client);
                if id.set(on, config, client) {
                    changed = true;
                }
            }
            WidgetKind::Slider(id) => {
                self.drag = Some(id);
                if id.set_fraction(fraction_at(widget.rect, x), config, client) {
                    self.drag_changed = true;
                    changed = true;
                }
            }
        }
        Outcome {
            changed,
            close: self.close_requested,
        }
    }

    /// The pointer moved with the left button down. The value is changed but
    /// not yet marked for saving; [`Self::release`] or [`Self::hide`] is.
    pub fn drag_at(
        &mut self,
        x: f32,
        config: &mut AvatarConfig,
        client: &mut ClientConfig,
        width: f32,
        height: f32,
    ) -> bool {
        let Some(id) = self.drag else {
            return false;
        };
        let rect = self
            .widgets(config, client, width, height)
            .into_iter()
            .find(|widget| widget.kind == WidgetKind::Slider(id))
            .map(|widget| widget.rect);
        let Some(rect) = rect else {
            return false;
        };
        let changed = id.set_fraction(fraction_at(rect, x), config, client);
        self.drag_changed |= changed;
        changed
    }

    /// The left button came up. True when the drag changed a stored value.
    pub fn release(&mut self) -> bool {
        self.drag = None;
        std::mem::take(&mut self.drag_changed)
    }

    /// A key over the panel: Escape and the open-menu key close it, Tab moves
    /// to the next tab. Every key is taken by the screen, whatever this does.
    pub fn key(&mut self, key: &str, close_key: &str) -> Outcome {
        if !self.open {
            return Outcome::default();
        }
        if key == "key.keyboard.escape" || key == close_key {
            let changed = self.end_drag();
            self.close_requested = true;
            return Outcome {
                changed,
                close: true,
            };
        }
        if key == "key.keyboard.tab" {
            let changed = self.end_drag();
            self.tab = self.tab.next();
            return Outcome {
                changed,
                close: false,
            };
        }
        Outcome::default()
    }

    fn end_drag(&mut self) -> bool {
        self.drag = None;
        std::mem::take(&mut self.drag_changed)
    }
}

fn enabled(value: bool) -> &'static str {
    if value { "Enabled" } else { "Disabled" }
}

fn yes_no(value: bool) -> &'static str {
    if value { "Yes" } else { "No" }
}

fn assign(slot: &mut f32, value: f32) -> bool {
    if (*slot - value).abs() > 1e-6 {
        *slot = value;
        true
    } else {
        false
    }
}

/// The slider's value from a pointer at `x`, as vanilla's
/// `AbstractSliderButton` reads it: the track is the widget less four pixels
/// at either end.
fn fraction_at(rect: [f32; 4], x: f32) -> f32 {
    ((x - rect[0] - 4.0) / (rect[2] - 8.0)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Gender;

    fn default_state() -> (Menu, AvatarConfig, ClientConfig) {
        let mut menu = Menu::default();
        menu.show();
        (menu, AvatarConfig::default(), ClientConfig::default())
    }

    fn widget<'a>(widgets: &'a [Widget], kind: WidgetKind) -> &'a Widget {
        widgets
            .iter()
            .find(|widget| widget.kind == kind)
            .unwrap_or_else(|| panic!("no {kind:?} widget"))
    }

    #[test]
    fn each_tab_has_the_original_rows() {
        let (mut menu, config, client) = default_state();
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        assert_eq!(widgets.len(), 3 + 5 + 1);
        assert!(widgets.iter().any(|w| w.kind == WidgetKind::Slider(SliderId::Bust)));

        menu.tab = Tab::Physics;
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        assert!(widgets.iter().any(|w| w.kind == WidgetKind::Toggle(ToggleId::Physics)));
        assert!(widgets.iter().any(|w| w.kind == WidgetKind::Slider(SliderId::Bounce)));

        menu.tab = Tab::Misc;
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        assert!(widgets.iter().any(|w| w.kind == WidgetKind::Toggle(ToggleId::HurtSounds)));
        assert!(widgets.iter().any(|w| w.kind == WidgetKind::Slider(SliderId::Pitch)));
    }

    #[test]
    fn slider_labels_match_the_original_sliders() {
        let config = AvatarConfig {
            breasts: crate::config::Breasts {
                bust_size: 0.5,
                x_offset: 0.5,
                y_offset: -0.25,
                z_offset: -0.5,
                cleavage: 0.05,
                bounce_multiplier: 0.25,
                floppiness: 0.5,
                ..Default::default()
            },
            sounds: crate::config::Sounds {
                hurt: true,
                voice_pitch: 1.1,
            },
            ..Default::default()
        };
        let client = ClientConfig::default();
        assert_eq!(SliderId::Bust.label(&config, &client), "Breast Size: 63%");
        assert_eq!(SliderId::Separation.label(&config, &client), "Separation: 5");
        assert_eq!(SliderId::Height.label(&config, &client), "Height: -3");
        assert_eq!(SliderId::Depth.label(&config, &client), "Depth: -5");
        assert_eq!(SliderId::Rotation.label(&config, &client), "Rotation: 5°");
        assert_eq!(SliderId::Bounce.label(&config, &client), "Intensity: 75%");
        assert_eq!(SliderId::Floppiness.label(&config, &client), "Momentum: 50%");
        assert_eq!(SliderId::Pitch.label(&config, &client), "Pitch: 110%");
    }

    #[test]
    fn dragging_a_slider_sets_its_value_and_clamps_at_the_ends() {
        let (mut menu, mut config, mut client) = default_state();
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let rect = widget(&widgets, WidgetKind::Slider(SliderId::Bust)).rect;
        let outcome = menu.press(rect[0] + 4.0, rect[1] + 10.0, &mut config, &mut client, 320.0, 240.0);
        assert!(outcome.changed);
        assert_eq!(config.breasts.bust_size, 0.0);
        assert!(menu.drag_at(rect[0] + rect[2] + 100.0, &mut config, &mut client, 320.0, 240.0));
        assert_eq!(config.breasts.bust_size, 0.8);
        assert!(menu.release());
        assert!(!menu.release());
    }

    #[test]
    fn a_click_on_the_track_jumps_to_the_value() {
        let (mut menu, mut config, mut client) = default_state();
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let rect = widget(&widgets, WidgetKind::Slider(SliderId::Separation)).rect;
        menu.press(rect[0] + 4.0 + (rect[2] - 8.0) * 0.75, rect[1] + 10.0, &mut config, &mut client, 320.0, 240.0);
        // 75% of -1..1 is 0.5.
        assert!((config.breasts.x_offset - 0.5).abs() < 1e-5);
    }

    #[test]
    fn disabled_physics_rows_do_not_change() {
        let (mut menu, mut config, mut client) = default_state();
        config.breasts.physics = false;
        menu.tab = Tab::Physics;
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let dual = widget(&widgets, WidgetKind::Toggle(ToggleId::DualPhysics));
        assert!(!dual.enabled);
        let before = config.breasts.uniboob;
        menu.press(dual.rect[0] + 2.0, dual.rect[1] + 2.0, &mut config, &mut client, 320.0, 240.0);
        assert_eq!(config.breasts.uniboob, before);
    }

    #[test]
    fn toggles_keep_the_originals_inverted_labels() {
        let (mut menu, mut config, mut client) = default_state();
        menu.tab = Tab::Misc;
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let hide = widget(&widgets, WidgetKind::Toggle(ToggleId::HideInArmor));
        assert_eq!(hide.label, "Hide In Armor: Disabled");
        // Clicking turns `show_in_armor` off, so the label reads Enabled.
        menu.press(hide.rect[0] + 2.0, hide.rect[1] + 2.0, &mut config, &mut client, 320.0, 240.0);
        assert!(!config.show_in_armor);
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        assert_eq!(
            widget(&widgets, WidgetKind::Toggle(ToggleId::HideInArmor)).label,
            "Hide In Armor: Enabled"
        );

        // The default config is a uniboob, so Dual-Physics reads No.
        let dual = ToggleId::DualPhysics;
        assert!(!dual.value(&config, &client));
        assert_eq!(dual.label(&config, &client), "Dual-Physics: No");
    }

    #[test]
    fn tabs_and_done_use_the_press_path() {
        let (mut menu, mut config, mut client) = default_state();
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let physics = widget(&widgets, WidgetKind::Tab(Tab::Physics));
        menu.press(physics.rect[0] + 2.0, physics.rect[1] + 2.0, &mut config, &mut client, 320.0, 240.0);
        assert_eq!(menu.tab, Tab::Physics);
        let widgets = menu.widgets(&config, &client, 320.0, 240.0);
        let done = widget(&widgets, WidgetKind::Done);
        let outcome = menu.press(done.rect[0] + 2.0, done.rect[1] + 2.0, &mut config, &mut client, 320.0, 240.0);
        assert!(outcome.close);
        assert!(menu.close_requested);
    }

    #[test]
    fn the_panel_is_centered_and_the_widgets_stay_inside_it() {
        let (menu, config, client) = default_state();
        let panel = Menu::panel(320.0, 240.0);
        assert_eq!(panel, [24.0, 29.0, PANEL_WIDTH, PANEL_HEIGHT]);
        for widget in menu.widgets(&config, &client, 320.0, 240.0) {
            let [x, y, w, h] = widget.rect;
            assert!(x >= panel[0] && y >= panel[1]);
            assert!(x + w <= panel[0] + panel[2]);
            assert!(y + h <= panel[1] + panel[3]);
        }
    }

    #[test]
    fn escape_and_the_bound_key_close_the_screen() {
        let (mut menu, _, _) = default_state();
        assert!(menu.key("key.keyboard.escape", "key.keyboard.h").close);
        menu.show();
        assert!(menu.key("key.keyboard.h", "key.keyboard.h").close);
        menu.show();
        assert!(!menu.key("key.keyboard.e", "key.keyboard.h").close);
    }

    #[test]
    fn a_female_size_is_drawn_on_the_bust_slider() {
        let with_gender = |gender, bust_size| AvatarConfig {
            gender,
            breasts: crate::config::Breasts {
                bust_size,
                ..Default::default()
            },
            ..Default::default()
        };
        let client = ClientConfig::default();
        let female = with_gender(Gender::Female, 0.6);
        assert_eq!(SliderId::Bust.label(&female, &client), "Breast Size: 75%");
        assert!((SliderId::Bust.fraction(&female, &client) - 0.75).abs() < 1e-6);
    }
}
