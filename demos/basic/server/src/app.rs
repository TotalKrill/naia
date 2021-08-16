use std::{rc::Rc, time::Duration};

use naia_server::{
    Event, Replicate, RoomKey, Server,
    ServerAddresses, ServerConfig, UserKey,
};

use naia_basic_demo_shared::{
    get_server_address, get_shared_config,
    protocol::{Character, Protocol, StringMessage},
};

pub struct App {
    server: Server<Protocol>,
    main_room_key: RoomKey,
    tick_count: u32,
}

impl App {
    pub async fn new() -> Self {
        let mut server_config = ServerConfig::default();

        server_config.socket_addresses = ServerAddresses::new(
            // IP Address to listen on for the signaling portion of WebRTC
            get_server_address(),
            // IP Address to listen on for UDP WebRTC data channels
            "127.0.0.1:14192"
                .parse()
                .expect("could not parse WebRTC data address/port"),
            // The public WebRTC IP address to advertise
            "127.0.0.1:14192"
                .parse()
                .expect("could not parse advertised public WebRTC data address/port"),
        );

        server_config.heartbeat_interval = Duration::from_secs(2);
        // Keep in mind that the disconnect timeout duration should always be at least
        // 2x greater than the heartbeat interval, to make it so at the worst case, the
        // server would need to miss 2 heartbeat signals before disconnecting from a
        // given client
        server_config.disconnection_timeout_duration = Duration::from_secs(5);

        let mut server =
            Server::new(Protocol::load(), Some(server_config), get_shared_config()).await;

        // This method is called during the connection handshake process, and can be
        // used to reject a new connection if the correct credentials have not been
        // provided
        server.on_auth(Rc::new(Box::new(|_, auth_type| {
            if let Protocol::Auth(auth_ref) = auth_type {
                let auth = auth_ref.borrow();
                let username = auth.username.get();
                let password = auth.password.get();
                return username == "charlie" && password == "12345";
            }
            return false;
        })));

        // Create a new, singular room, which will contain Users and Objects that they
        // can receive updates from
        let main_room_key = server.create_room();

        // Create 4 Character objects, with a range of X and name values
        {
            let mut count = 0;
            for (first, last) in [
                ("alpha", "red"),
                ("bravo", "blue"),
                ("charlie", "green"),
                ("delta", "yellow"),
            ]
            .iter()
            {
                count += 1;

                // Create a Character
                let character = Character::new((count * 4) as u8, 0, first, last);
                let character_key = server.register_object(character.to_protocol());

                // Add the Character to the main Room
                server.room_add_object(&main_room_key, &character_key);
            }
        }

        App {
            server,
            main_room_key,
            tick_count: 0,
        }
    }

    pub async fn update(&mut self) {
        match self.server.receive().await {
            Ok(event) => {
                match event {
                    Event::Connection(user_key) => {
                        self.server.room_add_user(&self.main_room_key, &user_key);
                        if let Some(user) = self.server.get_user(&user_key) {
                            info!("Naia Server connected to: {}", user.address);
                        }
                    }
                    Event::Disconnection(_, user) => {
                        info!("Naia Server disconnected from: {:?}", user.address);
                    }
                    Event::Message(user_key, message_type) => {
                        if let Some(user) = self.server.get_user(&user_key) {
                            match message_type {
                                Protocol::StringMessage(message_ref) => {
                                    let message = message_ref.borrow();
                                    let message_contents = message.contents.get();
                                    info!(
                                        "Naia Server recv <- {}: {}",
                                        user.address, message_contents
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    Event::Tick => {
                        // All game logic should happen here, on a tick event

                        // Message Sending
                        let mut iter_vec: Vec<UserKey> = Vec::new();
                        for (user_key, _) in self.server.users_iter() {
                            iter_vec.push(user_key);
                        }
                        for user_key in iter_vec {
                            let user = self.server.get_user(&user_key).unwrap();
                            let new_message = format!("Server Packet (tick {})", self.tick_count);
                            info!("Naia Server send -> {}: {}", user.address, new_message);

                            let message = StringMessage::new(new_message);
                            self.server.queue_message(&user_key, &message, true);
                        }

                        // Iterate through Characters, marching them from (0,0) to (20, N)
                        for object_key in self.server.objects_iter() {
                            let protocol = self.server.get_object(object_key).unwrap();
                            match protocol {
                                Protocol::Character(character_ref) => {
                                    character_ref.borrow_mut().step();
                                }
                                _ => {}
                            }
                        }

                        // Update scopes of objects
                        for (room_key, user_key, object_key) in self.server.object_scope_sets() {
                            if let Some(protocol) = self.server.get_object(&object_key) {
                                match protocol {
                                    Protocol::Character(character_ref) => {
                                        let x = *character_ref.borrow().x.get();
                                        let in_scope = x >= 5 && x <= 15;
                                        self.server.object_set_scope(
                                            &room_key,
                                            &user_key,
                                            &object_key,
                                            in_scope,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }

                        // VERY IMPORTANT! Calling this actually sends all update data
                        // packets to all Clients that require it. If you don't call this
                        // method, the Server will never communicate with it's connected Clients
                        self.server.send_all_updates().await;

                        self.tick_count = self.tick_count.wrapping_add(1);
                    }
                    _ => {}
                }
            }
            Err(error) => {
                info!("Naia Server Error: {}", error);
            }
        }
    }
}
