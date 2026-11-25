fn settings_female_gender_mod(
    game: &crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    if mod_id == fgm_core::MOD_ID {
        crate::game::fgm::settings(game, buttons);
    }
}
