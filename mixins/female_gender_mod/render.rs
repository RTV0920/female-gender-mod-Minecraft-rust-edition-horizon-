fn render_female_gender_mod(
    m: &crate::mobs::Mob,
    alpha: f32,
    camera: glam::DVec3,
    atlas: &dyn Fn(&str) -> Option<[f32; 4]>,
    light: u32,
    data: &crate::mobs::MobData,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    crate::mobs::render::fgm_render::append(m, alpha, camera, atlas, light, data, out);
}
