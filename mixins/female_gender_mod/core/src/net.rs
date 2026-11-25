//! The wire format shared with Fabric Female Gender Mod 5.0.0.
//!
//! Channels are Fabric custom payloads. The codec is
//! `AvatarConfig.COMPACT_STREAM_CODEC`: a boolean, then (when true) the full
//! config. UV layouts use `UVLayout.STREAM_CODEC` (a varint-sized map).

use crate::config::{AvatarConfig, Breasts, Gender, Sounds, UvLayout, UvQuad};
use std::sync::Mutex;

pub const CHANNEL_HELLO_CLIENTBOUND: &str = "female_gender_mod:clientbound/hello";
pub const CHANNEL_HELLO_SERVERBOUND: &str = "female_gender_mod:serverbound/hello";
pub const CHANNEL_SYNC_CLIENTBOUND: &str = "female_gender_mod:clientbound/sync";
pub const CHANNEL_SYNC_SERVERBOUND: &str = "female_gender_mod:serverbound/sync";
/// The pre-5.0.0 Fabric sync channels. TheReizi's Female Gender Plugin still
/// speaks these, and its V5 payload carries the player UUID plus the full
/// (non-compact) config instead of the 5.0.0 codec above.
pub const LEGACY_CHANNEL_SYNC_CLIENTBOUND: &str = "wildfire_gender:sync";
pub const LEGACY_CHANNEL_SYNC_SERVERBOUND: &str = "wildfire_gender:send_gender_info";
/// Legacy Fabric channel registration (`RegistrationPayload`): NUL-separated
/// receivable channels. The configuration phase answers the server's own
/// registration with this.
pub const CHANNEL_REGISTER: &str = "minecraft:register";
/// Fabric's common version negotiation (`CommonVersionPayload`).
pub const CHANNEL_COMMON_VERSION: &str = "c:version";
/// Fabric's common play-channel registration (`CommonRegisterPayload`).
pub const CHANNEL_COMMON_REGISTER: &str = "c:register";
/// Fabric Registry Sync: a server whose synced registries contain modded
/// entries (a LAN world with FGM's client-only hurt sound, for example) sends
/// this during configuration and disconnects clients that cannot receive it.
pub const CHANNEL_REGISTRY_SYNC: &str = "fabric:registry/sync";
/// The empty acknowledgement Fabric Registry Sync waits for.
pub const CHANNEL_REGISTRY_SYNC_COMPLETE: &str = "fabric:registry/sync/complete";
/// The version of the original mod's hello/sync protocol.
pub const PROTOCOL_VERSION: u32 = 2;
/// The only common packet version Fabric 26.3 supports.
pub const COMMON_VERSION: u32 = 1;

/// The channels we can receive while configuring. The server sends its own
/// `minecraft:register`; the reply must name every common/hello channel so the
/// server's version, register and hello tasks can send to us. Registry Sync is
/// advertised so a LAN server with modded registry entries accepts us; its
/// payload is acknowledged without remapping, like the real client does for a
/// local server.
pub const CONFIG_CHANNELS: [&str; 4] = [
    CHANNEL_HELLO_CLIENTBOUND,
    CHANNEL_COMMON_VERSION,
    CHANNEL_COMMON_REGISTER,
    CHANNEL_REGISTRY_SYNC,
];

pub fn config_registration_payload() -> Vec<u8> {
    CONFIG_CHANNELS.join("\0").into_bytes()
}

/// The play phase registers the 5.0.0 channels and the legacy plugin channels;
/// `clientbound/hello` and `serverbound/hello` exist only while configuring.
pub const PLAY_CHANNELS: [&str; 4] = [
    CHANNEL_SYNC_CLIENTBOUND,
    CHANNEL_SYNC_SERVERBOUND,
    LEGACY_CHANNEL_SYNC_CLIENTBOUND,
    LEGACY_CHANNEL_SYNC_SERVERBOUND,
];

/// The legacy play registration, kept as a fallback for servers that start
/// tracking players before the common `c:register` exchange completed.
pub fn play_registration_payload() -> Vec<u8> {
    PLAY_CHANNELS.join("\0").into_bytes()
}

pub fn hello_reply_payload() -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, PROTOCOL_VERSION);
    out
}

/// `CommonVersionPayload`: a varint-sized array, here `[1]`.
pub fn common_version_reply() -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, 1);
    write_varint(&mut out, COMMON_VERSION);
    out
}

/// `CommonRegisterPayload`: varint version, the protocol name, then the
/// varint-sized set of channels. The server only accepts play channels during
/// configuration through this packet.
pub fn common_register_reply(version: u32) -> Vec<u8> {
    let mut out = Vec::new();
    write_varint(&mut out, version);
    write_string(&mut out, "play");
    write_varint(&mut out, PLAY_CHANNELS.len() as u32);
    for channel in PLAY_CHANNELS {
        write_string(&mut out, channel);
    }
    out
}

/// Configuration-phase replies waiting for the game thread. The net thread
/// sees the payloads; `Game::poll_remote` has the writer.
static CONFIG_ACTIONS: Mutex<Vec<(String, Vec<u8>)>> = Mutex::new(Vec::new());

fn queue_config(channel: &str, data: Vec<u8>) {
    let mut actions = CONFIG_ACTIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if actions.iter().any(|(queued, _)| queued == channel) {
        return;
    }
    actions.push((channel.to_owned(), data));
}

/// Handles one configuration-phase custom payload from the server. The common
/// handshake runs before play, and the server blocks on each task until it
/// sees the matching reply, so these are queued immediately.
pub fn on_config_payload(channel: &str, data: &[u8]) {
    match channel {
        CHANNEL_REGISTER => queue_config(CHANNEL_REGISTER, config_registration_payload()),
        CHANNEL_COMMON_VERSION => queue_config(CHANNEL_COMMON_VERSION, common_version_reply()),
        CHANNEL_COMMON_REGISTER => {
            if let Some(version) = common_register_version(data) {
                queue_config(CHANNEL_COMMON_REGISTER, common_register_reply(version));
            }
        }
        CHANNEL_HELLO_CLIENTBOUND => {
            queue_config(CHANNEL_HELLO_SERVERBOUND, hello_reply_payload());
        }
        // The registry map itself can be ignored: FGM's only server-visible
        // entry is appended to the sound registry, so every vanilla raw ID we
        // receive still matches. The server blocks until this ack arrives.
        CHANNEL_REGISTRY_SYNC => {
            queue_config(CHANNEL_REGISTRY_SYNC_COMPLETE, Vec::new());
        }
        _ => {}
    }
}

/// The version of a server `c:register` for the play protocol. A register for
/// another protocol is ignored: our only non-play channel is the
/// configuration hello, which is already registered with `minecraft:register`.
fn common_register_version(data: &[u8]) -> Option<u32> {
    let mut reader = Reader::new(data);
    let version = reader.varint()?;
    if reader.string()? != "play" || version > i32::MAX as u32 {
        return None;
    }
    Some(version)
}

pub fn take_config_actions() -> Vec<(String, Vec<u8>)> {
    std::mem::take(
        &mut *CONFIG_ACTIONS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

/// A new configuration phase starts: replies queued for the previous server
/// must not reach this one.
pub fn clear_config_actions() {
    CONFIG_ACTIONS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clear();
}

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn u8(&mut self) -> Option<u8> {
        let value = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(value)
    }

    fn bool(&mut self) -> Option<bool> {
        Some(self.u8()? != 0)
    }

    fn varint(&mut self) -> Option<u32> {
        let mut value = 0u32;
        for shift in (0..35).step_by(7) {
            let byte = self.u8()?;
            value |= u32::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    /// `FriendlyByteBuf.readUtf`: a varint byte length, then UTF-8.
    fn string(&mut self) -> Option<String> {
        let len = self.varint()? as usize;
        let end = self.pos.checked_add(len)?;
        let bytes = self.data.get(self.pos..end)?;
        self.pos = end;
        std::str::from_utf8(bytes).ok().map(str::to_owned)
    }

    fn f32(&mut self) -> Option<f32> {
        let bytes = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn uuid(&mut self) -> Option<u128> {
        let bytes = self.data.get(self.pos..self.pos + 16)?;
        self.pos += 16;
        Some(u128::from_be_bytes(bytes.try_into().ok()?))
    }
}

fn write_varint(out: &mut Vec<u8>, mut value: u32) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn write_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_bool(out: &mut Vec<u8>, value: bool) {
    out.push(u8::from(value));
}

/// `FriendlyByteBuf.writeUtf`: a varint byte length, then UTF-8.
fn write_string(out: &mut Vec<u8>, value: &str) {
    write_varint(out, value.len() as u32);
    out.extend_from_slice(value.as_bytes());
}

fn write_layout(out: &mut Vec<u8>, layout: &UvLayout) {
    let present = layout.iter().filter(|quad| quad.is_some()).count();
    write_varint(out, present as u32);
    for (direction, quad) in layout.iter().enumerate() {
        if let Some(quad) = quad {
            write_varint(out, direction as u32);
            write_varint(out, quad.x1.max(0) as u32);
            write_varint(out, quad.y1.max(0) as u32);
            write_varint(out, quad.x2.max(0) as u32);
            write_varint(out, quad.y2.max(0) as u32);
        }
    }
}

fn read_layout(reader: &mut Reader<'_>) -> Option<UvLayout> {
    let count = reader.varint()? as usize;
    if count > 5 {
        return None;
    }
    let mut layout: UvLayout = [None; 5];
    for _ in 0..count {
        let direction = reader.varint()? as usize;
        if direction >= 5 {
            return None;
        }
        let quad = UvQuad {
            x1: reader.varint()? as i32,
            y1: reader.varint()? as i32,
            x2: reader.varint()? as i32,
            y2: reader.varint()? as i32,
        };
        layout[direction] = Some(quad);
    }
    Some(layout)
}

fn clamp(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

/// `AvatarConfig.COMPACT_STREAM_CODEC.encode`: a default male config is one
/// `false` byte; everything else is the full stream.
pub fn encode_compact(config: &AvatarConfig) -> Vec<u8> {
    let mut out = Vec::new();
    if config.gender == Gender::Male {
        write_bool(&mut out, false);
        return out;
    }
    write_bool(&mut out, true);
    write_varint(&mut out, config.gender.wire());
    write_f32(&mut out, config.breasts.x_offset);
    write_f32(&mut out, config.breasts.y_offset);
    write_f32(&mut out, config.breasts.z_offset);
    write_f32(&mut out, config.breasts.bust_size);
    write_f32(&mut out, config.breasts.cleavage);
    write_bool(&mut out, config.breasts.physics);
    if config.breasts.physics {
        write_bool(&mut out, config.breasts.uniboob);
        write_f32(&mut out, config.breasts.bounce_multiplier);
        write_f32(&mut out, config.breasts.floppiness);
    }
    for layout in &config.uvs {
        write_layout(&mut out, layout);
    }
    write_bool(&mut out, config.sounds.hurt);
    write_f32(&mut out, config.sounds.voice_pitch);
    write_bool(&mut out, config.show_in_armor);
    out
}

pub fn decode_compact(data: &[u8]) -> Option<AvatarConfig> {
    let mut reader = Reader::new(data);
    if !reader.bool()? {
        return Some(AvatarConfig::default());
    }
    let gender = Gender::from_wire(reader.varint()?)?;
    let x_offset = clamp(reader.f32()?, -1.0, 1.0);
    let y_offset = clamp(reader.f32()?, -1.0, 1.0);
    let z_offset = clamp(reader.f32()?, -1.0, 0.0);
    let bust_size = clamp(reader.f32()?, 0.0, 0.8);
    let cleavage = clamp(reader.f32()?, 0.0, 0.1);
    let physics = reader.bool()?;
    let (uniboob, bounce_multiplier, floppiness) = if physics {
        (
            reader.bool()?,
            clamp(reader.f32()?, 0.0, 0.5),
            clamp(reader.f32()?, 0.25, 1.0),
        )
    } else {
        (true, 0.333, 0.75)
    };
    let mut uvs = crate::config::default_uvs();
    for layout in &mut uvs {
        *layout = read_layout(&mut reader)?;
    }
    let hurt = reader.bool()?;
    let voice_pitch = clamp(reader.f32()?, 0.8, 1.2);
    let show_in_armor = reader.bool()?;
    Some(AvatarConfig {
        gender,
        breasts: Breasts {
            x_offset,
            y_offset,
            z_offset,
            bust_size,
            cleavage,
            physics,
            uniboob,
            bounce_multiplier,
            floppiness,
        },
        uvs,
        sounds: Sounds { hurt, voice_pitch },
        show_in_armor,
    })
}

/// `ClientboundSyncPacket`: UUID (`UUIDUtil.STREAM_CODEC`, two big-endian
/// longs) followed by the compact avatar config.
pub fn decode_clientbound_sync(data: &[u8]) -> Option<(u128, AvatarConfig)> {
    if data.len() < 16 {
        return None;
    }
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&data[..16]);
    let uuid = u128::from_be_bytes(bytes);
    let config = decode_compact(&data[16..])?;
    Some((uuid, config))
}

pub fn encode_clientbound_sync(uuid: u128, config: &AvatarConfig) -> Vec<u8> {
    let mut out = Vec::with_capacity(17);
    out.extend_from_slice(&uuid.to_be_bytes());
    out.extend_from_slice(&encode_compact(config));
    out
}

pub fn encode_serverbound_sync(config: &AvatarConfig) -> Vec<u8> {
    encode_compact(config)
}

/// The legacy plugin's V5 layout: all five directions are always present,
/// because the plugin fills missing keys with the mod defaults and would
/// otherwise lose deliberately hidden faces.
fn write_legacy_layout(out: &mut Vec<u8>, index: usize, layout: &UvLayout) {
    let fallback = crate::config::default_layout(index);
    write_varint(out, 5);
    for (direction, fallback_quad) in fallback.iter().enumerate() {
        let quad = layout[direction]
            .or(*fallback_quad)
            .unwrap_or(UvQuad::UNUSED);
        write_varint(out, direction as u32);
        write_varint(out, quad.x1.max(0) as u32);
        write_varint(out, quad.y1.max(0) as u32);
        write_varint(out, quad.x2.max(0) as u32);
        write_varint(out, quad.y2.max(0) as u32);
    }
}

fn read_legacy_layout(reader: &mut Reader<'_>, index: usize) -> Option<UvLayout> {
    let count = reader.varint()? as usize;
    if count > 5 {
        return None;
    }
    let mut layout = crate::config::default_layout(index);
    for _ in 0..count {
        let direction = reader.varint()? as usize;
        if direction >= 5 {
            return None;
        }
        layout[direction] = Some(UvQuad {
            x1: reader.varint()? as i32,
            y1: reader.varint()? as i32,
            x2: reader.varint()? as i32,
            y2: reader.varint()? as i32,
        });
    }
    Some(layout)
}

/// The legacy plugin's `send_gender_info`/`sync` payload:
/// `UUID + GenderIdentities + full AvatarConfig fields + four UV layouts`.
pub fn encode_legacy_sync(uuid: u128, config: &AvatarConfig) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&uuid.to_be_bytes());
    write_varint(&mut out, config.gender.wire());
    write_f32(&mut out, config.breasts.bust_size);
    write_bool(&mut out, config.sounds.hurt);
    write_f32(&mut out, config.sounds.voice_pitch);
    write_bool(&mut out, config.breasts.physics);
    write_bool(&mut out, config.show_in_armor);
    write_f32(&mut out, config.breasts.bounce_multiplier);
    write_f32(&mut out, config.breasts.floppiness);
    write_f32(&mut out, config.breasts.x_offset);
    write_f32(&mut out, config.breasts.y_offset);
    write_f32(&mut out, config.breasts.z_offset);
    write_bool(&mut out, config.breasts.uniboob);
    write_f32(&mut out, config.breasts.cleavage);
    for (index, layout) in config.uvs.iter().enumerate() {
        write_legacy_layout(&mut out, index, layout);
    }
    out
}

pub fn decode_legacy_clientbound_sync(data: &[u8]) -> Option<(u128, AvatarConfig)> {
    let mut reader = Reader::new(data);
    let uuid = reader.uuid()?;
    let gender = Gender::from_wire(reader.varint()?)?;
    let bust_size = clamp(reader.f32()?, 0.0, 0.8);
    let hurt = reader.bool()?;
    let voice_pitch = clamp(reader.f32()?, 0.8, 1.2);
    let physics = reader.bool()?;
    let show_in_armor = reader.bool()?;
    let bounce_multiplier = clamp(reader.f32()?, 0.0, 0.5);
    let floppiness = clamp(reader.f32()?, 0.25, 1.0);
    let x_offset = clamp(reader.f32()?, -1.0, 1.0);
    let y_offset = clamp(reader.f32()?, -1.0, 1.0);
    let z_offset = clamp(reader.f32()?, -1.0, 0.0);
    let uniboob = reader.bool()?;
    let cleavage = clamp(reader.f32()?, 0.0, 0.1);
    let mut uvs = crate::config::default_uvs();
    for (index, layout) in uvs.iter_mut().enumerate() {
        *layout = read_legacy_layout(&mut reader, index)?;
    }
    if reader.remaining() != 0 {
        return None;
    }
    Some((
        uuid,
        AvatarConfig {
            gender,
            breasts: Breasts {
                x_offset,
                y_offset,
                z_offset,
                bust_size,
                cleavage,
                physics,
                uniboob,
                bounce_multiplier,
                floppiness,
            },
            uvs,
            sounds: Sounds { hurt, voice_pitch },
            show_in_armor,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LAYOUT_SKIN_LEFT, UvQuad};

    /// The configuration replies live in one process-wide queue, so the two
    /// tests that touch it must not interleave.
    static QUEUE_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn default_male_is_one_byte() {
        assert_eq!(encode_compact(&AvatarConfig::default()), vec![0]);
    }

    #[test]
    fn full_round_trip() {
        let mut config = AvatarConfig::default();
        config.gender = Gender::Female;
        config.breasts.bust_size = 0.55;
        config.breasts.cleavage = 0.04;
        config.uvs[LAYOUT_SKIN_LEFT][0] = Some(UvQuad {
            x1: 1,
            y1: 2,
            x2: 3,
            y2: 4,
        });
        let bytes = encode_compact(&config);
        assert_eq!(decode_compact(&bytes), Some(config));
    }

    #[test]
    fn sync_uuid_is_big_endian() {
        let config = AvatarConfig::default();
        let bytes = encode_clientbound_sync(0x0123456789abcdef, &config);
        assert_eq!(
            decode_clientbound_sync(&bytes),
            Some((0x0123456789abcdef, config))
        );
    }

    #[test]
    fn common_version_reply_is_the_version_array() {
        assert_eq!(common_version_reply(), vec![1, 1]);
    }

    #[test]
    fn common_register_reply_lists_the_play_channels() {
        let bytes = common_register_reply(1);
        let mut reader = Reader::new(&bytes);
        assert_eq!(reader.varint(), Some(1));
        assert_eq!(reader.string().as_deref(), Some("play"));
        assert_eq!(reader.varint(), Some(4));
        assert_eq!(
            reader.string().as_deref(),
            Some(CHANNEL_SYNC_CLIENTBOUND)
        );
        assert_eq!(
            reader.string().as_deref(),
            Some(CHANNEL_SYNC_SERVERBOUND)
        );
        assert_eq!(
            reader.string().as_deref(),
            Some(LEGACY_CHANNEL_SYNC_CLIENTBOUND)
        );
        assert_eq!(
            reader.string().as_deref(),
            Some(LEGACY_CHANNEL_SYNC_SERVERBOUND)
        );
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn legacy_sync_round_trip() {
        let mut config = AvatarConfig::default();
        config.gender = Gender::Female;
        config.breasts.bust_size = 0.55;
        config.breasts.cleavage = 0.04;
        config.breasts.bounce_multiplier = 0.25;
        config.breasts.floppiness = 0.6;
        config.sounds.hurt = false;
        config.sounds.voice_pitch = 1.1;
        config.show_in_armor = false;
        config.uvs[LAYOUT_SKIN_LEFT][0] = Some(UvQuad {
            x1: 1,
            y1: 2,
            x2: 3,
            y2: 4,
        });
        let bytes = encode_legacy_sync(0x0123456789abcdef, &config);
        assert_eq!(
            decode_legacy_clientbound_sync(&bytes),
            Some((0x0123456789abcdef, config))
        );
    }

    #[test]
    fn legacy_sync_writes_all_five_faces_of_every_layout() {
        let config = AvatarConfig::default();
        let bytes = encode_legacy_sync(0, &config);
        let mut reader = Reader::new(&bytes);
        assert_eq!(reader.uuid(), Some(0));
        assert_eq!(reader.varint(), Some(config.gender.wire()));
        assert_eq!(reader.f32(), Some(config.breasts.bust_size));
        assert_eq!(reader.bool(), Some(config.sounds.hurt));
        assert_eq!(reader.f32(), Some(config.sounds.voice_pitch));
        assert_eq!(reader.bool(), Some(config.breasts.physics));
        assert_eq!(reader.bool(), Some(config.show_in_armor));
        assert_eq!(reader.f32(), Some(config.breasts.bounce_multiplier));
        assert_eq!(reader.f32(), Some(config.breasts.floppiness));
        assert_eq!(reader.f32(), Some(config.breasts.x_offset));
        assert_eq!(reader.f32(), Some(config.breasts.y_offset));
        assert_eq!(reader.f32(), Some(config.breasts.z_offset));
        assert_eq!(reader.bool(), Some(config.breasts.uniboob));
        assert_eq!(reader.f32(), Some(config.breasts.cleavage));
        for _ in 0..4 {
            assert_eq!(reader.varint(), Some(5));
            for direction in 0..5 {
                assert_eq!(reader.varint(), Some(direction));
                assert!(reader.varint().is_some());
                assert!(reader.varint().is_some());
                assert!(reader.varint().is_some());
                assert!(reader.varint().is_some());
            }
        }
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn config_registration_is_nul_separated_without_a_trailer() {
        let payload = String::from_utf8(config_registration_payload()).unwrap();
        assert_eq!(payload.split('\0').collect::<Vec<_>>(), CONFIG_CHANNELS);
        assert!(!payload.ends_with('\0'));
    }

    #[test]
    fn configuration_payloads_queue_one_reply_each() {
        let _guard = QUEUE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_config_actions();
        on_config_payload(CHANNEL_REGISTER, b"");
        on_config_payload(CHANNEL_REGISTER, b"");
        on_config_payload(CHANNEL_COMMON_VERSION, &[1, 1]);
        on_config_payload(CHANNEL_COMMON_REGISTER, &[1, 4, b'p', b'l', b'a', b'y']);
        on_config_payload(CHANNEL_HELLO_CLIENTBOUND, &[1, 2]);
        on_config_payload(CHANNEL_REGISTRY_SYNC, &[0, 0]);
        let actions = take_config_actions();
        let find = |channel: &str| {
            actions
                .iter()
                .find(|(queued, _)| queued == channel)
                .map(|(_, data)| data.clone())
        };
        assert_eq!(find(CHANNEL_REGISTER), Some(config_registration_payload()));
        assert_eq!(find(CHANNEL_COMMON_VERSION), Some(vec![1, 1]));
        assert_eq!(find(CHANNEL_COMMON_REGISTER), Some(common_register_reply(1)));
        assert_eq!(find(CHANNEL_HELLO_SERVERBOUND), Some(vec![2]));
        assert_eq!(find(CHANNEL_REGISTRY_SYNC_COMPLETE), Some(Vec::new()));
        assert_eq!(actions.len(), 5);
    }

    #[test]
    fn a_register_for_another_protocol_is_ignored() {
        let _guard = QUEUE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_config_actions();
        let mut payload = vec![1, 13];
        payload.extend_from_slice(b"configuration");
        on_config_payload(CHANNEL_COMMON_REGISTER, &payload);
        assert!(take_config_actions().is_empty());
    }
}
