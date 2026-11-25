fn screen_char_female_gender_mod(
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    canceled: &mut bool,
) {
    crate::game::fgm::screen_char(screen, canceled);
}
