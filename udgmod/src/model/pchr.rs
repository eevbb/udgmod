use proc::mapped;

use crate::model::MeshData;

#[mapped]
pub struct PchrData {
    #[offset(0xA0)]
    pub mesh_data: *mut MeshData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PchrKind(pub u32);

impl PchrKind {
    pub const KOMARU_BODY: Self = Self(1);
    pub const KOMARU_FACE: Self = Self(2);
    pub const KOMARU_HAIR: Self = Self(3);
    pub const KOMARU_WEAPON: Self = Self(4);
    pub const KOMARU_BODY_SHADOW: Self = Self(5);
    pub const KOMARU_HAIR_SHADOW: Self = Self(6);
    pub const KOMARU_FACE_SHADOW: Self = Self(7);
    pub const KOMARU_BANGLE: Self = Self(8);
    pub const KOMARU_BODY_D1: Self = Self(11);
    pub const KOMARU_BODY_D1_SHADOW: Self = Self(12);
    pub const KOMARU_BODY_D2: Self = Self(13);
    pub const KOMARU_BODY_D2_SHADOW: Self = Self(14);
    pub const TOKO_BODY: Self = Self(16);
    pub const TOKO_HEAD: Self = Self(17);
    pub const TOKO_HAIR: Self = Self(18);

    pub const fn is_komaru_object(self) -> bool {
        matches!(
            self,
            Self::KOMARU_BODY
                | Self::KOMARU_FACE
                | Self::KOMARU_HAIR
                | Self::KOMARU_WEAPON
                | Self::KOMARU_BODY_SHADOW
                | Self::KOMARU_HAIR_SHADOW
                | Self::KOMARU_FACE_SHADOW
                | Self::KOMARU_BANGLE
                | Self::KOMARU_BODY_D1
                | Self::KOMARU_BODY_D1_SHADOW
                | Self::KOMARU_BODY_D2
                | Self::KOMARU_BODY_D2_SHADOW
        )
    }

    pub const fn is_komaru_body(self) -> bool {
        matches!(
            self,
            Self::KOMARU_BODY | Self::KOMARU_BODY_D1 | Self::KOMARU_BODY_D2
        )
    }
}
