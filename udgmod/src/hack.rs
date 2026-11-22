use std::{ffi::c_void, marker::PhantomData, mem, ptr};

use eyre::{Result, bail, eyre};
use minhook::MinHook;
use windows::Win32::{
    Foundation::HANDLE,
    System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
};

use crate::util::GameVersion;

#[derive(Clone, Copy)]
pub struct Offset {
    pub en: usize,
    pub jp: usize,
}

impl Offset {
    pub const fn get(self, version: GameVersion) -> usize {
        match version {
            GameVersion::En => self.en,
            GameVersion::Jp => self.jp,
        }
    }
}

pub struct Var<T> {
    name: &'static str,
    offset: Offset,
    ptr: *mut T,
}

unsafe impl<T> Send for Var<T> {}
unsafe impl<T> Sync for Var<T> {}

impl<T> Var<T> {
    pub const fn new(name: &'static str, offset: Offset) -> Self {
        Self {
            name,
            offset,
            ptr: ptr::null_mut(),
        }
    }

    pub fn load(&mut self, base_addr: usize, version: GameVersion) -> Result<()> {
        if base_addr == 0 {
            bail!("Base address is null");
        }
        let address = base_addr + self.offset.get(version);
        self.ptr = address as *mut T;
        log::info!("{} address: 0x{address:X}", self.name);
        Ok(())
    }

    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        unsafe { self.ptr.as_mut() }
    }
}

pub struct Patch<const N: usize> {
    var: Var<c_void>,
    handle: HANDLE,
    bytes_en: [u8; N],
    bytes_jp: [u8; N],
    bytes: [u8; N],
}

unsafe impl<const N: usize> Sync for Patch<N> {}
unsafe impl<const N: usize> Send for Patch<N> {}

// Didn't end up using this, but it might be useful for future mods so I'm keeping it around for
// now.
#[expect(dead_code)]
impl<const N: usize> Patch<N> {
    pub const fn new(
        name: &'static str,
        offset: Offset,
        bytes_en: [u8; N],
        bytes_jp: [u8; N],
    ) -> Self {
        Self {
            var: Var::new(name, offset),
            handle: HANDLE(ptr::null_mut()),
            bytes_en,
            bytes_jp,
            bytes: bytes_en,
        }
    }

    pub fn load(&mut self, handle: HANDLE, base_addr: usize, version: GameVersion) -> Result<()> {
        self.var.load(base_addr, version)?;
        self.handle = handle;
        self.bytes = match version {
            GameVersion::En => self.bytes_en,
            GameVersion::Jp => self.bytes_jp,
        };
        let mut buffer = [0u8; N];
        unsafe {
            ReadProcessMemory(
                self.handle,
                self.var.ptr,
                buffer.as_mut_ptr().cast::<c_void>(),
                N,
                None,
            )?;
        }

        if buffer != self.bytes {
            bail!(
                "Original bytes do not match for patch {}: expected {:02X?}, found {:02X?}",
                self.var.name,
                self.bytes,
                buffer
            );
        }

        Ok(())
    }

    pub fn apply(&self, patch: [u8; N]) -> Result<()> {
        unsafe {
            self.var
                .ptr
                .as_ref()
                .ok_or_else(|| eyre!("Variable {} is not loaded", self.var.name))?;
            WriteProcessMemory(
                self.handle,
                self.var.ptr,
                patch.as_ptr().cast::<c_void>(),
                N,
                None,
            )?;
        }
        Ok(())
    }

    pub fn restore(&self) -> Result<()> {
        self.apply(self.bytes)
    }
}

pub struct Func<T> {
    var: Var<c_void>,
    _marker: PhantomData<T>,
}

unsafe impl<T> Sync for Func<T> {}
unsafe impl<T> Send for Func<T> {}

impl<T> Func<T> {
    pub const fn new(name: &'static str, offset: Offset) -> Self {
        Self {
            var: Var::new(name, offset),
            _marker: PhantomData,
        }
    }

    pub fn load(&mut self, base_addr: usize, version: GameVersion) -> Result<()> {
        self.var.load(base_addr, version)
    }

    pub fn func(&self) -> Option<T> {
        let r#ref = self.var.as_ref()?;
        Some(unsafe { mem::transmute_copy::<&c_void, T>(&r#ref) })
    }
}

pub struct FuncHook<T> {
    func: Func<T>,
    hook: T,
    trampoline: Option<T>,
}

unsafe impl<T> Sync for FuncHook<T> {}
unsafe impl<T> Send for FuncHook<T> {}

impl<T: Copy> FuncHook<T> {
    pub const fn new(name: &'static str, offset: Offset, hook: T) -> Self {
        Self {
            func: Func::<T>::new(name, offset),
            hook,
            trampoline: None,
        }
    }

    pub fn load(&mut self, base_addr: usize, version: GameVersion) -> Result<()> {
        self.func.load(base_addr, version)?;

        if self.trampoline.is_some() {
            bail!("Function {} is already hooked", self.func.var.name);
        }

        let func = self
            .func
            .func()
            .ok_or_else(|| eyre!("Function {} is not loaded", self.func.var.name))?;
        unsafe {
            let func = mem::transmute_copy::<T, *mut c_void>(&func);
            let trampoline =
                MinHook::create_hook(func, mem::transmute_copy::<T, *mut c_void>(&self.hook))?;
            self.trampoline = Some(mem::transmute_copy::<*mut c_void, T>(&trampoline));
            MinHook::enable_hook(func)?;
        }

        Ok(())
    }

    pub fn trampoline(&self) -> Option<T> {
        self.trampoline
    }
}

macro_rules! patch {
    (try $name:ident($value:expr)) => {
        $name.write().apply($value)
    };
    ($name:ident($value:expr)) => {
        patch!(try $name($value)).unwrap()
    };
    (try $name:ident) => {
        $name.write().restore()
    };
    ($name:ident) => {
        patch!(try $name).unwrap()
    };
}
pub(crate) use patch;

macro_rules! func {
    ($name:ident($($arg:expr),* $(,)?)) => {
        func!(try $name($($arg),*)).unwrap()
    };
    (try $name:ident($($arg:expr),* $(,)?)) => {
        $name.read().func().map(|t| t($($arg),*))
    };
}
pub(crate) use func;

macro_rules! trampoline {
    ($name:ident($($arg:expr),* $(,)?)) => {
        trampoline!(try $name($($arg),*)).unwrap()
    };
    (try $name:ident($($arg:expr),* $(,)?)) => {
        $name.read().trampoline().map(|t| t($($arg),*))
    };
}
pub(crate) use trampoline;
