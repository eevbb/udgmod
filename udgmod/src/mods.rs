mod borderless_toggle;
mod common;
mod fix_screensize;
mod frame_advance;
mod freecam;
mod outfit;
mod shadows;
mod toko_skirt;

use windows::core::PCWSTR;

use crate::util::{get_module_addr, get_process_handle};

macro_rules! init_mods {
    ($handle:expr, $game_addr:expr, $register_update:expr, [$($($mod_name:ident)::*),* ,]) => {
        $(
            $($mod_name::)*init($handle, $game_addr, &mut $register_update)
                .inspect_err(|e| {
                    log::error!("Failed to initialize {} mod: {e:?}", stringify!($($mod_name::)*));
                })
                .ok();
        )*
    };
}

// Mod offsets are for BUILD_170706_202743_0944 (buildver.txt)

pub fn init(mut register_update: impl FnMut(fn())) {
    if let Ok(handle) = get_process_handle(std::process::id())
        .inspect_err(|e| log::error!("Failed to get process handle: {e:?}"))
        && let Ok(game_addr) = get_module_addr(PCWSTR::null())
            .inspect_err(|e| log::error!("Failed to get game base address: {e:?}"))
    {
        log::info!("Game base address: 0x{game_addr:X}");
        init_mods!(
            handle,
            game_addr,
            register_update,
            [
                borderless_toggle,
                common,
                fix_screensize,
                frame_advance,
                freecam,
                outfit,
                shadows,
                toko_skirt,
            ]
        );
    }
}
