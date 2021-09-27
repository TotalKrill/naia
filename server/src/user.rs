use std::net::SocketAddr;

use naia_shared::Timestamp;

use super::world_type::WorldType;

#[allow(missing_docs)]
#[allow(unused_doc_comments)]
pub mod user_key {
    // The Key used to get a reference of a User
    new_key_type! { pub struct UserKey; }
}

#[derive(Clone)]
pub struct User {
    pub address: SocketAddr,
    pub timestamp: Timestamp,
}

impl User {
    pub fn new(address: SocketAddr, timestamp: Timestamp) -> User {
        User { address, timestamp }
    }
}

// user references

use naia_shared::ProtocolType;

use crate::{RoomKey, Server, UserKey};

// UserRef

pub struct UserRef<'s, P: ProtocolType, W: WorldType<P>> {
    server: &'s Server<P, W>,
    key: UserKey,
}

impl<'s, P: ProtocolType, W: WorldType<P>> UserRef<'s, P, W> {
    pub fn new(server: &'s Server<P, W>, key: &UserKey) -> Self {
        UserRef { server, key: *key }
    }

    pub fn key(&self) -> UserKey {
        self.key
    }

    pub fn address(&self) -> SocketAddr {
        return self.server.get_user_address(&self.key).unwrap();
    }
}

// UserMut
pub struct UserMut<'s, P: ProtocolType, W: WorldType<P>> {
    server: &'s mut Server<P, W>,
    key: UserKey,
}

impl<'s, P: ProtocolType, W: WorldType<P>> UserMut<'s, P, W> {
    pub fn new(server: &'s mut Server<P, W>, key: &UserKey) -> Self {
        UserMut { server, key: *key }
    }

    pub fn key(&self) -> UserKey {
        self.key
    }

    pub fn address(&self) -> SocketAddr {
        return self.server.get_user_address(&self.key).unwrap();
    }

    pub fn disconnect(&mut self) {
        self.server.user_force_disconnect(&self.key);
    }

    // Rooms

    pub fn enter_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.server.room_add_user(room_key, &self.key);

        self
    }

    pub fn leave_room(&mut self, room_key: &RoomKey) -> &mut Self {
        self.server.room_remove_user(room_key, &self.key);

        self
    }
}
