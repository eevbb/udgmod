mod assets;
mod borderless_toggle;
mod common;
mod fix_screensize;
mod frame_advance;
mod freecam;
mod outfit;
mod shadows;
mod toko_skirt;

use windows::core::PCWSTR;

use crate::util::{detect_version, get_module_addr, get_process_handle};

macro_rules! init_mods {
    (
        $handle:expr,
        $game_addr:expr,
        $version:expr,
        $register_update:expr,
        [$($($mod_name:ident)::*),* ,]
    ) => {
        $(
            $($mod_name::)*init($handle, $game_addr, $version, &mut $register_update)
                .inspect_err(|e| {
                    log::error!("Failed to initialize {} mod: {e:?}", stringify!($($mod_name::)*));
                })
                .ok();
        )*
    };
}

// Mod offsets are given as (EN, JP) pairs:
//   EN: BUILD_170706_202743_0944 (buildver.txt)
//   JP: BUILD_170706_202740_0944 (buildver.txt)

pub fn init(mut register_update: impl FnMut(fn())) {
    if let Ok(handle) = get_process_handle(std::process::id())
        .inspect_err(|e| log::error!("Failed to get process handle: {e:?}"))
        && let Ok(game_addr) = get_module_addr(PCWSTR::null())
            .inspect_err(|e| log::error!("Failed to get game base address: {e:?}"))
        && let Ok(version) = detect_version(game_addr)
            .inspect_err(|e| log::error!("Failed to detect game version, no mods loaded: {e:?}"))
    {
        log::info!("Game base address: 0x{game_addr:X}");
        log::info!("Detected game version: {version:?}");
        init_mods!(
            handle,
            game_addr,
            version,
            register_update,
            [
                assets,
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
