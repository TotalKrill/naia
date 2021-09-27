use std::any::TypeId;

use naia_shared::ProtocolType;

use super::world_type::WorldType;

pub trait KeyType {}

#[derive(Debug)]
pub struct ComponentKey<P: ProtocolType, W: WorldType<P>>(pub W::EntityKey, pub TypeId);

impl<P: ProtocolType, W: WorldType<P>> ComponentKey<P, W> {
    pub fn new(key: &W::EntityKey, type_id: &TypeId) -> Self {
        ComponentKey(key, type_id)
    }
}