use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{MNC_CLOSE, SC_KEYMENU, WM_MENUCHAR, WM_SYSCOMMAND},
};

use crate::{
    hack::trampoline,
    mods::{borderless_toggle, frame_advance},
    util::init,
};

init!([
    WND_PROC<fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>(0x0011_4e60) => wnd_proc;
]);

extern "C" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_MENUCHAR if borderless_toggle::is_enabled() => {
            // This is just to prevent an annoying beep sound when pressing Alt+Enter to toggle
            // fullscreen...
            LRESULT((MNC_CLOSE.cast_signed() as isize) << 16)
        }
        WM_SYSCOMMAND if wparam.0 == SC_KEYMENU as usize && frame_advance::is_enabled() => {
            // Prevent the game from pausing when F10 is pressed, since we use that key to toggle
            // our own pause implementation.
            LRESULT(0)
        }
        _ => trampoline!(WND_PROC(hwnd, msg, wparam, lparam)),
    }
}
