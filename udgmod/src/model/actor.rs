use proc::mapped;

use crate::model::{PchrKind, RenderObj};

#[mapped(0x588)]
pub struct Actor {
    pub pchr_kind: PchrKind,

    #[offset(0x10)]
    pub position: *mut Position,
    pub render_obj: *mut RenderObj,

    #[offset(0x580)]
    pub data: *mut Data,
}

#[mapped(0x640)]
pub struct Position {}

#[mapped(0x200)]
pub struct Data {
    #[offset(0x64)]
    pub flags: u32,

    #[offset(0x70)]
    pub field_70: u16,
}
