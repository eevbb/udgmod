use glam::{Vec2, Vec3};
use proc::mapped;

#[mapped(0x50)]
#[derive(Clone)]
pub struct CamData {
    pub near_plane: f32,
    pub far_plane: f32,
    pub cull_radius: f32,
    pub zoom: f32,
    pub roll: f32,
    pub pos: Vec3,
    pub focus: Vec3,
    pub rot: Vec2,
    pub dist_3d: f32,
    pub dist_xy: f32,
    pub delta_z: f32,
    pub field_of_view: f32,
    pub fwd: Vec3,
}
