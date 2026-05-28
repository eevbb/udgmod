use glam::IVec2;
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::GetClientRect,
};

use crate::{
    hack::{func, trampoline},
    util::{asmut, init},
};

init!([
    SCREEN_SIZE<IVec2>(0x083_3d1c);
    GET_HWND<fn() -> HWND>(0x0011_33e0);
    SET_SCREEN_SIZE<fn(u32, u32)>(0x0010_fa90) => set_screen_size;
]);

extern "C" fn set_screen_size(width: u32, height: u32) {
    // The game relies on the swap chain buffer size being the same as the window's client area size
    // but that isn't the case when using borderless windowed mode, which breaks mouse input and
    // maybe other things too.
    log::info!("Screen requested as {width}x{height}");

    trampoline!(SET_SCREEN_SIZE(width, height));

    let mut rect = RECT::default();
    if let Some(screen_size) = asmut!(SCREEN_SIZE)
        && unsafe { GetClientRect(func!(GET_HWND()), &raw mut rect) }
            .inspect_err(|e| log::warn!("Failed to get client rect: {e:?}"))
            .is_ok()
    {
        // Fix this static variable only.
        // Do not call the original function with the new size, that breaks stuff.
        screen_size.x = rect.right - rect.left;
        screen_size.y = rect.bottom - rect.top;
        log::info!(
            "Adjusted screen size to {}x{}",
            screen_size.x,
            screen_size.y
        );
    }
}
