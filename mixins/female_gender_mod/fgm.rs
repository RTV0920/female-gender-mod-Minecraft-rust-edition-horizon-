// The game-thread half of Female Gender Mod: its Options > Mods page and the
// once-a-tick physics for the local player and every other player in view.
//
// The values, the page stack and the physics live in `fgm_core`; this module
// only reads Horizon's `Mob` state and the persisted `config` key.

use crate::game::{
    Game, Screen,
    native_hooks::{NativeMod, NativeScreenView},
};
use fgm_core::menu::{self, Menu, Widget, WidgetKind};
use fgm_core::physics::{Armor, EntityMotion, MotionPose};
use fgm_core::state;
use horizon_client::hud::HudBatch;
use horizon_client::mobs::Mob;
use horizon_client::pose::Pose;

/// The mod's row under Options > Mods.
pub(crate) fn list(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: fgm_core::MOD_ID.into(),
        title: format!("{} {}", fgm_core::TITLE, fgm_core::VERSION),
        description: "Bust size, physics, UV layouts and the Fabric mod's sync. Port of the \
                      LGPL-3.0 Female Gender Mod by WildfireMC."
            .into(),
        keys: vec![[
            "open_menu".into(),
            "Open Character Personalization".into(),
            "key.keyboard.h".into(),
        ]],
    });
}

/// The buttons of the mod's settings page, after loading the stored values
/// once.
pub(in crate::game) fn settings(game: &Game<'_>, buttons: &mut Vec<String>) {
    load(game);
    buttons.extend(state::labels());
}

/// A click on the settings page. A changed avatar config is stored and synced
/// on the next frame; client-only settings are only stored.
pub(in crate::game) fn click(game: &Game<'_>, button: usize) {
    load(game);
    if state::click(button) {
        save(game);
    }
}

/// The open-menu key (`H` by default). Play only: the title, the pause menu,
/// the inventory and every other screen keep their keys.
pub(in crate::game) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || state::menu_visible() || key != close_key(game) {
        return;
    }
    if !game.native_playing() || game.screen_over_playing() || game.remote_loading() {
        return;
    }
    // `native_tick` opens the screen, so this press cannot also reach it.
    state::menu_request_open();
}

/// Draws the character personalization panel over the Options screen and
/// follows an active slider drag.
pub(in crate::game) fn screen_render(
    game: &Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
) {
    if !state::menu_visible() || screen.id != "options" {
        return;
    }
    let (width, height) = (batch.width, batch.height);
    // The press started the drag; this keeps the value under the pointer even
    // between mouse events. The release (or the close) stores it.
    state::menu_drag(screen.mouse[0], width, height);
    let widgets = state::menu_widgets(width, height);
    draw_menu(game, batch, screen, &widgets, width, height);
}

/// The left button went down over the panel. Every button is taken while the
/// panel is up, so nothing under it (an Options row or key bind) reacts.
pub(in crate::game) fn screen_mouse_press(
    game: &Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if !state::menu_visible() || screen.id != "options" {
        return;
    }
    *canceled = true;
    if button != 0 {
        return;
    }
    let (width, height) = (screen.rect[2], screen.rect[3]);
    if state::menu_press(screen.mouse[0], screen.mouse[1], width, height).changed {
        save(game);
    }
}

/// The left button came up. A drag's last value is stored now, once.
pub(in crate::game) fn screen_mouse_release(
    game: &Game<'_>,
    screen: &NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if !state::menu_visible() || screen.id != "options" {
        return;
    }
    *canceled = true;
    if button == 0 {
        // The release can come before the frame that would sample the last
        // move: take its position now, then store whatever the drag changed.
        let (width, height) = (screen.rect[2], screen.rect[3]);
        state::menu_drag(screen.mouse[0], width, height);
        if state::menu_release() {
            save(game);
        }
    }
}

pub(in crate::game) fn screen_wheel(
    screen: &NativeScreenView<'_>,
    canceled: &mut bool,
) {
    if state::menu_visible() && screen.id == "options" {
        *canceled = true;
    }
}

/// A key over the panel: Escape and the open-menu key close it, Tab moves to
/// the next tab. Every key is taken, so none reaches the Options screen.
pub(in crate::game) fn screen_key(
    game: &Game<'_>,
    screen: &NativeScreenView<'_>,
    key: &str,
    repeat: bool,
    canceled: &mut bool,
) {
    if !state::menu_visible() || screen.id != "options" {
        return;
    }
    *canceled = true;
    if repeat {
        return;
    }
    if state::menu_key(key, &close_key(game)).changed {
        save(game);
    }
}

/// A typed character while the panel is up: the screen has no text box, so it
/// goes nowhere.
pub(in crate::game) fn screen_char(
    screen: &NativeScreenView<'_>,
    canceled: &mut bool,
) {
    if state::menu_visible() && screen.id == "options" {
        *canceled = true;
    }
}

/// `LivingEntity.tick` for every player: the physics reads position, velocity,
/// body rotation, the walk animation, the pose and the current swing.
pub(in crate::game) fn tick(game: &mut Game<'_>) {
    // The integrated server's `Game` ticks too, and it shares the process-wide
    // state. Its player model must not advance (or read) the client physics.
    if !game.native_window() {
        return;
    }
    load(game);
    let mut motions = Vec::new();
    if let Some(model) = game.player_model.as_ref() {
        motions.push((0u32, motion(model)));
    }
    for mob in &game.mobs {
        // Mannequins have their own config type in the original mod; only
        // players participate in the player sync.
        if &*mob.kind.id != "minecraft:player" {
            continue;
        }
        motions.push((mob.id, motion(mob)));
    }
    if motions.is_empty() {
        return;
    }
    state::with_mut(|state| {
        for (id, (motion, armor)) in motions {
            state.tick_physics(id, &motion, armor);
        }
    });
}

pub(in crate::game) fn load(game: &Game<'_>) {
    if state::loaded() {
        return;
    }
    state::load(
        game.native_setting(fgm_core::MOD_ID, fgm_core::STORAGE_KEY)
            .as_deref(),
    );
}

/// Runs once a frame, paused or not: opens the character personalization
/// screen on the frame after the key, and closes it when Done, Escape or the
/// key asked. The panel rides the Options screen, which gives it the pointer
/// and the input hooks.
pub(in crate::game) fn frame(game: &mut Game<'_>) {
    load(game);
    if state::menu_visible() {
        if state::menu_close_requested() || game.screen != Screen::Options {
            close_menu(game);
        }
    }
    if !state::menu_take_open_request() {
        return;
    }
    if !game.native_playing() || game.screen_over_playing() || game.remote_loading() {
        return;
    }
    game.open_options(false);
    game.capture(false);
    state::menu_open();
}

fn close_menu(game: &mut Game<'_>) {
    let changed = state::menu_close();
    if game.screen == Screen::Options {
        game.screen = Screen::Playing;
        game.capture(true);
    }
    if changed {
        save(game);
    }
}

fn close_key(game: &Game<'_>) -> String {
    game.native_mod_key(fgm_core::MOD_ID, "open_menu", "key.keyboard.h")
}

fn save(game: &Game<'_>) {
    game.native_set_setting(fgm_core::MOD_ID, fgm_core::STORAGE_KEY, &state::save_text());
}

const PANEL_FILL: u32 = 0xF2181922;
const PANEL_HEADER: u32 = 0xFF252738;
const PANEL_BORDER: u32 = 0xFF454864;
const SLOT: u32 = 0xFF2A2C3A;
const SLOT_HOVER: u32 = 0xFF373A4E;
const TAB_ACTIVE: u32 = 0xFF443259;
const ACCENT: u32 = 0xFF9B6BD6;
const ACCENT_HOVER: u32 = 0xFFB183F0;
const ON_FILL: u32 = 0xFF2F6240;
const OFF_FILL: u32 = 0xFF23242E;
const TRACK: u32 = 0xFF0A0B10;
const HANDLE: u32 = 0xFFF2F2F8;
const TEXT: u32 = 0xFFFFFFFF;
const TEXT_DIM: u32 = 0xFFC9CBD8;
const TEXT_OFF: u32 = 0xFF70727F;
const DISABLED_FILL: u32 = 0xFF20212B;
const DISABLED_TRACK: u32 = 0xFF55576A;

fn draw_menu(
    game: &Game<'_>,
    batch: &mut HudBatch,
    screen: &NativeScreenView<'_>,
    widgets: &[Widget],
    width: f32,
    height: f32,
) {
    let [x, y, w, h] = Menu::panel(width, height);
    // A dark veil over the Options page, then the panel itself.
    game.native_fill(batch, [0.0, 0.0, width, height], 0xE0000000);
    game.native_fill(batch, [x + 3.0, y + 3.0, w, h], 0x60000000);
    game.native_fill(batch, [x - 2.0, y - 2.0, w + 4.0, h + 4.0], PANEL_BORDER);
    game.native_fill(batch, [x, y, w, h], PANEL_FILL);
    game.native_fill(batch, [x, y, w, 24.0], PANEL_HEADER);
    game.native_fill(batch, [x, y + 24.0, w, 1.0], ACCENT);
    for widget in widgets {
        draw_widget(game, batch, widget, screen.mouse);
    }
    game.native_text(batch, menu::TITLE, [x + 8.0, y + 7.0], TEXT, true);
    game.native_text(
        batch,
        "H / Esc close    Tab: next tab",
        [x + 10.0, y + h - 18.0],
        TEXT_OFF,
        false,
    );
}

fn draw_widget(game: &Game<'_>, batch: &mut HudBatch, widget: &Widget, mouse: [f32; 2]) {
    let [x, y, w, h] = widget.rect;
    let hovered = widget.enabled && widget.contains(mouse[0], mouse[1]);
    match widget.kind {
        WidgetKind::Tab(_) => {
            let active = !widget.enabled;
            let fill = if active {
                TAB_ACTIVE
            } else if hovered {
                SLOT_HOVER
            } else {
                SLOT
            };
            game.native_fill(batch, widget.rect, fill);
            if active {
                game.native_fill(batch, [x, y, w, 2.0], ACCENT);
            }
            centered_text(
                game,
                batch,
                &widget.label,
                x,
                y + 6.0,
                w,
                if active { TEXT } else { TEXT_DIM },
                true,
            );
        }
        WidgetKind::Done => {
            game.native_fill(
                batch,
                widget.rect,
                if hovered { ACCENT_HOVER } else { ACCENT },
            );
            centered_text(game, batch, &widget.label, x, y + 6.0, w, TEXT, true);
        }
        WidgetKind::Toggle(_) => {
            let fill = if !widget.enabled {
                DISABLED_FILL
            } else if widget.on {
                ON_FILL
            } else if hovered {
                SLOT_HOVER
            } else {
                SLOT
            };
            let fg = if widget.enabled { TEXT } else { TEXT_OFF };
            game.native_fill(batch, widget.rect, fill);
            if widget.enabled && widget.on {
                game.native_fill(batch, [x, y, 3.0, h], ACCENT);
            }
            centered_text(game, batch, &widget.label, x, y + 6.0, w, fg, true);
        }
        WidgetKind::Slider(_) => {
            let (fill, handle, fg) = if widget.enabled {
                (ACCENT, HANDLE, TEXT)
            } else {
                (DISABLED_TRACK, TEXT_OFF, TEXT_OFF)
            };
            game.native_fill(
                batch,
                widget.rect,
                if hovered { SLOT_HOVER } else { SLOT },
            );
            let track_x = x + 4.0;
            let track_w = (w - 8.0).max(1.0);
            let track_y = y + 15.0;
            game.native_fill(batch, [track_x, track_y, track_w, 3.0], TRACK);
            game.native_fill(
                batch,
                [track_x, track_y, track_w * widget.fraction, 3.0],
                fill,
            );
            let handle_x = track_x + track_w * widget.fraction - 3.0;
            game.native_fill(batch, [handle_x, y + 11.0, 6.0, 9.0], handle);
            centered_text(game, batch, &widget.label, x, y + 2.0, w, fg, true);
        }
    }
}

fn centered_text(
    game: &Game<'_>,
    batch: &mut HudBatch,
    text: &str,
    x: f32,
    y: f32,
    width: f32,
    rgb: u32,
    shadow: bool,
) {
    let text_width = game.native_text_width(text);
    game.native_text(
        batch,
        text,
        [x + (width - text_width) * 0.5, y],
        rgb,
        shadow,
    );
}

fn motion(m: &Mob) -> (EntityMotion, Armor) {
    let pose = match m.extra.avatar_pose {
        Pose::Crouching => MotionPose::Crouching,
        Pose::Sleeping => MotionPose::Sleeping,
        Pose::Standing => MotionPose::Standing,
        Pose::Swimming | Pose::FallFlying | Pose::SpinAttack => MotionPose::Other,
    };
    (
        EntityMotion {
            position: [m.position.x, m.position.y, m.position.z],
            velocity: [m.velocity.x, m.velocity.y, m.velocity.z],
            body_yaw: m.body_yaw,
            previous_body_yaw: m.previous_body_yaw,
            limb_position: m.limb_position,
            limb_speed: m.limb_speed,
            pose,
            swinging: m.ai.swinging,
            swing_right_arm: m
                .extra
                .swing_right_arm
                .unwrap_or(!m.extra.main_hand_left),
            age: m.age,
        },
        Armor::from_chest(m.extra.chest.as_deref()),
    )
}
