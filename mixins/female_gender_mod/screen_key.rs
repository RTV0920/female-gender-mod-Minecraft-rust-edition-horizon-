fn screen_key_female_gender_mod(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    key: &str,
    repeat: bool,
    canceled: &mut bool,
) {
    crate::game::fgm::screen_key(game, screen, key, repeat, canceled);
}
