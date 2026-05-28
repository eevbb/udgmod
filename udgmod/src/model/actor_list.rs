use proc::mapped;

use crate::model::Actor;

#[repr(C, packed)]
#[mapped]
pub struct ActorList {
    #[offset(0x18)]
    llist: *mut ActorListNode,
}

#[repr(C)]
pub struct ActorListNode {
    prev: *mut ActorListNode,
    next: *mut ActorListNode,
    actor: *mut Actor,
}

impl IntoIterator for &ActorList {
    type Item = *mut Actor;
    type IntoIter = ActorListIter;

    fn into_iter(self) -> Self::IntoIter {
        ActorListIter {
            last: unsafe { self.llist.as_mut() }
                .map(|node| node.prev)
                .unwrap_or_default(),
            current: self.llist,
        }
    }
}

pub struct ActorListIter {
    last: *mut ActorListNode,
    current: *mut ActorListNode,
}

impl Iterator for ActorListIter {
    type Item = *mut Actor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last.is_null() || self.current == self.last {
            None
        } else {
            let current = unsafe { self.current.as_mut() }?;
            self.current = current.next;
            Some(current.actor)
        }
    }
}
