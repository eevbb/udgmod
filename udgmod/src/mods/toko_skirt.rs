use parking_lot::RwLock;
use windows::Win32::UI::Input::KeyboardAndMouse::VK_F5;

use crate::{
    hack::{func, trampoline},
    model::{ActorList, JointData, PchrData, PchrKind, RenderObj},
    util::{asref, init, is_key_pressed},
};

init!(
    [
        ACTOR_LIST<*mut ActorList>(0x007e_cb10);
        JOINT_LIST_COUNT<i16>(0x0080_2b78);
        JOINT_LIST<*mut JointData>(0x0080_2b80);
        JOINT_IDX_TABLE<*mut i16>(0x0080_2b88);
        GET_PCHR<fn(PchrKind, bool) -> *mut PchrData>(0x000b_49c0);
        UPDATE_SKELETON<fn(*mut RenderObj, *mut *mut JointData)>(0x0010_7350) => update_skeleton;
    ]
    update: update;
);

fn update() {
    if is_key_pressed(VK_F5) {
        let mut enabled = ENABLED.write();
        *enabled = !*enabled;
    }
}

extern "C" fn update_skeleton(render_obj: *mut RenderObj, joint_data: *mut *mut JointData) {
    trampoline!(UPDATE_SKELETON(render_obj, joint_data));

    if *ENABLED.read()
        && let Some(render_obj) = unsafe { render_obj.as_ref() }
        && let Some(toko_pchr) = unsafe { func!(GET_PCHR(PchrKind::TOKO_BODY, false)).as_ref() }
        && render_obj.mesh_data == toko_pchr.mesh_data
        && let Some(&joint_list_count) = asref!(JOINT_LIST_COUNT)
        && let Some(&joint_list) = asref!(JOINT_LIST)
        && let Some(&joint_idx_table) = asref!(JOINT_IDX_TABLE)
    {
        let mut target = None;
        for (n, joint) in render_obj
            .joints(joint_list_count, joint_list, joint_idx_table)
            .enumerate()
            .filter_map(|(n, joint)| unsafe { joint.as_mut().map(|joint| (n, joint)) })
        {
            // This is a joint inside Toko's torso.
            // By moving all skirt joints into this joint, we effectively remove the skirt!
            if n == 3 {
                target = Some(joint.matrix);
            }

            // Toko's skirt joints are conveniently all in a row!
            if (101..=160).contains(&n)
                && let Some(mat) = target
            {
                joint.matrix = mat;
            }
        }
    }
}

static ENABLED: RwLock<bool> = RwLock::new(false);
