fn screen_mouse_release_female_gender_mod(
    game: &crate::game::Game<'_>,
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    crate::game::fgm::screen_mouse_release(game, screen, button, canceled);
}
