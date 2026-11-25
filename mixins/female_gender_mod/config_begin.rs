// Publishes the connection's writer so the payload hook can answer the
// configuration handshake from the network thread. `Session::start` creates
// the writer and hands the same `Arc` to the network thread; the session it
// returns still owns that writer, so a reference to it is all we need.
fn begin_female_gender_mod_session(session: &crate::net::Session) {
    crate::net::fgm_net::remember_session_writer(&session.writer);
}
