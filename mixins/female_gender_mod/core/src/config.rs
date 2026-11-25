//! The values of Female Gender Mod: every setting the Fabric mod stores,
//! their defaults, and the one-line text form this port keeps in Horizon's
//! native settings file.
//!
//! The network codec lives in `net.rs`; this module only describes data.

use std::collections::HashMap;

/// `com.wildfire.api.Gender`. The ordinal order is the wire order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gender {
    Female,
    #[default]
    Male,
    Other,
}

impl Gender {
    /// `Gender.next`: male -> female -> other -> male.
    pub fn next(self) -> Self {
        match self {
            Self::Male => Self::Female,
            Self::Female => Self::Other,
            Self::Other => Self::Male,
        }
    }

    pub fn can_have_breasts(self) -> bool {
        !matches!(self, Self::Male)
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Female => "female",
            Self::Male => "male",
            Self::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Female => "Female",
            Self::Male => "Male",
            Self::Other => "Other",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value {
            "female" => Some(Self::Female),
            "male" => Some(Self::Male),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    pub fn wire(self) -> u32 {
        match self {
            Self::Female => 0,
            Self::Male => 1,
            Self::Other => 2,
        }
    }

    pub fn from_wire(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Female),
            1 => Some(Self::Male),
            2 => Some(Self::Other),
            _ => None,
        }
    }
}

/// One face of a breast box, in texture pixels (the original uses a 64x64
/// skin texture). `None` means the layout leaves that face out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UvQuad {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

impl UvQuad {
    pub const UNUSED: Self = Self {
        x1: 0,
        y1: 0,
        x2: 0,
        y2: 0,
    };

    pub fn is_unused(self) -> bool {
        self == Self::UNUSED
    }
}

/// Faces in `UVDirection` ordinal order: east, west, down, up, north.
pub type UvLayout = [Option<UvQuad>; 5];
pub const UV_DIRECTIONS: [&str; 5] = ["east", "west", "down", "up", "north"];
pub const UV_DIRECTION_SHORT: [&str; 5] = ["E", "W", "D", "U", "N"];

pub const LAYOUT_SKIN_LEFT: usize = 0;
pub const LAYOUT_SKIN_RIGHT: usize = 1;
pub const LAYOUT_OVERLAY_LEFT: usize = 2;
pub const LAYOUT_OVERLAY_RIGHT: usize = 3;
pub const LAYOUT_NAMES: [&str; 4] = ["Skin Left", "Skin Right", "Overlay Left", "Overlay Right"];

fn q(x1: i32, y1: i32, x2: i32, y2: i32) -> Option<UvQuad> {
    Some(UvQuad { x1, y1, x2, y2 })
}

/// The four layouts of `com.wildfire.common.config.UVs`, exactly as the
/// original stores them (overlay left has no east face, but it is present in
/// the map as `UVQuad.UNUSED`; keeping it here preserves the wire size).
pub fn default_layout(index: usize) -> UvLayout {
    match index {
        LAYOUT_SKIN_LEFT => [
            q(24, 21, 27, 26),
            q(16, 21, 20, 26),
            q(20, 17, 24, 21),
            q(20, 25, 24, 27),
            q(20, 21, 24, 26),
        ],
        LAYOUT_SKIN_RIGHT => [
            q(28, 21, 32, 26),
            q(21, 21, 24, 26),
            q(24, 17, 28, 21),
            q(24, 25, 28, 27),
            q(24, 21, 28, 26),
        ],
        LAYOUT_OVERLAY_LEFT => [
            q(0, 0, 0, 0),
            q(17, 37, 20, 42),
            q(20, 34, 24, 37),
            q(20, 41, 24, 44),
            q(20, 37, 24, 42),
        ],
        LAYOUT_OVERLAY_RIGHT => [
            q(28, 37, 31, 42),
            q(0, 0, 0, 0),
            q(24, 34, 28, 37),
            q(24, 41, 28, 44),
            q(24, 37, 28, 42),
        ],
        _ => default_layout(LAYOUT_SKIN_LEFT),
    }
}

pub fn default_uvs() -> [UvLayout; 4] {
    [
        default_layout(0),
        default_layout(1),
        default_layout(2),
        default_layout(3),
    ]
}

pub const ARMOR_LAYOUT_LEFT: usize = 0;
pub const ARMOR_LAYOUT_RIGHT: usize = 1;

/// `IBreastArmorTexture.DEFAULT_UVS`: the armour texture is 64x32, and these
/// five faces per side sample the chestplate's own body region.
pub fn default_armor_layout(index: usize) -> UvLayout {
    match index {
        ARMOR_LAYOUT_RIGHT => [
            q(28, 21, 32, 26),
            q(20, 21, 24, 26),
            q(24, 17, 28, 21),
            q(24, 25, 28, 27),
            q(24, 21, 28, 26),
        ],
        _ => [
            q(24, 21, 28, 26),
            q(16, 21, 20, 26),
            q(20, 17, 24, 21),
            q(20, 25, 24, 27),
            q(20, 21, 24, 26),
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Breasts {
    pub x_offset: f32,
    pub y_offset: f32,
    pub z_offset: f32,
    pub bust_size: f32,
    pub cleavage: f32,
    pub physics: bool,
    pub uniboob: bool,
    pub bounce_multiplier: f32,
    pub floppiness: f32,
}

impl Default for Breasts {
    fn default() -> Self {
        Self {
            x_offset: 0.0,
            y_offset: 0.0,
            z_offset: 0.0,
            bust_size: 0.6,
            cleavage: 0.0,
            physics: true,
            uniboob: true,
            bounce_multiplier: 0.333,
            floppiness: 0.75,
        }
    }
}

impl Breasts {
    pub fn reset_physics(&mut self) {
        let default = Self::default();
        self.physics = default.physics;
        self.uniboob = default.uniboob;
        self.bounce_multiplier = default.bounce_multiplier;
        self.floppiness = default.floppiness;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sounds {
    pub hurt: bool,
    pub voice_pitch: f32,
}

impl Default for Sounds {
    fn default() -> Self {
        Self {
            hurt: true,
            voice_pitch: 1.0,
        }
    }
}

/// `com.wildfire.common.entities.avatars.AvatarConfig`, the part that is sent
/// to Fabric servers and other clients.
#[derive(Clone, Debug, PartialEq)]
pub struct AvatarConfig {
    pub gender: Gender,
    pub breasts: Breasts,
    /// skin left, skin right, overlay left, overlay right.
    pub uvs: [UvLayout; 4],
    pub sounds: Sounds,
    pub show_in_armor: bool,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            gender: Gender::Male,
            breasts: Breasts::default(),
            uvs: default_uvs(),
            sounds: Sounds::default(),
            show_in_armor: true,
        }
    }
}

impl AvatarConfig {
    pub fn can_render(&self) -> bool {
        self.gender.can_have_breasts() && self.breasts.bust_size >= 0.02
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerListMode {
    #[default]
    ModUiOnly,
    TabListOpen,
    Always,
}

impl PlayerListMode {
    pub fn next(self) -> Self {
        match self {
            Self::ModUiOnly => Self::TabListOpen,
            Self::TabListOpen => Self::Always,
            Self::Always => Self::ModUiOnly,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::ModUiOnly => "mod_ui_only",
            Self::TabListOpen => "tab_list_open",
            Self::Always => "always",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ModUiOnly => "Mod UI only",
            Self::TabListOpen => "Tab list open",
            Self::Always => "Always",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value {
            "mod_ui_only" => Some(Self::ModUiOnly),
            "tab_list_open" => Some(Self::TabListOpen),
            "always" => Some(Self::Always),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SyncVerbosity {
    #[default]
    Default,
    ShowFetches,
}

impl SyncVerbosity {
    pub fn next(self) -> Self {
        match self {
            Self::Default => Self::ShowFetches,
            Self::ShowFetches => Self::Default,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::ShowFetches => "show_fetches",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::ShowFetches => "Show fetches",
        }
    }

    pub fn from_name(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "show_fetches" => Some(Self::ShowFetches),
            _ => None,
        }
    }
}

/// The original client-only config: `ClientConfigInstance`, `ConfigOverrides`
/// and `CloudSyncConfig`.
#[derive(Clone, Debug, PartialEq)]
pub struct ClientConfig {
    pub first_time_load: bool,
    pub show_toast: bool,
    pub armor_stat: bool,
    pub player_list_mode: PlayerListMode,
    pub hide_own_contributor_tag: bool,

    pub cloud_enabled: bool,
    pub cloud_automatic: bool,
    pub cloud_server: String,
    pub cloud_log_verbosity: SyncVerbosity,

    pub armor_physics_override: bool,
    pub disable_rendering: bool,
    pub disable_sound_replacement: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            first_time_load: true,
            show_toast: true,
            armor_stat: true,
            player_list_mode: PlayerListMode::ModUiOnly,
            hide_own_contributor_tag: false,
            cloud_enabled: false,
            cloud_automatic: false,
            cloud_server: String::new(),
            cloud_log_verbosity: SyncVerbosity::Default,
            armor_physics_override: false,
            disable_rendering: false,
            disable_sound_replacement: false,
        }
    }
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

fn push_bool(out: &mut String, key: &str, value: bool) {
    out.push_str(key);
    out.push('=');
    out.push(if value { '1' } else { '0' });
    out.push(';');
}

fn push_f32(out: &mut String, key: &str, value: f32) {
    out.push_str(key);
    out.push('=');
    out.push_str(&format!("{value:.5}"));
    out.push(';');
}

fn push_quad(out: &mut String, key: &str, quad: &Option<UvQuad>) {
    out.push_str(key);
    out.push('=');
    match quad {
        Some(quad) => out.push_str(&format!(
            "{},{},{},{}",
            quad.x1, quad.y1, quad.x2, quad.y2
        )),
        // A hidden face must be stored, or it would come back as the layout's
        // default quad.
        None => out.push_str("hidden"),
    }
    out.push(';');
}

/// One line, because Horizon stores a setting key's value as one line.
/// Values never contain `;` or `=`.
pub fn serialize(config: &AvatarConfig, client: &ClientConfig) -> String {
    let mut out = String::with_capacity(1400);
    out.push_str("v=1;");
    out.push_str("gender=");
    out.push_str(config.gender.name());
    out.push(';');
    push_f32(&mut out, "bust", config.breasts.bust_size);
    push_f32(&mut out, "cleavage", config.breasts.cleavage);
    push_f32(&mut out, "ox", config.breasts.x_offset);
    push_f32(&mut out, "oy", config.breasts.y_offset);
    push_f32(&mut out, "oz", config.breasts.z_offset);
    push_bool(&mut out, "phys", config.breasts.physics);
    push_bool(&mut out, "uni", config.breasts.uniboob);
    push_f32(&mut out, "bounce", config.breasts.bounce_multiplier);
    push_f32(&mut out, "floppy", config.breasts.floppiness);
    for (layout_index, layout) in config.uvs.iter().enumerate() {
        for (direction, quad) in layout.iter().enumerate() {
            let key = format!(
                "uv{}.{}",
                layout_index,
                UV_DIRECTION_SHORT[direction].to_ascii_lowercase()
            );
            push_quad(&mut out, &key, quad);
        }
    }
    push_bool(&mut out, "hurt", config.sounds.hurt);
    push_f32(&mut out, "pitch", config.sounds.voice_pitch);
    push_bool(&mut out, "armor", config.show_in_armor);

    push_bool(&mut out, "c.first", client.first_time_load);
    push_bool(&mut out, "c.toast", client.show_toast);
    push_bool(&mut out, "c.armorstat", client.armor_stat);
    out.push_str("c.list=");
    out.push_str(client.player_list_mode.name());
    out.push(';');
    push_bool(&mut out, "c.contrib", client.hide_own_contributor_tag);
    push_bool(&mut out, "c.cloud", client.cloud_enabled);
    push_bool(&mut out, "c.auto", client.cloud_automatic);
    out.push_str("c.server=");
    out.push_str(&client.cloud_server.replace([';', '='], ""));
    out.push(';');
    out.push_str("c.verb=");
    out.push_str(client.cloud_log_verbosity.name());
    out.push(';');
    push_bool(&mut out, "c.armorphys", client.armor_physics_override);
    push_bool(&mut out, "c.norender", client.disable_rendering);
    push_bool(&mut out, "c.nosound", client.disable_sound_replacement);
    out.pop();
    out
}

fn map_of(raw: &str) -> HashMap<&str, &str> {
    raw.split(';')
        .filter_map(|entry| entry.split_once('='))
        .collect()
}

fn get_bool(map: &HashMap<&str, &str>, key: &str, default: bool) -> bool {
    match map.get(key).copied() {
        Some("1" | "true" | "TRUE") => true,
        Some("0" | "false" | "FALSE") => false,
        _ => default,
    }
}

fn get_f32(map: &HashMap<&str, &str>, key: &str, default: f32) -> f32 {
    map.get(key)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
}

fn parse_quad(raw: &str) -> Option<UvQuad> {
    let mut values = raw.split(',').filter_map(|v| v.parse::<i32>().ok());
    Some(UvQuad {
        x1: values.next()?,
        y1: values.next()?,
        x2: values.next()?,
        y2: values.next()?,
    })
}

fn parse_layout(map: &HashMap<&str, &str>, index: usize) -> UvLayout {
    let mut layout = default_layout(index);
    for direction in 0..5 {
        let key = format!(
            "uv{}.{}",
            index,
            UV_DIRECTION_SHORT[direction].to_ascii_lowercase()
        );
        if let Some(&value) = map.get(key.as_str()) {
            if value == "hidden" {
                layout[direction] = None;
            } else if let Some(quad) = parse_quad(value) {
                layout[direction] = Some(UvQuad {
                    x1: quad.x1.clamp(0, 64),
                    y1: quad.y1.clamp(0, 64),
                    x2: quad.x2.clamp(0, 64),
                    y2: quad.y2.clamp(0, 64),
                });
            }
        }
    }
    layout
}

pub fn deserialize(raw: &str) -> (AvatarConfig, ClientConfig) {
    let map = map_of(raw);
    let mut config = AvatarConfig::default();
    if let Some(gender) = map.get("gender").and_then(|v| Gender::from_name(v)) {
        config.gender = gender;
    }
    config.breasts.bust_size = clamp(get_f32(&map, "bust", 0.6), 0.0, 0.8);
    config.breasts.cleavage = clamp(get_f32(&map, "cleavage", 0.0), 0.0, 0.1);
    config.breasts.x_offset = clamp(get_f32(&map, "ox", 0.0), -1.0, 1.0);
    config.breasts.y_offset = clamp(get_f32(&map, "oy", 0.0), -1.0, 1.0);
    config.breasts.z_offset = clamp(get_f32(&map, "oz", 0.0), -1.0, 0.0);
    config.breasts.physics = get_bool(&map, "phys", true);
    config.breasts.uniboob = get_bool(&map, "uni", true);
    config.breasts.bounce_multiplier = clamp(get_f32(&map, "bounce", 0.333), 0.0, 0.5);
    config.breasts.floppiness = clamp(get_f32(&map, "floppy", 0.75), 0.25, 1.0);
    for (index, layout) in config.uvs.iter_mut().enumerate() {
        *layout = parse_layout(&map, index);
    }
    config.sounds.hurt = get_bool(&map, "hurt", true);
    config.sounds.voice_pitch = clamp(get_f32(&map, "pitch", 1.0), 0.8, 1.2);
    config.show_in_armor = get_bool(&map, "armor", true);

    let mut client = ClientConfig::default();
    client.first_time_load = get_bool(&map, "c.first", true);
    client.show_toast = get_bool(&map, "c.toast", true);
    client.armor_stat = get_bool(&map, "c.armorstat", true);
    if let Some(mode) = map.get("c.list").and_then(|v| PlayerListMode::from_name(v)) {
        client.player_list_mode = mode;
    }
    client.hide_own_contributor_tag = get_bool(&map, "c.contrib", false);
    client.cloud_enabled = get_bool(&map, "c.cloud", false);
    client.cloud_automatic = get_bool(&map, "c.auto", false);
    client.cloud_server = map.get("c.server").copied().unwrap_or("").to_owned();
    if let Some(verbosity) = map.get("c.verb").and_then(|v| SyncVerbosity::from_name(v)) {
        client.cloud_log_verbosity = verbosity;
    }
    client.armor_physics_override = get_bool(&map, "c.armorphys", false);
    client.disable_rendering = get_bool(&map, "c.norender", false);
    client.disable_sound_replacement = get_bool(&map, "c.nosound", false);
    (config, client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut config = AvatarConfig::default();
        config.gender = Gender::Female;
        config.breasts.bust_size = 0.73;
        config.breasts.cleavage = 0.07;
        config.uvs[LAYOUT_SKIN_LEFT][0] = Some(UvQuad {
            x1: 1,
            y1: 2,
            x2: 3,
            y2: 4,
        });
        let mut client = ClientConfig::default();
        client.cloud_server = "http://localhost:8080".into();
        client.player_list_mode = PlayerListMode::Always;
        let text = serialize(&config, &client);
        assert!(!text.contains('\n'));
        let (round, round_client) = deserialize(&text);
        assert_eq!(round, config);
        assert_eq!(round_client, client);
    }

    #[test]
    fn hidden_face_round_trips() {
        let mut config = AvatarConfig::default();
        config.gender = Gender::Other;
        // Clear every face of the overlay-right layout, so none may fall back
        // to the skin defaults when the text is read again.
        config.uvs[LAYOUT_OVERLAY_RIGHT] = [None; 5];
        config.uvs[LAYOUT_SKIN_LEFT][2] = None;
        let client = ClientConfig::default();
        let text = serialize(&config, &client);
        assert!(text.contains("uv3.e=hidden;"));
        let (round, _) = deserialize(&text);
        assert_eq!(round, config);
    }

    #[test]
    fn armor_layout_matches_upstream() {
        let left = default_armor_layout(ARMOR_LAYOUT_LEFT);
        let right = default_armor_layout(ARMOR_LAYOUT_RIGHT);
        assert_eq!(left[4], Some(UvQuad { x1: 20, y1: 21, x2: 24, y2: 26 }));
        assert_eq!(left[0], Some(UvQuad { x1: 24, y1: 21, x2: 28, y2: 26 }));
        assert_eq!(right[4], Some(UvQuad { x1: 24, y1: 21, x2: 28, y2: 26 }));
        assert_eq!(right[2], Some(UvQuad { x1: 24, y1: 17, x2: 28, y2: 21 }));
    }
}
