use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        UI::WindowsAndMessaging::{
            GCLP_HICON, GCLP_HICONSM, GWL_STYLE, GetClassLongPtrW, GetSystemMetrics, HICON,
            ICON_BIG, ICON_SMALL, PostMessageW, SM_CXSCREEN, SM_CYSCREEN, SWP_ASYNCWINDOWPOS,
            SetWindowLongPtrW, SetWindowPos, WM_SETICON, WS_OVERLAPPEDWINDOW, WS_POPUP, WS_VISIBLE,
        },
    },
    core::PSTR,
};

use crate::{
    hack::{func, trampoline},
    util::{asmut, asref, init},
};

init!([
    IS_EXCLUSIVE_FULLSCREEN<u8>(0x0080_6064);
    WINDOW_MODE<WindowMode>(0x0082_a90c);
    WINDOW_SIZE<WindowSize>(0x0082_a8fa);
    SYS_SCREEN_WIDTH<i16>(0x0082_a90e);
    SYS_SCREEN_HEIGHT<i16>(0x0082_a910);
    WINDOW_RECT<RECT>(0x0080_6038);
    GET_HWND<fn() -> HWND>(0x0011_33e0);
    UPDATE_FULLSCREEN<fn(u8)>(0x0010_fe10);
    SET_WINDOW_TITLE<fn(PSTR)>(0x0011_3900) => set_window_title;
    SET_FULLSCREEN<fn(u8)>(0x0010_fcb0) => set_fullscreen;
]);

extern "C" fn set_window_title(title: PSTR) {
    if let Some(window_mode) = asmut!(WINDOW_MODE)
        && *window_mode == WindowMode::Borderless
    {
        let previous = *window_mode;
        *window_mode = WindowMode::Windowed;
        // This call does nothing if the setting is Borderless, so temporarily switch to Windowed
        // mode to set the title.
        trampoline!(SET_WINDOW_TITLE(title));
        *window_mode = previous;
    }

    trampoline!(SET_WINDOW_TITLE(title));
}

extern "C" fn set_fullscreen(mode: u8) {
    if let Some(is_exclusive_fullscreen) = asmut!(IS_EXCLUSIVE_FULLSCREEN)
        && let Some(window_mode) = asmut!(WINDOW_MODE)
        && let Some(window_size) = asref!(WINDOW_SIZE)
        && let Some(width) = asmut!(SYS_SCREEN_WIDTH)
        && let Some(height) = asmut!(SYS_SCREEN_HEIGHT)
        && let Some(window_rect) = asmut!(WINDOW_RECT)
    {
        *is_exclusive_fullscreen = 0;

        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

        let style = if *window_mode == WindowMode::Windowed {
            *window_mode = WindowMode::Borderless;
            *width = screen_width.try_into().unwrap();
            *height = screen_height.try_into().unwrap();

            window_rect.left = 0;
            window_rect.top = 0;

            WS_POPUP
        } else {
            *window_mode = WindowMode::Windowed;
            (*width, *height) = window_size.dimensions();

            window_rect.left = screen_width / 2 - i32::from(*width) / 2;
            window_rect.top = screen_height / 2 - i32::from(*height) / 2;

            WS_OVERLAPPEDWINDOW
        } | WS_VISIBLE;

        window_rect.right = window_rect.left + i32::from(*width);
        window_rect.bottom = window_rect.top + i32::from(*height);

        unsafe {
            let hwnd = func!(GET_HWND());
            SetWindowLongPtrW(hwnd, GWL_STYLE, style.0.cast_signed() as isize);
            SetWindowPos(
                hwnd,
                None,
                window_rect.left,
                window_rect.top,
                window_rect.right - window_rect.left,
                window_rect.bottom - window_rect.top,
                SWP_ASYNCWINDOWPOS,
            )
            .inspect_err(|e| log::warn!("SetWindowPos failed: {e:?}"))
            .ok();

            if *window_mode == WindowMode::Windowed {
                const WP_ICON_SMALL: WPARAM = WPARAM(ICON_SMALL as _);
                const WP_ICON_BIG: WPARAM = WPARAM(ICON_BIG as _);

                // Rapply the small icon since it doesn't load correctly if the game started in
                // borderless mode.
                // Reapply the big one too; if we reapply the small one only, the big one breaks...

                // They have to be unset first or it won't update sometimes.
                PostMessageW(Some(hwnd), WM_SETICON, WP_ICON_SMALL, LPARAM(0))
                    .inspect_err(|e| log::warn!("Failed to unset small icon: {e:?}"))
                    .ok();
                PostMessageW(Some(hwnd), WM_SETICON, WP_ICON_BIG, LPARAM(0))
                    .inspect_err(|e| log::warn!("Failed to unset big icon: {e:?}"))
                    .ok();

                let icon = HICON(GetClassLongPtrW(hwnd, GCLP_HICONSM) as _);
                log::info!("Reapplying small icon: {icon:?}");
                PostMessageW(Some(hwnd), WM_SETICON, WP_ICON_SMALL, LPARAM(icon.0 as _))
                    .inspect_err(|e| log::warn!("Failed to set small icon {icon:?}: {e:?}"))
                    .ok();
                let icon = HICON(GetClassLongPtrW(hwnd, GCLP_HICON) as _);
                log::info!("Reapplying big icon: {icon:?}");
                PostMessageW(Some(hwnd), WM_SETICON, WP_ICON_BIG, LPARAM(icon.0 as _))
                    .inspect_err(|e| log::warn!("Failed to set big icon {icon:?}: {e:?}"))
                    .ok();
            }
        };

        func!(UPDATE_FULLSCREEN(u8::from(
            *window_mode == WindowMode::Borderless
        )));
        return;
    }

    trampoline!(SET_FULLSCREEN(mode));
}

#[expect(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum WindowMode {
    Fullscreen = 0,
    Borderless = 1,
    Windowed = 2,
}

#[expect(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum WindowSize {
    Size544 = 0,
    Size720 = 1,
    Size1080 = 2,
}

impl WindowSize {
    fn dimensions(self) -> (i16, i16) {
        match self {
            WindowSize::Size544 => (960, 544),
            WindowSize::Size720 => (1280, 720),
            WindowSize::Size1080 => (1920, 1080),
        }
    }
}
