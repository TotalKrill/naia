use naia_shared::EntityKey;

#[allow(missing_docs)]
#[allow(unused_doc_comments)]
pub mod replicate_key {
    // The Global Key used to get a reference of an Replicate
    new_key_type! { pub struct ReplicaKey; }
}

/// Key to be used to reference an Object Replicate
pub type ObjectKey = replicate_key::ReplicaKey;

/// Key to be used to reference a Component Replicate
pub type ComponentKey = replicate_key::ReplicaKey;

/// GlobalPawnKey
pub enum GlobalPawnKey {
    /// Object
    Object(ObjectKey),
    /// Entity
    Entity(EntityKey),
}
