use naia_socket_shared::PacketReader;

use super::protocol_type::ProtocolType;

/// Handles the creation of new Replica (Message/Component) instances
pub trait ReplicaBuilder<P: ProtocolType>: Send + Sync + std::fmt::Debug {
    /// Create a new Replica instance
    fn build(&self, reader: &mut PacketReader, packet_index: u16) -> P;
    /// Gets the ProtocolKind of the Replica the builder is able to build
    fn get_kind(&self) -> P::Kind;
}
