use std::{
    any::TypeId,
    fmt::{Debug, Formatter, Result},
};

use naia_socket_shared::PacketReader;

use super::protocol_type::ProtocolType;

/// Handles the creation of new Replica (Message/Component) instances
pub trait ReplicaBuilder<P: ProtocolType>: Send + Sync {
    /// Create a new Replica instance
    fn build(&self, reader: &mut PacketReader) -> P;
    /// Gets the TypeId of the Replica the builder is able to build
    fn get_type_id(&self) -> TypeId;
}

impl<P: ProtocolType> Debug for Box<dyn ReplicaBuilder<P>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("Boxed ReplicaBuilder")
    }
}