use windows::core::HRESULT;

use crate::{
    hack::trampoline,
    util::{asmut, init},
};

init!([
    SHADOW_TEXTURE_SIZE<u32>(0x0030_a5d8);
    SHADOW_TEXTURE_SETUP<fn() -> HRESULT>(0x0011_0a20) => shadow_texture_setup;
    SHADOW_SETUP<fn(f32, f32, f32, f32)>(0x0012_6a60) => shadow_setup;
]);

const FACTOR: u32 = 2_u32.pow(2);

extern "C" fn shadow_texture_setup() -> HRESULT {
    if let Some(size) = asmut!(SHADOW_TEXTURE_SIZE) {
        *size *= FACTOR;
        log::info!("Shadow texture size set to {}", *size);
    }

    trampoline!(SHADOW_TEXTURE_SETUP())
}

extern "C" fn shadow_setup(mut extents: f32, arg2: f32, arg3: f32, arg4: f32) {
    const FACTOR: u32 = 2_u32.pow(2);

    extents *= FACTOR as f32;

    trampoline!(SHADOW_SETUP(extents, arg2, arg3, arg4));

    log::info!("Shadow extents set to {extents}");
}
