// The multiplayer half: register the Fabric channels in play, send our own
// avatar config and learn the other players' configs.
//
// Fabric carries the configuration handshake during the configuration phase
// (see `crate::net::fgm_net`); the sync channels are play packets. The server
// broadcasts a player's config to everyone tracking that player, so one sync
// per connection plus one after each local change is enough.

use crate::game::Game;
use fgm_core::{net, state};
use horizon_protocol::packets::play::Clientbound;

/// Called at the head of `Game::poll_remote`: every frame while connected,
/// including the configuration phase.
pub(crate) fn poll(game: &mut Game<'_>) {
    // Only the window's `Game` owns the connection state. The integrated
    // server has no remote and must not reset what the client just built.
    if !game.native_window() {
        return;
    }
    // The stored config must be in the state before the first server sync or
    // render: the settings page and the tick also call this, but a frame can
    // run before either.
    crate::game::fgm::load(game);
    let Some(remote) = game.remote.as_ref() else {
        state::reset_if_active();
        return;
    };
    // The configuration channels are read on the network thread; the game
    // thread owns the writer, so the replies are flushed here.
    horizon_client::net::fgm_net::flush_config(&remote.session);
    if !remote.joined {
        // A reconfiguration starts the play phase again later; the first
        // connection has nothing to reset.
        state::reset_if_joined();
        return;
    }
    let profile = remote.profile;
    let players: Vec<(u32, u128)> = remote
        .level_players
        .iter()
        .map(|(&id, &uuid)| (id as u32, uuid))
        .collect();
    let actions = state::with_mut(|state| {
        state.set_own(profile);
        state.set_entity_map(players);
        state.play_actions(true)
    });
    if actions.register
        && !horizon_client::net::fgm_net::send_play(
            &remote.session,
            net::CHANNEL_REGISTER,
            &net::play_registration_payload(),
        )
    {
        state::with_mut(|state| state.play_failed(actions));
    }
    if actions.sync {
        let (data, legacy) = state::with(|state| {
            (
                net::encode_serverbound_sync(&state.config),
                state
                    .own_uuid
                    .map(|uuid| net::encode_legacy_sync(uuid, &state.config)),
            )
        });
        let sent = horizon_client::net::fgm_net::send_play(
            &remote.session,
            net::CHANNEL_SYNC_SERVERBOUND,
            &data,
        );
        let legacy_sent = legacy.is_none_or(|data| {
            horizon_client::net::fgm_net::send_play(
                &remote.session,
                net::LEGACY_CHANNEL_SYNC_SERVERBOUND,
                &data,
            )
        });
        if !sent || !legacy_sent {
            state::with_mut(|state| state.play_failed(actions));
        }
    }
}

/// A play packet: a `clientbound/sync` carries a player's UUID and config.
pub(crate) fn packet(packet: &Clientbound) {
    let Clientbound::CustomPayload(payload) = packet else {
        return;
    };
    let decoded = match payload.channel.as_str() {
        net::CHANNEL_SYNC_CLIENTBOUND => net::decode_clientbound_sync(&payload.data),
        net::LEGACY_CHANNEL_SYNC_CLIENTBOUND => {
            net::decode_legacy_clientbound_sync(&payload.data)
        }
        _ => None,
    };
    if let Some((uuid, config)) = decoded {
        state::with_mut(|state| state.apply_remote(uuid, config));
    }
}
