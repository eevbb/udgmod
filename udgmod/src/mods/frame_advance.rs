use parking_lot::RwLock;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_F10, VK_F11};

use crate::{
    hack::patch,
    util::{asmut, asref, init, is_key_pressed},
};

init!(
    [
        IN_MENULOOP<u8>(0x0080_6062);
        ENABLE_MENULOOP(0x0011_5252) [0xc6, 0x05, 0x09, 0x0e, 0x6f, 0x00, 0x01];
        DISABLE_MENULOOP(0x0011_5293) [0xc6, 0x05, 0xc8, 0x0d, 0x6f, 0x00, 0x00];
    ]
    start: start;
    update: update;
);

fn start() {
    // This is the game variable we're coopting for pausing the game, so patch the places where the
    // game modifies it.
    patch!(try ENABLE_MENULOOP([0x90; 7])).unwrap_or_else(|e| {
        log::error!("Failed to patch ENABLE_MENULOOP: {e:?}");
    });
    patch!(try DISABLE_MENULOOP([0x90; 7])).unwrap_or_else(|e| {
        log::error!("Failed to patch DISABLE_MENULOOP: {e:?}");
    });
}

fn update() {
    if let Some(in_menu_loop) = asmut!(IN_MENULOOP) {
        let mut frames_left = FRAMES_LEFT.write();
        if *frames_left > 0 {
            *frames_left -= 1;
            if *frames_left == 0 {
                *in_menu_loop = 1;
            }
        }

        if is_key_pressed(VK_F10) {
            *in_menu_loop = u8::from(*in_menu_loop == 0);
        }

        if is_key_pressed(VK_F11) {
            *in_menu_loop = 0;
            *frames_left = 1;
        }
    }
}

pub fn is_paused() -> bool {
    asref!(IN_MENULOOP).map_or(false, |&v| v != 0)
}

static FRAMES_LEFT: RwLock<u32> = RwLock::new(0);
