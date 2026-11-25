fn packet_female_gender_mod(packet: &horizon_protocol::packets::play::Clientbound) {
    crate::game::remote::fgm_remote::packet(packet);
}
