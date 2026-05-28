use std::ffi::c_void;

use glam::{IVec2, Vec2, Vec3};
use parking_lot::RwLock;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VIRTUAL_KEY, VK_F7, VK_F8, VK_H, VK_I, VK_J, VK_K, VK_L, VK_O, VK_U, VK_Y,
};

use crate::{
    hack::{func, trampoline},
    model::{CamData, PchrData, PchrKind, RenderObj},
    mods::frame_advance,
    util::{asmut, asref, init, is_key_held, is_key_pressed, toggle},
};

init!(
    [
        GAME_UNFOCUS<u8>(0x0080_2be7);
        CURSOR_POS<IVec2>(0x0083_40f4);
        SCREEN_SIZE<IVec2>(0x0083_3d1c);
        CAM_DATA<CamData>(0x0080_2cb0);
        RESET_CURSOR_POS<fn()>(0x0011_6f90);
        GET_PCHR<fn(PchrKind, bool) -> *mut PchrData>(0x000b_49c0);
        PREPARE_RENDER<fn(*mut RenderObj)>(0x0011_8910) => prepare_render;
        CAM_UPDATE<fn()>(0x0012_4b20) => cam_update;
        RENDER_UI<fn(*mut c_void)>(0x0011_e2d0) => render_ui;
        RENDER_DIALOGUE<fn(*mut c_void)>(0x0011_e100) => render_dialogue;
    ]
    update: update;
);

fn update() {
    if is_key_pressed(VK_F7) {
        toggle!(HIDE_WEAPON);
    }

    if is_key_pressed(VK_F8) {
        toggle_freecam();
    }

    if let Some(cam) = asmut!(CAM_DATA) {
        // It's current year and we're not playing on a PSP anymore!
        cam.cull_radius = cam.far_plane;
    }

    // The regular update function isn't called when the game is paused so call it here manually.
    if frame_advance::is_paused() {
        update_freecam();
    }
}

extern "C" fn prepare_render(obj: *mut RenderObj) {
    if let Some(obj) = unsafe { obj.as_mut() }
        && *FREECAM_ENABLED.read()
    {
        if *HIDE_WEAPON.read()
            && let Some(gun_pchr) =
                unsafe { func!(GET_PCHR(PchrKind::KOMARU_WEAPON, false)).as_ref() }
            && obj.mesh_data == gun_pchr.mesh_data
        {
            return;
        }

        obj.render_flags |= 0x40; // Disable proximity culling
    }

    trampoline!(PREPARE_RENDER(obj));
}

extern "C" fn cam_update() {
    update_freecam();
    trampoline!(CAM_UPDATE());
}

extern "C" fn render_ui(ui: *mut c_void) {
    // Disable UI when freecam is enabled.
    if !*FREECAM_ENABLED.read() {
        trampoline!(RENDER_UI(ui));
    }
}

extern "C" fn render_dialogue(dialogue: *mut c_void) {
    // Disable UI when freecam is enabled.
    if !*FREECAM_ENABLED.read() {
        trampoline!(RENDER_DIALOGUE(dialogue));
    }
}

static FREECAM_ENABLED: RwLock<bool> = RwLock::new(false);
static FREECAM_DATA: RwLock<Option<CamData>> = RwLock::new(None);
static HIDE_WEAPON: RwLock<bool> = RwLock::new(false);

fn update_freecam() {
    if *FREECAM_ENABLED.read()
        && (frame_advance::is_paused() || asref!(GAME_UNFOCUS).is_some_and(|&v| v == 0))
        && let Some(freecam) = asmut!(FREECAM_DATA)
        && let Some(cam) = asmut!(CAM_DATA)
    {
        // Rotation
        let rot = &mut freecam.rot;
        if let Some(screen_size) = asref!(SCREEN_SIZE)
            && let Some(cursor_pos) = asref!(CURSOR_POS)
        {
            const SENSITIVITY: f32 = 0.001;

            // Get mouse delta
            let delta = cursor_pos - screen_size / 2;
            let delta = Vec2::new(delta.x as f32, delta.y as f32);

            // The game does this by itself normally but not during cutscenes or when paused. So we
            // do it ourselves!
            func!(RESET_CURSOR_POS());

            *rot -= delta * SENSITIVITY;
            rot.y = rot.y.clamp(-1.5, 1.5);
        }

        let pitchcos = rot.y.cos();
        let fwd = Vec3::new(rot.x.cos() * pitchcos, rot.x.sin() * pitchcos, rot.y.sin());
        let right = fwd.cross(Vec3::Z).normalize();
        let up = right.cross(fwd).normalize();

        // Movement
        let pos = &mut freecam.pos;
        {
            const DT: f32 = 1.0 / 60.0; // TODO: get actual delta time?
            const SPEED_SLOW: f32 = 1.0;
            const SPEED_NORMAL: f32 = 5.0;
            const SPEED_FAST: f32 = 25.0;

            fn axis_input(pos: VIRTUAL_KEY, neg: VIRTUAL_KEY) -> f32 {
                f32::from(i8::from(is_key_held(pos))) - f32::from(i8::from(is_key_held(neg)))
            }

            let speed = if is_key_held(VK_Y) {
                SPEED_SLOW
            } else if is_key_held(VK_H) {
                SPEED_FAST
            } else {
                SPEED_NORMAL
            };

            let delta = Vec3::ZERO
                + axis_input(VK_I, VK_K) * fwd
                + axis_input(VK_L, VK_J) * right
                + axis_input(VK_O, VK_U) * up;

            *pos += delta * (speed * DT);
        }

        freecam.focus = *pos + fwd;
        freecam.fwd = fwd;

        *cam = freecam.clone();
    }
}

fn toggle_freecam() {
    let enabled = toggle!(FREECAM_ENABLED);

    if enabled && let Some(cam) = asmut!(CAM_DATA) {
        // Avoid a big mouse jump on enabling freecam by resetting the cursor position immediately.
        func!(RESET_CURSOR_POS());

        // Recalculate camera forward and rotation for our purposes.
        cam.fwd = (cam.focus - cam.pos).normalize();
        cam.rot.x = cam.fwd.y.atan2(cam.fwd.x);
        cam.rot.y = cam.fwd.z.asin().clamp(-0.99, 0.99);

        *FREECAM_DATA.write() = Some(cam.clone());
    }
}
