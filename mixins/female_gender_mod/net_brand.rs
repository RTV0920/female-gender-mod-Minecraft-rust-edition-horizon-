fn config_female_gender_mod(
    payload: &horizon_protocol::packets::configuration::CustomPayload,
) {
    fgm_core::net::on_config_payload(&payload.channel, &payload.data);
    // The handler runs on the network thread during configuration; the server
    // waits for these replies before it runs its hello task, so send them now
    // rather than on the next game frame.
    crate::net::fgm_net::flush_config_now();
}
