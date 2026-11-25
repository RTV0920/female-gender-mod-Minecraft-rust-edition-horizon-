// The breast layer of Female Gender Mod, ported from `GenderLayer` and
// `WildfireModelRenderer`.
//
// Horizon draws a player's body, armour and layers through
// `mobs::render::render_with`; this module appends the two breast boxes after
// the normal body pass, once for each side. The geometry is the original's
// four by five by three pixel box, its layouts and its spring values; see
// `PORTING.md` for the armour layer approximation.

use super::{
    Atlas, Mob, MobData, Poses, Vertex, anim, armor_tint, base_transform, body, body_color,
    body_overlay, mark_oit, part_matrix, translucent_body,
};
use fgm_core::config::{UvLayout, default_armor_layout};
use fgm_core::physics::Armor;
use fgm_core::state;
use glam::{Mat3, Mat4, Quat, Vec3, Vec4};

/// `UVDirection`'s unit vectors, in ordinal order (east, west, down, up,
/// north).
const NORMALS: [[f32; 3]; 5] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, -1.0],
];

/// `GenderLayer.setupRender` and `setuptTransformations`: draws the player's
/// breasts at `m`'s current interpolated pose.
pub(crate) fn append(
    m: &Mob,
    alpha: f32,
    camera: glam::DVec3,
    atlas: Atlas<'_>,
    light: u32,
    data: &MobData,
    out: &mut Vec<Vertex>,
) {
    // Mannequins have their own config type in the original; only players
    // share the Fabric sync this port implements.
    if &*m.kind.id != "minecraft:player" {
        return;
    }
    let armor = Armor::from_chest(m.extra.chest.as_deref());
    let info = state::with(|state| state.render_info(m.id, alpha));
    let Some(info) = info else {
        return;
    };
    if !info.visible(armor) {
        return;
    }
    let (model_key, texture) = body(m);
    let Some(uv) = atlas(&texture).or_else(|| atlas(&m.kind.texture)) else {
        return;
    };
    let model = m
        .kind
        .layer(&model_key)
        .cloned()
        .unwrap_or_else(|| m.kind.model.clone());
    let s = anim::state(m, alpha);
    let mut poses = Poses::new(&model);
    anim::setup(m, &s, &mut poses, &data.animations);
    let Some(body_matrix) = part_matrix(&model, base_transform(m, alpha, camera), &poses, "body", 0.0)
    else {
        return;
    };

    let bounce = info.bounce_enabled(armor);
    let chestplate = info.chestplate_occupied(armor);
    let covered = info.armor_layer_visible(armor);
    let breathing = info.breathing(armor, !m.fluids.eye_in_water);
    let has_jacket = m.extra.hidden_model_parts & 0x02 == 0;
    let size = info.breast_size();
    let z_offset = info.z_offset();
    let outward_angle = info.outward_angle();
    let uniboob = info.config.breasts.uniboob;
    let offsets = [
        round_offset(info.config.breasts.x_offset),
        -round_offset(info.config.breasts.y_offset),
        -round_offset(info.config.breasts.z_offset),
    ];
    let age_in_ticks = m.age as f32 + alpha;
    let color = body_color(m, alpha);
    let layer = body_overlay(m, alpha);
    // `GenderArmorLayer`: a chestplate that covers the breasts is drawn with
    // the armour's own equipment texture (and the default armour UVs), not the
    // player's skin. This follows `armor.covers` rather than
    // `chestplate_occupied`: the physics override must not make the layer fall
    // back to skin. Without a texture to use, hide the layer under the
    // chestplate rather than draw bare skin through it.
    let armor = if covered { armor_texture(m, atlas) } else { None };
    if covered && armor.is_none() {
        return;
    }
    let start = out.len();

    for left in [true, false] {
        let (position_x, position_y, bounce_rotation) = if left || uniboob {
            (
                info.values.left.position_x,
                info.values.left.position_y,
                info.values.left.bounce_rotation,
            )
        } else {
            (
                info.values.right.position_x,
                info.values.right.position_y,
                info.values.right.bounce_rotation,
            )
        };
        let mut mtx = body_matrix;
        if bounce {
            mtx *= Mat4::from_translation(Vec3::new(position_x / 32.0, position_y / 32.0, 0.0));
        }
        mtx *= Mat4::from_translation(Vec3::new(
            if left { offsets[0] } else { -offsets[0] } * 0.0625,
            0.05625 + offsets[1] * 0.0625,
            z_offset - 0.125 + offsets[2] * 0.0425,
        ));
        if !uniboob {
            mtx *= Mat4::from_translation(Vec3::new(if left { -0.125 } else { 0.125 }, 0.0, 0.0));
        }
        if bounce {
            mtx *= Mat4::from_rotation_y(bounce_rotation.to_radians());
        }
        if !uniboob {
            mtx *= Mat4::from_translation(Vec3::new(if left { 0.125 } else { -0.125 }, 0.0, 0.0));
        }
        let mut rotation = size;
        if bounce {
            mtx *= Mat4::from_translation(Vec3::new(0.0, -0.035 * size, 0.0));
            rotation -= position_y / 12.0;
        }
        rotation = rotation.min(size + 0.2).min(1.0);
        if chestplate {
            mtx *= Mat4::from_translation(Vec3::new(0.0, 0.0, 0.01));
        }
        let mut quat = Quat::from_rotation_y(
            (if left { outward_angle } else { -outward_angle }).to_radians(),
        ) * Quat::from_rotation_x((-35.0 * rotation).to_radians());
        if breathing {
            let f5 = -(age_in_ticks * 0.09).cos() * 0.45 + 0.45;
            quat *= Quat::from_rotation_x(f5.to_radians());
        }
        mtx *= Mat4::from_quat(quat);
        mtx *= Mat4::from_scale(Vec3::new(0.9995, 1.0, 1.0));

        let x = if left { -4.0 } else { 0.0 };
        if let Some((armor_uv, tint)) = armor {
            // `GenderArmorLayer.setupTransformations`: the armour box sits a
            // little higher, further forward and wider than the skin layer.
            if has_jacket {
                mtx *= Mat4::from_translation(Vec3::new(0.0, 0.0, -0.015))
                    * Mat4::from_scale(Vec3::splat(1.05));
            }
            mtx *= Mat4::from_translation(Vec3::new(
                if left { 0.001 } else { -0.001 },
                0.015,
                -0.015,
            ));
            mtx *= Mat4::from_scale(Vec3::new(1.05, 1.0, 1.0));
            push_box(
                out,
                mtx,
                &default_armor_layout(if left { 0 } else { 1 }),
                armor_uv,
                x,
                light,
                0,
                tint,
                [64.0, 32.0],
            );
        } else {
            let skin_layout = if left { 0 } else { 1 };
            let overlay_layout = if left { 2 } else { 3 };
            push_box(
                out,
                mtx,
                &info.config.uvs[skin_layout],
                uv,
                x,
                light,
                layer,
                color,
                [64.0, 64.0],
            );
            if has_jacket {
                let jacket = mtx
                    * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.015))
                    * Mat4::from_scale(Vec3::splat(1.05));
                push_box(
                    out,
                    jacket,
                    &info.config.uvs[overlay_layout],
                    uv,
                    x,
                    light,
                    layer,
                    color,
                    [64.0, 64.0],
                );
            }
        }
    }
    if translucent_body(m) {
        mark_oit(&mut out[start..]);
    }
}

/// `WildfireHelper.round(value, 1)`: Java's `Math.round` is
/// `floor(value * 10 + 0.5)`, which matters for negative halves.
fn round_offset(value: f32) -> f32 {
    (value * 10.0 + 0.5).floor() / 10.0
}

/// The five face quads of a breast box, in the original's vertex order. The
/// box is four by five by three pixels at `x`; the six-sided source model has
/// a south face too, but `BreastModelBox` allocates only the five the layouts
/// name.
fn faces(x: f32) -> [[[f32; 3]; 4]; 5] {
    let f = x + 4.0;
    let (y, f1) = (0.0, 5.0);
    let (z, f2) = (0.0, 3.0);
    let vertex = [f, y, z];
    let vertex1 = [f, f1, z];
    let vertex2 = [x, f1, z];
    let vertex3 = [x, y, f2];
    let vertex4 = [f, y, f2];
    let vertex5 = [f, f1, f2];
    let vertex6 = [x, f1, f2];
    let vertex7 = [x, y, z];
    [
        [vertex4, vertex, vertex1, vertex5],
        [vertex7, vertex3, vertex6, vertex2],
        [vertex4, vertex3, vertex7, vertex],
        [vertex1, vertex2, vertex6, vertex5],
        [vertex, vertex7, vertex2, vertex1],
    ]
}

/// `GenderArmorLayer`: the chestplate texture and tint for the armour breasts.
/// The original reads `IBreastArmorTexture` from the armour's data pack; this
/// port uses the vanilla equipment layer texture and the default armour UVs,
/// and falls back to hiding the layer when the atlas has no such texture.
fn armor_texture(m: &Mob, atlas: Atlas<'_>) -> Option<([f32; 4], u32)> {
    let item = m.extra.chest.as_deref()?;
    let name = item.rsplit(':').next()?;
    let material = name.strip_suffix("_chestplate").unwrap_or(name);
    let asset = match material {
        "golden" => "gold",
        "turtle" => "turtle_scute",
        other => other,
    };
    let uv = atlas(&format!("entity/equipment/humanoid/{asset}"))?;
    let tint = if asset == "leather" {
        armor_tint(Some(0xA06540), m.extra.armor[1].as_ref())
    } else {
        0xFFFF_FFFF
    };
    Some((uv, tint))
}

/// `ModelBox.initQuads` and `renderBox`: present quads take the next face's
/// vertices (a `None` face shifts the later ones), all-zero quads take an
/// index but draw nothing, and each vertex's local UV is its pixel over the
/// texture size (64x64 skin, 64x32 armour).
#[allow(clippy::too_many_arguments)]
fn push_box(
    out: &mut Vec<Vertex>,
    matrix: Mat4,
    layout: &UvLayout,
    atlas: [f32; 4],
    x: f32,
    light: u32,
    layer: u32,
    color: u32,
    tex: [f32; 2],
) {
    let faces = faces(x);
    let (normal_matrix, uniform) = normal_matrix(matrix);
    let mut face = 0;
    for (direction, quad) in layout.iter().enumerate() {
        let Some(quad) = *quad else {
            continue;
        };
        let vertices = &faces[face];
        face += 1;
        if quad.is_unused() {
            continue;
        }
        let n = normal_matrix * Vec3::from_array(NORMALS[direction]);
        let n = if uniform { n } else { n.normalize_or_zero() };
        let packed = n.to_array().map(|v| (v.clamp(-1.0, 1.0) * 127.0) as i8);
        let normal = [packed[0], packed[1], packed[2], -127];
        let corners = [
            [quad.x2, quad.y1],
            [quad.x1, quad.y1],
            [quad.x1, quad.y2],
            [quad.x2, quad.y2],
        ];
        for (vertex, local) in vertices.iter().zip(corners) {
            let pos = (matrix
                * Vec4::new(vertex[0] / 16.0, vertex[1] / 16.0, vertex[2] / 16.0, 1.0))
            .truncate();
            out.push(Vertex {
                pos: pos.to_array(),
                uv: [
                    atlas[0] + (atlas[2] - atlas[0]) * (local[0] as f32 / tex[0]),
                    atlas[1] + (atlas[3] - atlas[1]) * (local[1] as f32 / tex[1]),
                ],
                layer,
                color,
                light,
                normal,
                glint_uv: [0.0; 2],
            });
        }
    }
}

/// `ModelNormals`: a uniform scale keeps the normal length, a nonuniform one
/// needs the inverse transpose and normalization, exactly as Horizon's model
/// renderer packs its normals.
fn normal_matrix(matrix: Mat4) -> (Mat3, bool) {
    let m = Mat3::from_mat4(matrix);
    let lengths = Vec3::new(
        m.x_axis.length(),
        m.y_axis.length(),
        m.z_axis.length(),
    );
    let uniform = lengths.min_element() > 0.0
        && lengths.max_element() - lengths.min_element() < 1.0e-6;
    (
        if uniform {
            m / lengths.x
        } else {
            m.inverse().transpose()
        },
        uniform,
    )
}
