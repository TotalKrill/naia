use std::collections::HashMap;

use naia_shared::{
    sequence_greater_than, EntityNetId, Manifest, NaiaKey, PacketReader, ProtocolKindType,
    Protocolize, SequenceBuffer,
};

const MESSAGE_BUFFER_MAX_SIZE: u16 = 64;

/// Handles incoming Entity Messages, buffering them to be received on the correct tick
#[derive(Debug)]
pub struct EntityMessageReceiver<P: Protocolize> {
    queued_incoming_messages: SequenceBuffer<HashMap<EntityNetId, P>>,
}

impl<P: Protocolize> EntityMessageReceiver<P> {
    /// Creates a new EntityMessageReceiver
    pub fn new() -> Self {
        EntityMessageReceiver {
            queued_incoming_messages: SequenceBuffer::with_capacity(MESSAGE_BUFFER_MAX_SIZE),
        }
    }

    /// Get the most recently received Entity Message
    pub fn pop_incoming_entity_message(&mut self, server_tick: u16) -> Option<(EntityNetId, P)> {
        if let Some(map) = self.queued_incoming_messages.get_mut(server_tick) {
            let mut any_entity: Option<EntityNetId> = None;
            if let Some(any_entity_ref) = map.keys().next() {
                any_entity = Some(*any_entity_ref);
            }
            if let Some(any_entity) = any_entity {
                if let Some(message) = map.remove(&any_entity) {
                    return Some((any_entity, message));
                }
            }
        }
        return None;
    }

    /// Given incoming packet data, read transmitted Entity Message and store them to
    /// be returned to the application
    pub fn process_incoming_messages(
        &mut self,
        server_tick_opt: Option<u16>,
        client_tick: u16,
        reader: &mut PacketReader,
        manifest: &Manifest<P>,
    ) {
        let message_count = reader.read_u8();
        for _x in 0..message_count {
            let owned_entity = EntityNetId::from_u16(reader.read_u16());
            let replica_kind: P::Kind = P::Kind::from_u16(reader.read_u16());

            let new_message = manifest.create_replica(replica_kind, reader, 0);

            if let Some(server_tick) = server_tick_opt {
                if sequence_greater_than(client_tick, server_tick) {
                    if !self.queued_incoming_messages.exists(client_tick) {
                        self.queued_incoming_messages
                            .insert(client_tick, HashMap::new());
                    }
                    if let Some(map) = self.queued_incoming_messages.get_mut(client_tick) {
                        if !map.contains_key(&owned_entity) {
                            map.insert(owned_entity, new_message);
                        }
                    }
                }
            }
        }
    }
}