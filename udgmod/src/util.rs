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
            mut register_update: impl FnMut(fn())
        ) -> ::eyre::Result<()> {
            init!(@load (handle, game_addr) [] $($tt)*);
            $(register_update($update);)?
            $($start();)?
            *IS_ACTIVE.write() = true;
            Ok(())
        }
    };

    (@define [$($acc:tt)*]) => { $($acc)* };

    (@load ($handle:expr, $game_addr:expr) [$($acc:tt)*]) => { $($acc)* };

    // FuncHook
    (
        @define
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($offset:expr) => $hook:ident;
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<
                    crate::hack::FuncHook<extern "C" fn($($arg),*) $(-> $ret)?>
                > = ::parking_lot::RwLock::new(crate::hack::FuncHook::new(
                    stringify!($ident),
                    $offset,
                    $hook,
                ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr)
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($offset:expr) => $hook:ident;
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr)
            [
                $($acc)*
                $ident.write().load($game_addr)?;
            ]
            $($tail)*
        );
    };

    // Func
    (
        @define
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($offset:expr);
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<
                    crate::hack::Func<extern "C" fn($($arg),*) $(-> $ret)?>
                > = ::parking_lot::RwLock::new(crate::hack::Func::new(stringify!($ident), $offset));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr)
        [$($acc:tt)*]
        $ident:ident<fn($($arg:ty),*) $(-> $ret:ty)?>($offset:expr);
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr)
            [
                $($acc)*
                $ident.write().load($game_addr)?;
            ]
            $($tail)*
        );
    };

    // Patch
    (
        @define
        [$($acc:tt)*]
        $ident:ident($offset:expr) [$($bytes:literal),*];
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<crate::hack::Patch<
                    { 0 $(+ ($bytes * 0) + 1)* }
                >> = ::parking_lot::RwLock::new(crate::hack::Patch::new(
                    stringify!($ident), $offset, [$($bytes),*]
                ));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr)
        [$($acc:tt)*]
        $ident:ident($offset:expr) [$($bytes:literal),*];
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr)
            [
                $($acc)*
                $ident.write().load($handle, $game_addr)?;
            ]
            $($tail)*
        );
    };

    // Var
    (
        @define
        [$($acc:tt)*]
        $ident:ident<$ty:ty>($offset:expr);
        $($tail:tt)*
    ) => {
        init!(@define
            [
                $($acc)*
                static $ident: ::parking_lot::RwLock<crate::hack::Var<$ty>> =
                    ::parking_lot::RwLock::new(crate::hack::Var::new(stringify!($ident), $offset));
            ]
            $($tail)*
        );
    };

    (
        @load ($handle:expr, $game_addr:expr)
        [$($acc:tt)*]
        $ident:ident<$ty:ty>($offset:expr);
        $($tail:tt)*
    ) => {
        init!(@load ($handle, $game_addr)
            [
                $($acc)*
                $ident.write().load($game_addr)?;
            ]
            $($tail)*
        );
    };
}
pub(crate) use init;
