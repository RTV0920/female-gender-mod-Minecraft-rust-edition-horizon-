//! The mod's settings page for Horizon's Options > Mods list.
//!
//! Horizon shows a native mod's settings as a flat list of buttons and reports
//! clicks by index. The Fabric mod has three tabs, a UV editor and several
//! client menus; this module keeps the same values behind a small page stack.
//! Every numeric setting offers `(-)` and `(+)` buttons, so a click never has
//! to wrap through a whole range.

use crate::config::{
    AvatarConfig, ClientConfig, LAYOUT_NAMES, UvQuad, UV_DIRECTION_SHORT, default_layout,
    default_uvs,
};

/// One page of the settings UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Root,
    Appearance,
    Physics,
    Sounds,
    Armor,
    Client,
    Cloud,
    Reset,
    Layouts,
    Layout(usize),
    Face { layout: usize, direction: usize },
}

/// What a click on one button does.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Open(Page),
    Back,
    Gender,
    Bust(f32),
    Cleavage(f32),
    OffsetX(f32),
    OffsetY(f32),
    OffsetZ(f32),
    Physics,
    Uniboob,
    Bounce(f32),
    Floppiness(f32),
    Hurt,
    Pitch(f32),
    ShowInArmor,
    ArmorPhysics,
    ArmorStat,
    DisableRendering,
    DisableSoundReplacement,
    ShowToast,
    FirstTime,
    PlayerList,
    Contributor,
    CloudEnabled,
    CloudAutomatic,
    CloudServer,
    CloudVerbosity,
    UvCoord {
        direction: usize,
        field: usize,
        delta: i32,
    },
    UvVisible {
        direction: usize,
    },
    UvResetFace {
        direction: usize,
    },
    UvResetLayout,
    UvResetAll,
    ResetAppearance,
    ResetPhysics,
    ResetSounds,
    ResetArmor,
    ResetClient,
    ResetAll,
    None,
}

/// The page stack of the settings UI. It outlives a single opening of the
/// page, so the player comes back to the sub-page they left.
#[derive(Clone, Debug)]
pub struct Ui {
    stack: Vec<Page>,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            stack: vec![Page::Root],
        }
    }
}

impl Ui {
    pub fn page(&self) -> Page {
        self.stack.last().copied().unwrap_or(Page::Root)
    }

    /// The buttons of the current page, as labels and actions. The click
    /// handler asks the same function, so an index can never name a different
    /// row than the one the player sees.
    pub fn buttons(&self, config: &AvatarConfig, client: &ClientConfig) -> Vec<(String, Action)> {
        let mut out = Vec::new();
        match self.page() {
            Page::Root => {
                out.push((
                    format!("Gender: {}", config.gender.label()),
                    Action::Gender,
                ));
                out.push((
                    format!("Bust Size: {}%  >", bust_percent(config)),
                    Action::Open(Page::Appearance),
                ));
                out.push(("Appearance  >".into(), Action::Open(Page::Appearance)));
                out.push((
                    "Breast Physics  >".into(),
                    Action::Open(Page::Physics),
                ));
                out.push(("Sounds  >".into(), Action::Open(Page::Sounds)));
                out.push(("Armor  >".into(), Action::Open(Page::Armor)));
                out.push(("UV Layouts  >".into(), Action::Open(Page::Layouts)));
                out.push(("Client Settings  >".into(), Action::Open(Page::Client)));
                out.push(("Cloud Sync  >".into(), Action::Open(Page::Cloud)));
                out.push(("Reset  >".into(), Action::Open(Page::Reset)));
                out.push((
                    format!(
                        "{} {} (LGPL-3.0, port of the Fabric mod)",
                        crate::TITLE,
                        crate::VERSION
                    ),
                    Action::None,
                ));
            }
            Page::Appearance => {
                back(&mut out);
                out.push((
                    format!("Bust Size: {}%  (-)", bust_percent(config)),
                    Action::Bust(-0.01),
                ));
                out.push((
                    format!("Bust Size: {}%  (+)", bust_percent(config)),
                    Action::Bust(0.01),
                ));
                out.push((
                    format!("Rotation (cleavage): {}°  (-)", cleavage_degrees(config)),
                    Action::Cleavage(-0.1),
                ));
                out.push((
                    format!("Rotation (cleavage): {}°  (+)", cleavage_degrees(config)),
                    Action::Cleavage(0.1),
                ));
                out.push((
                    format!("Separation (X): {:+.2}  (-)", config.breasts.x_offset),
                    Action::OffsetX(-0.05),
                ));
                out.push((
                    format!("Separation (X): {:+.2}  (+)", config.breasts.x_offset),
                    Action::OffsetX(0.05),
                ));
                out.push((
                    format!("Height (Y): {:+.2}  (-)", config.breasts.y_offset),
                    Action::OffsetY(-0.05),
                ));
                out.push((
                    format!("Height (Y): {:+.2}  (+)", config.breasts.y_offset),
                    Action::OffsetY(0.05),
                ));
                out.push((
                    format!("Depth (Z): {:+.2}  (-)", config.breasts.z_offset),
                    Action::OffsetZ(-0.05),
                ));
                out.push((
                    format!("Depth (Z): {:+.2}  (+)", config.breasts.z_offset),
                    Action::OffsetZ(0.05),
                ));
                out.push((
                    "Reset Appearance".into(),
                    Action::ResetAppearance,
                ));
            }
            Page::Physics => {
                back(&mut out);
                out.push((
                    format!("Breast Physics: {}", enabled(config.breasts.physics)),
                    Action::Physics,
                ));
                out.push((
                    format!("Uniboob (single physics): {}", enabled(config.breasts.uniboob)),
                    Action::Uniboob,
                ));
                out.push((
                    format!("Intensity: {}%  (-)", bounce_percent(config)),
                    Action::Bounce(-0.005),
                ));
                out.push((
                    format!("Intensity: {}%  (+)", bounce_percent(config)),
                    Action::Bounce(0.005),
                ));
                out.push((
                    format!("Momentum: {}%  (-)", floppiness_percent(config)),
                    Action::Floppiness(-0.01),
                ));
                out.push((
                    format!("Momentum: {}%  (+)", floppiness_percent(config)),
                    Action::Floppiness(0.01),
                ));
                out.push(("Reset Physics".into(), Action::ResetPhysics));
            }
            Page::Sounds => {
                back(&mut out);
                out.push((
                    format!("Female Hurt Sounds: {}", enabled(config.sounds.hurt)),
                    Action::Hurt,
                ));
                out.push((
                    format!("Pitch: {}%  (-)", pitch_percent(config)),
                    Action::Pitch(-0.01),
                ));
                out.push((
                    format!("Pitch: {}%  (+)", pitch_percent(config)),
                    Action::Pitch(0.01),
                ));
                out.push((
                    format!(
                        "Disable Sound Replacement: {}",
                        enabled(client.disable_sound_replacement)
                    ),
                    Action::DisableSoundReplacement,
                ));
                out.push(("Reset Sounds".into(), Action::ResetSounds));
            }
            Page::Armor => {
                back(&mut out);
                out.push((
                    format!(
                        "Show Breasts In Armor: {}",
                        enabled(config.show_in_armor)
                    ),
                    Action::ShowInArmor,
                ));
                out.push((
                    format!(
                        "Armor Physics Override: {}",
                        enabled(client.armor_physics_override)
                    ),
                    Action::ArmorPhysics,
                ));
                out.push((
                    format!("Show Armor Tooltip: {}", enabled(client.armor_stat)),
                    Action::ArmorStat,
                ));
                out.push(("Reset Armor".into(), Action::ResetArmor));
            }
            Page::Client => {
                back(&mut out);
                out.push((
                    format!(
                        "Disable Rendering: {}",
                        enabled(client.disable_rendering)
                    ),
                    Action::DisableRendering,
                ));
                out.push((
                    format!("Show Sync Toast: {}", enabled(client.show_toast)),
                    Action::ShowToast,
                ));
                out.push((
                    format!(
                        "First-Time Setup Pending: {}",
                        enabled(client.first_time_load)
                    ),
                    Action::FirstTime,
                ));
                out.push((
                    format!("Player List: {}", client.player_list_mode.label()),
                    Action::PlayerList,
                ));
                out.push((
                    format!(
                        "Hide Own Contributor Tag: {}",
                        enabled(client.hide_own_contributor_tag)
                    ),
                    Action::Contributor,
                ));
                out.push((
                    format!(
                        "Disable Sound Replacement: {}",
                        enabled(client.disable_sound_replacement)
                    ),
                    Action::DisableSoundReplacement,
                ));
                out.push(("Reset Client Settings".into(), Action::ResetClient));
            }
            Page::Cloud => {
                back(&mut out);
                out.push((
                    format!("Cloud Sync: {}", enabled(client.cloud_enabled)),
                    Action::CloudEnabled,
                ));
                out.push((
                    format!(
                        "Automatic Sync: {}",
                        enabled(client.cloud_automatic)
                    ),
                    Action::CloudAutomatic,
                ));
                out.push((
                    if client.cloud_server.is_empty() {
                        "Cloud Server: default (wfgm.celestialfault.dev/v2)".into()
                    } else {
                        format!("Cloud Server: {}", client.cloud_server)
                    },
                    Action::CloudServer,
                ));
                out.push((
                    format!("Log Verbosity: {}", client.cloud_log_verbosity.label()),
                    Action::CloudVerbosity,
                ));
                out.push((
                    "Sync Now (unavailable: this port does not speak the cloud API)".into(),
                    Action::None,
                ));
                out.push((
                    "Delete Cloud Profile (unavailable)".into(),
                    Action::None,
                ));
            }
            Page::Reset => {
                back(&mut out);
                out.push(("Reset Appearance".into(), Action::ResetAppearance));
                out.push(("Reset Physics".into(), Action::ResetPhysics));
                out.push(("Reset Sounds".into(), Action::ResetSounds));
                out.push(("Reset Armor".into(), Action::ResetArmor));
                out.push(("Reset UV Layouts".into(), Action::UvResetAll));
                out.push(("Reset Client Settings".into(), Action::ResetClient));
                out.push(("Reset Everything (including gender)".into(), Action::ResetAll));
            }
            Page::Layouts => {
                back(&mut out);
                for (index, name) in LAYOUT_NAMES.iter().enumerate() {
                    let faces = config.uvs[index]
                        .iter()
                        .filter(|quad| quad.is_some_and(|quad| !quad.is_unused()))
                        .count();
                    out.push((
                        format!("{name}: {faces}/5 faces  >"),
                        Action::Open(Page::Layout(index)),
                    ));
                }
                out.push(("Reset All UV Layouts".into(), Action::UvResetAll));
            }
            Page::Layout(layout) => {
                back(&mut out);
                for direction in 0..5 {
                    let quad = config.uvs[layout][direction];
                    out.push((
                        format!(
                            "{}: {}  >",
                            UV_DIRECTION_SHORT[direction],
                            quad_text(quad)
                        ),
                        Action::Open(Page::Face { layout, direction }),
                    ));
                }
                out.push(("Reset This Layout".into(), Action::UvResetLayout));
                out.push(("Reset All UV Layouts".into(), Action::UvResetAll));
            }
            Page::Face { layout, direction } => {
                back(&mut out);
                let quad = config.uvs[layout][direction];
                let quad = quad.unwrap_or(UvQuad::UNUSED);
                for (field, name) in ["X1", "Y1", "X2", "Y2"].into_iter().enumerate() {
                    let value = match field {
                        0 => quad.x1,
                        1 => quad.y1,
                        2 => quad.x2,
                        _ => quad.y2,
                    };
                    out.push((
                        format!("{name}: {value}  (-)"),
                        Action::UvCoord {
                            direction,
                            field,
                            delta: -1,
                        },
                    ));
                    out.push((
                        format!("{name}: {value}  (+)"),
                        Action::UvCoord {
                            direction,
                            field,
                            delta: 1,
                        },
                    ));
                }
                out.push((
                    format!(
                        "Visible: {}",
                        enabled(config.uvs[layout][direction].is_some())
                    ),
                    Action::UvVisible { direction },
                ));
                out.push(("Reset This Face".into(), Action::UvResetFace { direction }));
                out.push(("Reset This Layout".into(), Action::UvResetLayout));
                out.push(("Reset All UV Layouts".into(), Action::UvResetAll));
            }
        }
        out
    }

    /// Applies `button` of the current page. Returns true when a stored value
    /// changed (navigation and read-only rows return false).
    pub fn click(
        &mut self,
        button: usize,
        config: &mut AvatarConfig,
        client: &mut ClientConfig,
    ) -> bool {
        let Some((_, action)) = self.buttons(config, client).get(button).cloned() else {
            return false;
        };
        if let Action::Open(page) = action {
            self.stack.push(page);
            return false;
        }
        if action == Action::Back {
            if self.stack.len() > 1 {
                self.stack.pop();
            }
            return false;
        }
        let changed = match action {
            Action::Gender => {
                config.gender = config.gender.next();
                true
            }
            Action::Bust(delta) => {
                config.breasts.bust_size = stepped(config.breasts.bust_size, delta, 0.0, 0.8, 0.01);
                true
            }
            Action::Cleavage(delta) => {
                config.breasts.cleavage = stepped(config.breasts.cleavage, delta, 0.0, 0.1, 0.1);
                true
            }
            Action::OffsetX(delta) => {
                config.breasts.x_offset =
                    stepped(config.breasts.x_offset, delta, -1.0, 1.0, 0.05);
                true
            }
            Action::OffsetY(delta) => {
                config.breasts.y_offset =
                    stepped(config.breasts.y_offset, delta, -1.0, 1.0, 0.05);
                true
            }
            Action::OffsetZ(delta) => {
                config.breasts.z_offset =
                    stepped(config.breasts.z_offset, delta, -1.0, 0.0, 0.05);
                true
            }
            Action::Physics => {
                config.breasts.physics = !config.breasts.physics;
                true
            }
            Action::Uniboob => {
                config.breasts.uniboob = !config.breasts.uniboob;
                true
            }
            Action::Bounce(delta) => {
                config.breasts.bounce_multiplier =
                    stepped(config.breasts.bounce_multiplier, delta, 0.0, 0.5, 0.005);
                true
            }
            Action::Floppiness(delta) => {
                config.breasts.floppiness =
                    stepped(config.breasts.floppiness, delta, 0.25, 1.0, 0.01);
                true
            }
            Action::Hurt => {
                config.sounds.hurt = !config.sounds.hurt;
                true
            }
            Action::Pitch(delta) => {
                config.sounds.voice_pitch =
                    stepped(config.sounds.voice_pitch, delta, 0.8, 1.2, 0.01);
                true
            }
            Action::ShowInArmor => {
                config.show_in_armor = !config.show_in_armor;
                true
            }
            Action::ArmorPhysics => {
                client.armor_physics_override = !client.armor_physics_override;
                true
            }
            Action::ArmorStat => {
                client.armor_stat = !client.armor_stat;
                true
            }
            Action::DisableRendering => {
                client.disable_rendering = !client.disable_rendering;
                true
            }
            Action::DisableSoundReplacement => {
                client.disable_sound_replacement = !client.disable_sound_replacement;
                true
            }
            Action::ShowToast => {
                client.show_toast = !client.show_toast;
                true
            }
            Action::FirstTime => {
                client.first_time_load = !client.first_time_load;
                true
            }
            Action::PlayerList => {
                client.player_list_mode = client.player_list_mode.next();
                true
            }
            Action::Contributor => {
                client.hide_own_contributor_tag = !client.hide_own_contributor_tag;
                true
            }
            Action::CloudEnabled => {
                client.cloud_enabled = !client.cloud_enabled;
                true
            }
            Action::CloudAutomatic => {
                client.cloud_automatic = !client.cloud_automatic;
                true
            }
            Action::CloudServer => {
                client.cloud_server = match client.cloud_server.as_str() {
                    "" => "https://wfgm.celestialfault.dev/v2".into(),
                    "https://wfgm.celestialfault.dev/v2" => "http://localhost:8080".into(),
                    _ => String::new(),
                };
                true
            }
            Action::CloudVerbosity => {
                client.cloud_log_verbosity = client.cloud_log_verbosity.next();
                true
            }
            Action::UvCoord {
                direction,
                field,
                delta,
            } => {
                let Page::Face { layout, .. } = self.page() else {
                    return false;
                };
                let fallback = default_layout(layout)[direction]
                    .filter(|quad| !quad.is_unused())
                    .unwrap_or(UvQuad {
                        x1: 0,
                        y1: 0,
                        x2: 4,
                        y2: 5,
                    });
                let quad = config.uvs[layout][direction].get_or_insert(fallback);
                let value = match field {
                    0 => &mut quad.x1,
                    1 => &mut quad.y1,
                    2 => &mut quad.x2,
                    _ => &mut quad.y2,
                };
                *value = (*value + delta).clamp(0, 64);
                true
            }
            Action::UvVisible { direction } => {
                let Page::Face { layout, .. } = self.page() else {
                    return false;
                };
                let quad = &mut config.uvs[layout][direction];
                if quad.is_some() {
                    *quad = None;
                } else {
                    *quad = Some(
                        default_layout(layout)[direction]
                            .filter(|quad| !quad.is_unused())
                            .unwrap_or(UvQuad {
                                x1: 0,
                                y1: 0,
                                x2: 4,
                                y2: 5,
                            }),
                    );
                }
                true
            }
            Action::UvResetFace { direction } => {
                let Page::Face { layout, .. } = self.page() else {
                    return false;
                };
                config.uvs[layout][direction] = default_layout(layout)[direction];
                true
            }
            Action::UvResetLayout => {
                let Page::Layout(layout) = self.page() else {
                    return false;
                };
                config.uvs[layout] = default_layout(layout);
                true
            }
            Action::UvResetAll => {
                config.uvs = default_uvs();
                true
            }
            Action::ResetAppearance => {
                let default = crate::config::Breasts::default();
                config.breasts.bust_size = default.bust_size;
                config.breasts.cleavage = default.cleavage;
                config.breasts.x_offset = default.x_offset;
                config.breasts.y_offset = default.y_offset;
                config.breasts.z_offset = default.z_offset;
                true
            }
            Action::ResetPhysics => {
                config.breasts.reset_physics();
                true
            }
            Action::ResetSounds => {
                let default = crate::config::Sounds::default();
                config.sounds = default;
                true
            }
            Action::ResetArmor => {
                config.show_in_armor = true;
                true
            }
            Action::ResetClient => {
                let first_time = client.first_time_load;
                *client = ClientConfig::default();
                client.first_time_load = first_time;
                true
            }
            Action::ResetAll => {
                *config = AvatarConfig::default();
                *client = ClientConfig::default();
                self.stack = vec![Page::Root];
                true
            }
            Action::Open(_) | Action::Back | Action::None => false,
        };
        changed
    }
}

fn back(out: &mut Vec<(String, Action)>) {
    out.push(("< Back".into(), Action::Back));
}

fn enabled(value: bool) -> &'static str {
    if value { "Enabled" } else { "Disabled" }
}

fn bust_percent(config: &AvatarConfig) -> i32 {
    (config.breasts.bust_size * 125.0).round() as i32
}

fn cleavage_degrees(config: &AvatarConfig) -> i32 {
    (config.breasts.cleavage * 100.0).round() as i32
}

fn bounce_percent(config: &AvatarConfig) -> i32 {
    (config.breasts.bounce_multiplier * 300.0).round() as i32
}

fn floppiness_percent(config: &AvatarConfig) -> i32 {
    (config.breasts.floppiness * 100.0).round() as i32
}

fn pitch_percent(config: &AvatarConfig) -> i32 {
    (config.sounds.voice_pitch * 100.0).round() as i32
}

fn quad_text(quad: Option<UvQuad>) -> String {
    match quad {
        Some(quad) if quad.is_unused() => "unused".into(),
        Some(quad) => format!("{},{} -> {},{}", quad.x1, quad.y1, quad.x2, quad.y2),
        None => "hidden".into(),
    }
}

/// Moves by `delta` and snaps to `step`, staying inside the original range
/// (a slider would stop at the same place).
fn stepped(value: f32, delta: f32, min: f32, max: f32, step: f32) -> f32 {
    let value = if value.is_finite() { value } else { min };
    let next = ((value + delta) / step).round() * step;
    next.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Gender, PlayerListMode};

    fn click_label(ui: &mut Ui, config: &mut AvatarConfig, client: &mut ClientConfig, label: &str) {
        let index = ui
            .buttons(config, client)
            .iter()
            .position(|(text, _)| text.starts_with(label))
            .unwrap_or_else(|| panic!("no button starting with {label:?}"));
        ui.click(index, config, client);
    }

    #[test]
    fn root_shows_the_main_values() {
        let ui = Ui::default();
        let config = AvatarConfig::default();
        let client = ClientConfig::default();
        let buttons = ui.buttons(&config, &client);
        assert!(buttons[0].0.contains("Male"));
        assert!(buttons[1].0.contains("75%"));
    }

    #[test]
    fn pages_open_and_go_back() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        let index = ui
            .buttons(&config, &client)
            .iter()
            .position(|(label, _)| label.starts_with("Appearance"))
            .unwrap();
        ui.click(index, &mut config, &mut client);
        assert_eq!(ui.page(), Page::Appearance);
        assert!(ui.buttons(&config, &client)[0].0.starts_with("< Back"));
        ui.click(0, &mut config, &mut client);
        assert_eq!(ui.page(), Page::Root);
    }

    #[test]
    fn every_button_of_every_page_is_clickable() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        config.gender = Gender::Female;
        for page in [
            Page::Root,
            Page::Appearance,
            Page::Physics,
            Page::Sounds,
            Page::Armor,
            Page::Client,
            Page::Cloud,
            Page::Reset,
            Page::Layouts,
            Page::Layout(0),
            Page::Face {
                layout: 0,
                direction: 0,
            },
            Page::Face {
                layout: 3,
                direction: 4,
            },
        ] {
            ui.stack = vec![page];
            let count = ui.buttons(&config, &client).len();
            for button in 0..count {
                let mut copy = ui.clone();
                copy.click(button, &mut config, &mut client);
                assert!(copy.buttons(&config, &client).len() > 0);
            }
        }
    }

    #[test]
    fn gender_cycles_and_bust_stops_at_the_ends() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        click_label(&mut ui, &mut config, &mut client, "Gender");
        assert_eq!(config.gender, Gender::Female);
        click_label(&mut ui, &mut config, &mut client, "Gender");
        assert_eq!(config.gender, Gender::Other);
        click_label(&mut ui, &mut config, &mut client, "Gender");
        assert_eq!(config.gender, Gender::Male);

        ui.stack = vec![Page::Appearance];
        for _ in 0..100 {
            click_label(&mut ui, &mut config, &mut client, "Bust Size");
        }
        // The first "Bust Size" row is the (-) one.
        assert_eq!(config.breasts.bust_size, 0.0);
    }

    #[test]
    fn uv_face_can_be_hidden_and_reset() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        ui.stack = vec![Page::Face {
            layout: 0,
            direction: 0,
        }];
        click_label(&mut ui, &mut config, &mut client, "Visible");
        assert_eq!(config.uvs[0][0], None);
        click_label(&mut ui, &mut config, &mut client, "Reset This Face");
        assert_eq!(config.uvs[0][0], default_layout(0)[0]);

        click_label(&mut ui, &mut config, &mut client, "X1");
        let quad = config.uvs[0][0].unwrap();
        assert_eq!(quad.x1, 23);
    }

    #[test]
    fn reset_everything_returns_to_the_defaults() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        config.gender = Gender::Female;
        config.breasts.bust_size = 0.2;
        client.disable_rendering = true;
        ui.stack = vec![Page::Reset];
        click_label(&mut ui, &mut config, &mut client, "Reset Everything");
        assert_eq!(config, AvatarConfig::default());
        assert_eq!(client, ClientConfig::default());
        assert_eq!(ui.page(), Page::Root);
    }

    #[test]
    fn player_list_cycles_all_modes() {
        let mut ui = Ui::default();
        let mut config = AvatarConfig::default();
        let mut client = ClientConfig::default();
        ui.stack = vec![Page::Client];
        click_label(&mut ui, &mut config, &mut client, "Player List");
        assert_eq!(client.player_list_mode, PlayerListMode::TabListOpen);
        click_label(&mut ui, &mut config, &mut client, "Player List");
        assert_eq!(client.player_list_mode, PlayerListMode::Always);
        click_label(&mut ui, &mut config, &mut client, "Player List");
        assert_eq!(client.player_list_mode, PlayerListMode::ModUiOnly);
    }
}
