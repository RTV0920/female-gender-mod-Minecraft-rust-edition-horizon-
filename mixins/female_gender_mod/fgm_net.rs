// The network-thread and game-thread part of the Fabric channel handshake.
//
// Configuration payloads are read on the net thread. The session's writer is
// published when the connection starts, so `receive_brand` answers the
// server's registration and hello tasks at once; without that, the server's
// `CONFIGURE` event runs before the client is known to accept
// `female_gender_mod:clientbound/hello` and it never sends the hello. Play
// packets are written by the game thread directly.

use horizon_protocol::connection::ConnWriter;
use horizon_protocol::packets::{configuration as cfg, play};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, OnceLock};

/// The session's writer: Horizon keeps one per connection and hands the same
/// `Arc` to the network thread. Cloning it is safe; the writer stays alive
/// even if the game thread drops its `Session`.
pub type SharedWriter = Arc<Mutex<Option<ConnWriter>>>;

fn config_writer() -> &'static Mutex<Option<SharedWriter>> {
    static WRITER: OnceLock<Mutex<Option<SharedWriter>>> = OnceLock::new();
    WRITER.get_or_init(|| Mutex::new(None))
}

/// Called at the head of `horizon_client::net::Session::start`: remembers the
/// writer the new connection will configure and play through.
pub fn remember_session_writer(writer: &SharedWriter) {
    let mut slot = config_writer()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *slot = Some(writer.clone());
}

/// Sends queued configuration replies from the network thread. The server
/// blocks its configuration tasks until the matching reply arrives, so the
/// game thread's next frame is too late.
pub fn flush_config_now() {
    let actions = fgm_core::net::take_config_actions();
    if actions.is_empty() {
        return;
    }
    let writer = config_writer()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let Some(writer) = writer else {
        return;
    };
    let mut writer = writer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(writer) = writer.as_mut() else {
        return;
    };
    for (channel, data) in actions {
        let packet = cfg::Serverbound::CustomPayload(cfg::CustomPayload { channel, data });
        if writer.send(&packet.encode()).is_err() {
            return;
        }
    }
}

/// Sends the queued configuration replies, if the session is still
/// configuring. The writer lock is held across the `configuring` check, as
/// `FinishConfiguration` flips it with the lock held: a reply either goes out
/// in the configuration phase or is dropped, never in play.
///
/// The queue is drained even when the session is not configuring: a reply
/// left over from an earlier connection must not reach a later server.
pub fn flush_config(session: &super::Session) {
    let actions = fgm_core::net::take_config_actions();
    if actions.is_empty() {
        return;
    }
    let mut writer = session
        .writer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !session.configuring.load(Ordering::Relaxed) {
        return;
    }
    let Some(writer) = writer.as_mut() else {
        return;
    };
    for (channel, data) in actions {
        let packet = cfg::Serverbound::CustomPayload(cfg::CustomPayload { channel, data });
        if writer.send(&packet.encode()).is_err() {
            return;
        }
    }
}

/// Sends one play-phase custom payload. False while configuring or without a
/// writer; the caller retries on its next frame.
pub fn send_play(session: &super::Session, channel: &str, data: &[u8]) -> bool {
    let packet = play::Serverbound::CustomPayload(cfg::CustomPayload {
        channel: channel.to_owned(),
        data: data.to_vec(),
    })
    .encode();
    let mut writer = session
        .writer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if session.configuring.load(Ordering::Relaxed) {
        return false;
    }
    match writer.as_mut() {
        Some(writer) => writer.send(&packet).is_ok(),
        None => false,
    }
}
