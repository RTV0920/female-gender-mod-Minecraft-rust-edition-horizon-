fn click_female_gender_mod(game: &crate::game::Game<'_>, mod_id: &str, button: usize) {
    if mod_id == fgm_core::MOD_ID {
        crate::game::fgm::click(game, button);
    }
}
