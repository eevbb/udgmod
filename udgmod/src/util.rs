use eyre::{Result, bail};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::{OpenProcess, PROCESS_ALL_ACCESS, PROCESS_VM_READ, PROCESS_VM_WRITE},
        },
        UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VIRTUAL_KEY},
    },
    core::PCWSTR,
};

macro_rules! asref {
    ($var:ident) => {
        $var.read().as_ref()
    };
}
pub(crate) use asref;

macro_rules! asmut {
    ($var:ident) => {
        $var.write().as_mut()
    };
}
pub(crate) use asmut;

macro_rules! toggle {
    ($var:ident) => {{
        let mut guard = $var.write();
        let value = !*guard;
        *guard = value;
        value
    }};
}
pub(crate) use toggle;

pub fn is_key_held(vk_code: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(i32::from(vk_code.0)) < 0 }
}

pub fn is_key_pressed(vk_code: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(i32::from(vk_code.0)) & 1 != 0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameVersion {
    /// `BUILD_170706_202743_0944`
    En,
    /// `BUILD_170706_202740_0944`
    Jp,
}

const TIMESTAMP_EN: u32 = 0x595e_1eaf;
const TIMESTAMP_JP: u32 = 0x595e_1eac;

/// Reads the PE header `TimeDateStamp` of the loaded module.
pub fn get_module_timestamp(base: usize) -> u32 {
    unsafe {
        let e_lfanew = *((base + 0x3c) as *const u32) as usize;
        *((base + e_lfanew + 8) as *const u32)
    }
}

pub fn detect_version(base: usize) -> Result<GameVersion> {
    Ok(match get_module_timestamp(base) {
        TIMESTAMP_EN => GameVersion::En,
        TIMESTAMP_JP => GameVersion::Jp,
        other => {
            bail!("Unsupported game build (PE timestamp 0x{other:08X})");
        }
    })
}

pub fn get_module_addr(name: PCWSTR) -> Result<usize> {
    let module = unsafe { GetModuleHandleW(name) }?;

    if module.is_invalid() {
        bail!("Failed to get module handle");
    }

    Ok(module.0 as usize)
}

pub fn get_process_handle(pid: u32) -> Result<HANDLE> {
    Ok(unsafe {
        OpenProcess(PROCESS_ALL_ACCESS, false, pid)
            .or_else(|_| OpenProcess(PROCESS_VM_READ | PROCESS_VM_WRITE, false, pid))?
    })
}

macro_rules! init {
    (
        [$($tt:tt)*]
        $(start: $start:ident;)?
        $(update: $update:ident;)?
    ) => {
        init!(@define [] $($tt)*);

        static IS_ACTIVE: ::parking_lot::RwLock<bool> = ::parking_lot::RwLock::new(false);

        #[allow(dead_code)]
        pub fn is_enabled() -> bool {
            *IS_ACTIVE.read()
        }

        #[allow(unused_mut, unused_variables)]
        pub fn init(
            handle: ::windows::Win32::Foundation::HANDLE,
            game_addr: usize,
            version: crate::util::GameVersion,
            mut register_update: impl FnMut(fn())
        ) -> ::eyre::Result<()> {
            init!(@load (handle, game_addr, version) [] $($tt)*);
            $(register_update($update);)?
            $($start();)?
            *IS_ACTIVE.write() = true;
            Ok(())
        }
    };

    (@define [$($acc:tt)*]) => { $($acc)* };

    (@load ($handle:expr, $game_addr:expr, $version:expr) [$($acc:tt)*]) => { $($acc)* };

    // FuncHook
    (
        @define
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($en:expr, $jp:expr) => $hook:ident;
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<
                    crate::hack::FuncHook<extern "C" fn($($arg),*) $(-> $ret)?>
                > = ::parking_lot::RwLock::new(crate::hack::FuncHook::new(
                    stringify!($ident),
                    crate::hack::Offset { en: $en, jp: $jp },
                    $hook,
                ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr, $version:expr)
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($en:expr, $jp:expr) => $hook:ident;
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr, $version)
            [
                $($acc)*
                $ident.write().load($game_addr, $version)?;
            ]
            $($tail)*
        );
    };

    // Func
    (
        @define
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($en:expr, $jp:expr);
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<
                    crate::hack::Func<extern "C" fn($($arg),*) $(-> $ret)?>
                > = ::parking_lot::RwLock::new(crate::hack::Func::new(
                    stringify!($ident),
                    crate::hack::Offset { en: $en, jp: $jp },
                ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr, $version:expr)
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($en:expr, $jp:expr);
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr, $version)
            [
                $($acc)*
                $ident.write().load($game_addr, $version)?;
            ]
            $($tail)*
        );
    };

    // Patch
    (
        @define
        [$($acc:tt)*]
        $ident:ident($en:expr, $jp:expr) [$($bytes_en:literal),*] [$($bytes_jp:literal),*];
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<crate::hack::Patch<
                    { 0 $(+ ($bytes_en * 0) + 1)* }
                >> = ::parking_lot::RwLock::new(crate::hack::Patch::new(
                    stringify!($ident),
                    crate::hack::Offset { en: $en, jp: $jp },
                    [$($bytes_en),*],
                    [$($bytes_jp),*],
                ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr, $version:expr)
        [$($acc:tt)*]
        $ident:ident($en:expr, $jp:expr) [$($bytes_en:literal),*] [$($bytes_jp:literal),*];
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr, $version)
            [
                $($acc)*
                $ident.write().load($handle, $game_addr, $version)?;
            ]
            $($tail)*
        );
    };

    // Var
    (
        @define
        [$($acc:tt)*]
        $ident:ident<$ty:ty>($en:expr, $jp:expr);
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<crate::hack::Var<$ty>> =
                    ::parking_lot::RwLock::new(crate::hack::Var::new(
                        stringify!($ident),
                        crate::hack::Offset { en: $en, jp: $jp },
                    ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr, $version:expr)
        [$($acc:tt)*]
        $ident:ident<$ty:ty>($en:expr, $jp:expr);
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr, $version)
            [
                $($acc)*
                $ident.write().load($game_addr, $version)?;
            ]
            $($tail)*
        );
    };
}
pub(crate) use init;
