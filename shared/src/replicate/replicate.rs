use std::{
    any::TypeId,
    fmt::{Debug, Formatter, Result},
};

use super::{shared_replicate_mutator::SharedReplicateMutator, protocol_type::ProtocolType, diff_mask::DiffMask};

use crate::{PacketReader, Ref};

/// An Replicate is a container of Properties that can be scoped, tracked, and
/// synced, with a remote host
pub trait Replicate<T: ProtocolType>: EventClone<T> {
    /// Gets the number of bytes of the Replicate's Replicate Mask
    fn get_diff_mask_size(&self) -> u8;
    /// Gets a copy of the Replicate, wrapped in an ProtocolType enum (which is the
    /// common protocol between the server/host)
    fn get_typed_copy(&self) -> T;
    /// Gets the TypeId of the Replicate's implementation, used to map to a
    /// registered ProtocolType
    fn get_type_id(&self) -> TypeId;
    /// Writes data into an outgoing byte stream, sufficient to completely
    /// recreate the Replicate on the client
    fn write(&self, out_bytes: &mut Vec<u8>);
    /// Write data into an outgoing byte stream, sufficient only to update the
    /// mutated Properties of the Replicate on the client
    fn write_partial(&self, diff_mask: &DiffMask, out_bytes: &mut Vec<u8>);
    /// Reads data from an incoming packet, sufficient to sync the in-memory
    /// Replicate with it's replicate on the Server
    fn read_full(&mut self, reader: &mut PacketReader, packet_index: u16);
    /// Reads data from an incoming packet, sufficient to sync the in-memory
    /// Replicate with it's replicate on the Server
    fn read_partial(
        &mut self,
        diff_mask: &DiffMask,
        reader: &mut PacketReader,
        packet_index: u16,
    );
    /// Set the Replicate's ReplicateMutator, which keeps track of which Properties
    /// have been mutated, necessary to sync only the Properties that have
    /// changed with the client
    fn set_mutator(&mut self, mutator: &Ref<dyn SharedReplicateMutator>);
}

//TODO: do we really need another trait here?
/// Handles equality of Replicates.. can't just derive PartialEq because we want
/// to only compare Properties
pub trait ReplicateEq<T: ProtocolType, Impl = Self>: Replicate<T> {
    /// Compare properties in another Replicate
    fn equals(&self, other: &Impl) -> bool;
    /// Sets the current Replicate to the replicate of another Replicate of the same type
    fn mirror(&mut self, other: &Impl);
}

impl<T: ProtocolType> Debug for dyn Replicate<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str("Replicate")
    }
}

/// A Boxed Event must be able to clone itself
pub trait EventClone<T: ProtocolType> {
    /// Clone the Boxed Event
    fn clone_box(&self) -> Box<dyn Replicate<T>>;
}

impl<Z: ProtocolType, T: 'static + Replicate<Z> + Clone> EventClone<Z> for T {
    fn clone_box(&self) -> Box<dyn Replicate<Z>> {
        Box::new(self.clone())
    }
}

impl<T: ProtocolType> Clone for Box<dyn Replicate<T>> {
    fn clone(&self) -> Box<dyn Replicate<T>> {
        EventClone::clone_box(self.as_ref())
    }
}

//impl<T: ProtocolType> Debug for Box<dyn Replicate<T>> {
//    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
//        f.write_str("Boxed Event")
//    }
//}
