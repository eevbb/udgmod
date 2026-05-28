use glam::Mat4;
use proc::mapped;

#[mapped(0x108)]
pub struct RenderObj {
    #[offset(0x2)]
    pub render_flags: u16,

    #[offset(0x28)]
    pub mesh_data: *mut MeshData,

    #[offset(0xD4)]
    pub root_joint_idx: i16,
}

impl RenderObj {
    pub fn joints(
        &self,
        joint_list_count: i16,
        joint_list: *mut JointData,
        joint_idx_table: *mut i16,
    ) -> JointIter {
        JointIter {
            joint_list_count,
            joint_list,
            joint_idx_table,
            idx: self.root_joint_idx,
        }
    }
}

#[mapped]
pub struct MeshData {
    #[offset(0x4)]
    pub joint_count: u8,
}

#[mapped]
pub struct JointData {
    #[offset(0x90)]
    pub matrix: Mat4,

    #[offset(0xd0)]
    _end: (),
}

pub struct JointIter {
    joint_list: *mut JointData,
    joint_idx_table: *mut i16,
    joint_list_count: i16,
    idx: i16,
}

impl Iterator for JointIter {
    type Item = *mut JointData;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx > 0 && self.idx < self.joint_list_count {
            let next = unsafe { self.joint_list.add(self.idx.cast_unsigned() as usize) };
            self.idx = unsafe {
                self.joint_idx_table
                    .add(self.idx.cast_unsigned() as usize)
                    .as_ref()
                    .copied()
                    .unwrap_or(-1)
            };

            Some(next)
        } else {
            None
        }
    }
}
