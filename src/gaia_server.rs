
use std::{
    net::SocketAddr,
    collections::{VecDeque, HashMap},
    rc::Rc,
    cell::RefCell,
};

use log::{info};

use gaia_server_socket::{ServerSocket, SocketEvent, MessageSender, Config as SocketConfig, GaiaServerSocketError};
pub use gaia_shared::{Config, PacketType, NetConnection, Timer, Timestamp, EventManifest, EntityManifest, EntityStore, EntityKey, NetEvent, NetEntity, ManagerType, HostType, EventType, EntityType};

use super::server_event::ServerEvent;
use crate::{
    Packet,
    error::GaiaServerError};

pub struct GaiaServer<T: EventType, U: EntityType> {
    event_manifest: EventManifest<T>,
    entity_manifest: EntityManifest<U>,
    global_entity_store: EntityStore<U>,
    scope_map: HashMap<EntityKey, Rc<Box<dyn Fn(&SocketAddr) -> bool>>>,
    config: Config,
    socket: ServerSocket,
    sender: MessageSender,
    client_connections: HashMap<SocketAddr, NetConnection<T, U>>,
    outstanding_disconnects: VecDeque<SocketAddr>,
    heartbeat_timer: Timer,
    drop_counter: u8,
    drop_max: u8,
}

impl<T: EventType, U: EntityType> GaiaServer<T, U> {
    pub async fn listen(address: &str, event_manifest: EventManifest<T>, entity_manifest: EntityManifest<U>, config: Option<Config>) -> Self {

        let mut config = match config {
            Some(config) => config,
            None => Config::default()
        };
        config.heartbeat_interval /= 2;

        let mut socket_config = SocketConfig::default();
        socket_config.connectionless = true;
        socket_config.tick_interval = config.tick_interval;
        let mut server_socket = ServerSocket::listen(address, Some(socket_config)).await;

        let sender = server_socket.get_sender();
        let clients_map = HashMap::new();
        let heartbeat_timer = Timer::new(config.heartbeat_interval);

        GaiaServer {
            event_manifest,
            entity_manifest,
            global_entity_store: EntityStore::new(),
            scope_map: HashMap::new(),
            socket: server_socket,
            sender,
            config,
            client_connections: clients_map,
            outstanding_disconnects: VecDeque::new(),
            heartbeat_timer,
            drop_counter: 1,
            drop_max: 3,
        }
    }

    pub async fn receive(&mut self) -> Result<ServerEvent<T>, GaiaServerError> {
        let mut output: Option<Result<ServerEvent<T>, GaiaServerError>> = None;
        while output.is_none() {

            // heartbeats
            if self.heartbeat_timer.ringing() {
                self.heartbeat_timer.reset();

                for (address, connection) in self.client_connections.iter_mut() {
                    if connection.should_drop() {
                        self.outstanding_disconnects.push_back(*address);
                    } else if connection.should_send_heartbeat() {
                        // Don't try to refactor this to self.internal_send, doesn't seem to work cause of iter_mut()
                        let payload = connection.process_outgoing(PacketType::Heartbeat, &[]);
                        self.sender.send(Packet::new_raw(*address, payload))
                            .await
                            .expect("send failed!");
                        connection.mark_sent();
                    }
                }
            }

            // timeouts
            if let Some(addr) = self.outstanding_disconnects.pop_front() {
                self.client_connections.remove(&addr);
                output = Some(Ok(ServerEvent::Disconnection(addr)));
                continue;
            }


            for (address, connection) in self.client_connections.iter_mut() {
                //receive events from anyone
                if let Some(something) = connection.get_incoming_event() {
                    output = Some(Ok(ServerEvent::Event(*address, something)));
                    continue;
                }
            }

            //receive socket events
            match self.socket.receive().await {
                Ok(event) => {
                    match event {
                        SocketEvent::Packet(packet) => {
                            let address = packet.address();
                            match self.client_connections.get_mut(&address) {
                                Some(connection) => {
                                    connection.mark_heard();
                                }
                                None => {} //not yet established connection
                            }

                            let packet_type = PacketType::get_from_packet(packet.payload());
                            if packet_type == PacketType::Data {
                                //simulate dropping
                                if self.drop_counter >= self.drop_max {
                                    self.drop_counter = 0;
                                    info!("~~~~~~~~~~  dropped packet from client  ~~~~~~~~~~");
                                    continue;
                                } else {
                                    self.drop_counter += 1;
                                }
                            }

                            match packet_type {
                                PacketType::ClientHandshake => {
                                    let payload = gaia_shared::utils::read_headerless_payload(packet.payload());
                                    let timestamp = Timestamp::read(&payload);

                                    if !self.client_connections.contains_key(&address) {
                                        self.client_connections.insert(address,
                                                                       NetConnection::new(HostType::Server,
                                                                                          self.config.heartbeat_interval,
                                                                                          self.config.disconnection_timeout_duration,
                                                                                          timestamp));
                                        output = Some(Ok(ServerEvent::Connection(address)));
                                    }

                                    match self.client_connections.get_mut(&address) {
                                        Some(connection) => {
                                            if timestamp == connection.connection_timestamp {
                                                self.send_internal(PacketType::ServerHandshake, Packet::new_raw(address, Box::new([])))
                                                    .await;
                                                continue;
                                            } else {
                                                // Incoming Timestamp is different than recorded.. must be the same client trying to connect..
                                                // so disconnect them to provide continuity
                                                self.client_connections.remove(&address);
                                                output = Some(Ok(ServerEvent::Disconnection(address)));
                                                continue;
                                            }
                                        }
                                        None => {}
                                    }
                                }
                                PacketType::Data => {

                                    match self.client_connections.get_mut(&address) {
                                        Some(connection) => {
                                            let mut payload = connection.process_incoming(packet.payload());
                                            connection.process_data(&self.event_manifest, &self.entity_manifest, &mut payload);
                                            continue;
                                        }
                                        None => {
                                            warn!("received data from unauthenticated client: {}", address);
                                        }
                                    }
                                }
                                PacketType::Heartbeat => {
                                    match self.client_connections.get_mut(&address) {
                                        Some(connection) => {
                                            // Still need to do this so that proper notify events fire based on the heartbeat header
                                            connection.process_incoming(packet.payload());
                                            info!("<- c");
                                            continue;
                                        }
                                        None => {
                                            warn!("received heartbeat from unauthenticated client: {}", address);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        SocketEvent::Tick => {

                            // update entity scopes
                            self.update_entity_scopes();

                            // loop through all connections, send packet
                            for (address, connection) in self.client_connections.iter_mut() {
                                if let Some(payload) = connection.get_outgoing_packet(&self.event_manifest) {
                                    match self.sender.send(Packet::new_raw(*address, payload))
                                        .await {
                                        Ok(_) => {}
                                        Err(err) => {
                                            info!("send error! {}", err);
                                        }
                                    }
                                    connection.mark_sent();
                                }
                            }

                            output = Some(Ok(ServerEvent::Tick));
                            continue;
                        }
                        _ => {} // We are not using Socket Connection/Disconnection Events
                    }
                }
                Err(error) => {
                    if let GaiaServerSocketError::SendError(address) = error {
                        self.client_connections.remove(&address);
                        output = Some(Ok(ServerEvent::Disconnection(address)));
                        continue;
                    }

                    output = Some(Err(GaiaServerError::Wrapped(Box::new(error))));
                    continue;
                }
            }
        }
        return output.unwrap();
    }

    pub fn send_event(&mut self, addr: SocketAddr, event: &impl NetEvent<T>) {
        if let Some(connection) = self.client_connections.get_mut(&addr) {
            connection.queue_event(event);
        }
    }

    async fn send_internal(&mut self, packet_type: PacketType, packet: Packet) {
        if let Some(connection) = self.client_connections.get_mut(&packet.address()) {
            let payload = connection.process_outgoing(packet_type, packet.payload());
            match self.sender.send(Packet::new_raw(packet.address(), payload))
                .await {
                Ok(_) => {}
                Err(err) => {
                    info!("send error! {}", err);
                }
            }
            connection.mark_sent();
        }
    }

    pub fn add_entity(&mut self, entity: Rc<RefCell<dyn NetEntity<U>>>) -> EntityKey {
        return self.global_entity_store.add_entity(entity);
    }

    pub fn remove_entity(&mut self, key: EntityKey) {
        self.scope_map.remove(&key);
        return self.global_entity_store.remove_entity(key);
    }

    pub fn get_entity(&mut self, key: EntityKey) -> Option<&Rc<RefCell<dyn NetEntity<U>>>> {
        return self.global_entity_store.get_entity(key);
    }

    pub fn scope_entity(&mut self, key: EntityKey, scope_func: Rc<Box<dyn Fn(&SocketAddr) -> bool>>) {
        self.scope_map.insert(key, scope_func);
    }

    pub fn get_clients(&mut self) -> Vec<SocketAddr> {
        self.client_connections.keys().cloned().collect()
    }

    pub fn get_sequence_number(&mut self, addr: SocketAddr) -> Option<u16> {
        if let Some(connection) = self.client_connections.get_mut(&addr) {
            return Some(connection.get_next_packet_index());
        }
        return None;
    }

    fn update_entity_scopes(&mut self) {
        for (address, connection) in self.client_connections.iter_mut() {
            for (key, entity) in self.global_entity_store.iter() {
                if let Some(scope_func) = self.scope_map.get(&key) {
                    let currently_in_scope: bool = connection.contains_key(key);
                    let should_be_in_scope = (scope_func.as_ref().as_ref())(address);
                    if currently_in_scope {
                        if !should_be_in_scope {
                            // remove entity from the connections local scope
                        }
                    } else {
                        if should_be_in_scope {
                            // add entity to the connections local scope
                        }
                    }
                }
            }
        }
    }
}