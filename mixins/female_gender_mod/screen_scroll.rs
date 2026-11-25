fn screen_scroll_female_gender_mod(
    screen: &crate::game::native_hooks::NativeScreenView<'_>,
    canceled: &mut bool,
) {
    crate::game::fgm::screen_wheel(screen, canceled);
}
