use std::{
    ffi::c_void,
    time::{Duration, Instant},
};

use parking_lot::RwLock;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_F2, VK_F3, VK_F4, VK_SHIFT};

use crate::{
    hack::{func, trampoline},
    model::{Actor, PchrKind},
    util::{asmut, asref, init, is_key_held, is_key_pressed},
};

init!(
    [
        OUTFIT_TO_LOAD<i16>(0x0031_7d90);
        MAIN_ACTOR<*mut Actor>(0x007e_bc08);
        OUTFIT_HEALTH<i16>(0x007b_93c0);
        LOAD_OUTFIT<fn(u8)>(0x0005_77d0);
        PRELOAD_PCHR_KIND<fn(PchrKind) -> *const c_void>(0x000b_5310);
        PRELOAD<fn(i16, u8, u8) -> *const c_void>(0x0002_3410) => preload;
        INIT_MAIN_ACTOR<fn(*mut Actor)>(0x0006_00d0) => init_main_actor;
        CLEAR_DATA<fn(u32) -> u64>(0x0009_8680) => clear_data;
        LOAD_PCHR<fn(*const c_void, PchrKind) -> *const c_void>(0x000b_9ff0) => load_pchr;
    ]
    update: update;
);

fn update() {
    if is_key_held(VK_SHIFT) {
        if is_key_pressed(VK_F2) {
            set_outfit_override(None);
        }
    } else {
        if is_key_pressed(VK_F2) {
            set_outfit_override(Some(Outfit::Default));
        }

        if is_key_pressed(VK_F3) {
            set_outfit_override(Some(Outfit::NoSkirt));
        }

        if is_key_pressed(VK_F4) {
            set_outfit_override(Some(Outfit::NoSkirtNoShirt));
        }
    }

    if Some(get_intended_outfit()) != *LAST_SET_OUTFIT.read() {
        reapply_outfit();
    }
}

extern "C" fn preload(kind: i16, arg2: u8, arg3: u8) -> *const c_void {
    func!(PRELOAD_PCHR_KIND(Outfit::NoSkirt.get_kind()));
    func!(PRELOAD_PCHR_KIND(Outfit::NoSkirtNoShirt.get_kind()));
    trampoline!(PRELOAD(kind, arg2, arg3))
}

// These functions reset the outfit health back to 0 when they run.
// To prevent that, save the current outfit health so it can be restored afterwards.
extern "C" fn init_main_actor(actor: *mut Actor) {
    let previous_health = asref!(OUTFIT_HEALTH).copied();
    trampoline!(INIT_MAIN_ACTOR(actor));
    restore_previous_health(previous_health);
}

extern "C" fn clear_data(arg1: u32) -> u64 {
    let previous_health = asref!(OUTFIT_HEALTH).copied();
    let ret = trampoline!(CLEAR_DATA(arg1));
    restore_previous_health(previous_health);
    ret
}

extern "C" fn load_pchr(arg1: *const c_void, mut kind: PchrKind) -> *const c_void {
    if kind.is_komaru_body() {
        // If the kind being set is one of Komaru's outfits, replace it with
        // the intended outfit instead.
        let outfit = get_intended_outfit();
        kind = outfit.get_kind();
        LAST_SET_OUTFIT.write().replace(outfit);
    }

    // Don't reset the timer when Komaru-related objects load.
    if !kind.is_komaru_object() {
        *LOAD_TIME.write() = Some(Instant::now());
    }

    trampoline!(LOAD_PCHR(arg1, kind))
}

// This is ugly but it prevents a crash that happens if you try
// to change outfits too soon after spawning.
// It's probably because of a race condition that happens because the game is still
// finishing loading the stage after spawning.
static LOAD_TIME: RwLock<Option<Instant>> = RwLock::new(None);
fn is_load_time_safe() -> bool {
    const LOAD_TIME_WAIT: Duration = Duration::from_secs(3);
    LOAD_TIME
        .read()
        .is_some_and(|t| t.elapsed() > LOAD_TIME_WAIT)
}

static OUTFIT_OVERRIDE: RwLock<Option<Outfit>> = RwLock::new(Some(Outfit::Default));
static LAST_SET_OUTFIT: RwLock<Option<Outfit>> = RwLock::new(None);

fn get_intended_outfit() -> Outfit {
    if let Some(outfit_override) = *OUTFIT_OVERRIDE.read() {
        outfit_override
    } else if let Some(&health) = asref!(OUTFIT_HEALTH) {
        if health == 0 {
            Outfit::NoSkirtNoShirt
        } else if health <= 3 {
            Outfit::NoSkirt
        } else {
            Outfit::Default
        }
    } else {
        Outfit::Default
    }
}

fn set_outfit_override(value: Option<Outfit>) {
    if value.is_none()
        && let Some(outfit_health) = asmut!(OUTFIT_HEALTH)
    {
        // None is clothing destruction mode, so restore outfit health to the
        // default value.
        *outfit_health = 6;
    }

    *OUTFIT_OVERRIDE.write() = value;
    reapply_outfit();
}

fn reapply_outfit() {
    if is_load_time_safe()
        && let Some(actor) = asref!(MAIN_ACTOR)
        && let Some(actor) = unsafe { actor.as_mut() }
        && actor.pchr_kind.is_komaru_body()
        && let Some(data) = unsafe { actor.data.as_mut() }
        // Not sure why but this avoids a crash on game load
        && data.field_70 == 0
    {
        // It actually doesn't matter what we set here as long as it's not 0
        // as it'll get corrected in the LoadPchr hook.
        if let Some(outfit_to_load) = asmut!(OUTFIT_TO_LOAD) {
            *outfit_to_load = 1;
        }

        // The game won't load the outfit if this flag isn't set.
        data.flags |= 2;
        func!(LOAD_OUTFIT(1));

        if let Some(outfit_to_load) = asmut!(OUTFIT_TO_LOAD) {
            *outfit_to_load = 0;
        }
    }
}

fn restore_previous_health(previous_health: Option<i16>) {
    // If the outfit override is None (clothing destruction mode) and the outfit health was reset to
    // 0, restore it to the previous value.
    if OUTFIT_OVERRIDE.read().is_none()
        && let Some(previous_health) = previous_health
        && let Some(health) = asmut!(OUTFIT_HEALTH)
        && *health == 0
    {
        *health = previous_health;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
enum Outfit {
    Default = 1,
    NoSkirt = 2,
    NoSkirtNoShirt = 3,
}

impl Outfit {
    const fn get_kind(self) -> PchrKind {
        match self {
            Outfit::Default => PchrKind::KOMARU_BODY,
            Outfit::NoSkirt => PchrKind::KOMARU_BODY_D1,
            Outfit::NoSkirtNoShirt => PchrKind::KOMARU_BODY_D2,
        }
    }
}
