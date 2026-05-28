mod hack;
mod logging;
mod model;
mod mods;
mod util;

use std::{ffi::c_void, mem, sync::LazyLock};

use eyre::{OptionExt, Result, bail, eyre};
use minhook::MinHook;
use parking_lot::RwLock;
use windows::{
    Win32::{
        Foundation::{E_FAIL, HINSTANCE, HMODULE},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE, D3D_FEATURE_LEVEL},
            Direct3D11::{D3D11_CREATE_DEVICE_FLAG, ID3D11Device, ID3D11DeviceContext},
            Dxgi::{
                Common::{
                    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
                    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
                },
                DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
                IDXGIDevice, IDXGIFactory, IDXGISwapChain,
            },
        },
        System::{
            LibraryLoader::{GetProcAddress, LoadLibraryA},
            SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
        },
        UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE, WS_OVERLAPPEDWINDOW,
        },
    },
    core::{BOOL, HRESULT, Interface as _, PCSTR, s, w},
};

use crate::util::asref;

type PresentFn = unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;

static DEVICE: RwLock<Option<ID3D11Device>> = RwLock::new(None);
static ORIGINAL_PRESENT: RwLock<Option<PresentFn>> = RwLock::new(None);
static MODS: RwLock<Vec<fn()>> = RwLock::new(Vec::new());

static ORIGINAL_DLL: LazyLock<Module> = LazyLock::new(|| {
    let system_path = std::env::var("SYSTEMROOT").unwrap_or_else(|_| "C:\\Windows".to_string());
    let dll_path = format!("{system_path}\\System32\\d3d11.dll\0");
    let original_dll =
        unsafe { LoadLibraryA(PCSTR(dll_path.as_ptr())) }.expect("original d3d11.dll");
    Module(original_dll)
});

struct Module(HMODULE);
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

// Export D3D11CreateDevice
#[unsafe(no_mangle)]
pub unsafe extern "system" fn D3D11CreateDevice(
    p_adapter: *mut c_void,
    driver_type: D3D_DRIVER_TYPE,
    software: isize,
    flags: D3D11_CREATE_DEVICE_FLAG,
    p_feature_levels: *const D3D_FEATURE_LEVEL,
    feature_levels: u32,
    sdk_version: u32,
    pp_device: *mut Option<ID3D11Device>,
    p_feature_level: *mut D3D_FEATURE_LEVEL,
    pp_immediate_context: *mut Option<ID3D11DeviceContext>,
) -> HRESULT {
    type D3D11CreateDeviceFn = unsafe extern "system" fn(
        *mut c_void,
        D3D_DRIVER_TYPE,
        isize,
        D3D11_CREATE_DEVICE_FLAG,
        *const D3D_FEATURE_LEVEL,
        u32,
        u32,
        *mut Option<ID3D11Device>,
        *mut D3D_FEATURE_LEVEL,
        *mut Option<ID3D11DeviceContext>,
    ) -> HRESULT;

    static ORIGINAL_FUNC: LazyLock<D3D11CreateDeviceFn> = LazyLock::new(|| {
        let proc = unsafe {
            GetProcAddress(ORIGINAL_DLL.0, s!("D3D11CreateDevice"))
                .expect("original D3D11CreateDevice")
        };
        unsafe { mem::transmute(proc) }
    });

    log::info!("D3D11CreateDevice called");

    let result = unsafe {
        ORIGINAL_FUNC(
            p_adapter,
            driver_type,
            software,
            flags,
            p_feature_levels,
            feature_levels,
            sdk_version,
            pp_device,
            p_feature_level,
            pp_immediate_context,
        )
    };

    // Save the device
    if result.is_ok()
        && !pp_device.is_null()
        && let Some(Some(device)) = unsafe { pp_device.as_ref() }
    {
        log::info!("D3D11CreateDevice succeeded");

        if let Err(e) = dummy_swapchain_hook_present(device) {
            log::error!("Failed to hook Present: {e:?}");
        }

        DEVICE.write().replace(device.clone());
    }

    result
}

unsafe extern "system" fn present_hook(
    swap_chain: *mut c_void,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    for update_fn in MODS.read().iter() {
        update_fn();
    }

    // Call original Present
    if let Some(original) = asref!(ORIGINAL_PRESENT) {
        unsafe { (*original)(swap_chain, sync_interval, flags) }
    } else {
        E_FAIL
    }
}

fn dummy_swapchain_hook_present(device: &ID3D11Device) -> Result<()> {
    let mut original = ORIGINAL_PRESENT.write();
    if original.is_some() {
        log::info!("Present already hooked, skipping");
        return Ok(());
    }

    log::info!("Creating dummy swap chain to find Present...");

    // Get DXGI device
    let dxgi_device = device
        .cast::<IDXGIDevice>()
        .map_err(|e| eyre!("Failed to get DXGI device: {e:?}"))?;

    // Get adapter
    let adapter =
        unsafe { dxgi_device.GetAdapter() }.map_err(|e| eyre!("Failed to get adapter: {e:?}"))?;

    // Get factory
    let factory = unsafe { adapter.GetParent::<IDXGIFactory>() }
        .map_err(|e| eyre!("Failed to get factory: {e:?}"))?;

    // Create a message-only dummy window for the swap chain
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("Static"), // Use built-in Static class
            w!("DummyWindow"),
            WS_OVERLAPPEDWINDOW,
            0,
            0,
            100,
            100,
            Some(HWND_MESSAGE), // Message-only window
            None,
            None,
            None,
        )
    }
    .map_err(|e| eyre!("Failed to create dummy window: {e:?}"))?;

    log::info!("Created dummy window: {hwnd:?}");

    (|| {
        // Setup swap chain description
        let desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC {
                Width: 100,
                Height: 100,
                RefreshRate: DXGI_RATIONAL {
                    Numerator: 60,
                    Denominator: 1,
                },
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
            },
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 1,
            OutputWindow: hwnd,
            Windowed: true.into(),
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        };

        // Create the swap chain
        let mut swap_chain = None;
        let result =
            unsafe { factory.CreateSwapChain(device, &raw const desc, &raw mut swap_chain) };
        if result.is_err() {
            bail!("Failed to create dummy swap chain: {result:?}");
        }

        let swap_chain = swap_chain.ok_or_eyre("CreateSwapChain succeeded but returned None")?;

        unsafe { hook_present(&swap_chain, &mut original) }?;

        Ok(())
    })()?;

    // Clean up dummy window
    unsafe { DestroyWindow(hwnd) }
        .inspect_err(|e| log::warn!("Error destroying dummy window: {e:?}"))
        .ok();

    Ok(())
}

unsafe fn hook_present(
    swap_chain: &IDXGISwapChain,
    original: &mut Option<PresentFn>,
) -> Result<()> {
    // IDXGISwapChain is a wrapper around a COM pointer
    // Get the raw COM interface pointer
    let com_ptr: *const *const usize = unsafe { mem::transmute_copy(swap_chain) };
    log::info!("COM interface pointer: {com_ptr:?}");

    let Some(&vtable) = (unsafe { com_ptr.as_ref() }) else {
        bail!("COM pointer is invalid");
    };

    log::info!("VTable address: {vtable:?}");
    if vtable.is_null() {
        bail!("VTable is null!");
    }

    let present_ptr = unsafe { *vtable.add(8) };
    log::info!("Present function: 0x{present_ptr:X}");

    if present_ptr == 0 {
        bail!("Present pointer is null!");
    }

    let addr = unsafe { MinHook::create_hook(present_ptr as _, present_hook as _) }
        .map_err(|e| eyre!("Create hook failed: {e:?}"))?;

    unsafe { MinHook::enable_hook(present_ptr as _) }
        .map_err(|e| eyre!("Enable all hooks failed: {e:?}"))?;
    *original = Some(unsafe { mem::transmute::<*const c_void, PresentFn>(addr) });

    log::info!("Present hooked successfully!");
    Ok(())
}

// DLL entry point
#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // Initialize debug console
            #[cfg(debug_assertions)]
            {
                use windows::Win32::System::Console::AllocConsole;
                unsafe { AllocConsole() }.ok();
            }

            logging::init(
                "udgmod.log",
                log::LevelFilter::Trace,
                log::LevelFilter::Info,
            )
            .ok();

            let mut mods = MODS.write();
            mods::init(|update_fn| mods.push(update_fn));
        }
        DLL_PROCESS_DETACH => {
            log::info!("DLL unloading");
        }
        _ => {}
    }
    true.into()
}
