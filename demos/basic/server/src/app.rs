use naia_server::{Event, RoomKey, Server, ServerConfig, UserKey};

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
    pub fn new() -> Self {
        info!("Basic Naia Server Demo started");

        let shared_config = get_shared_config();

        let mut server_config = ServerConfig::default();
        server_config.socket_config.session_listen_addr = get_server_address();

        let mut server = Server::new(Some(server_config), shared_config);

        // Create a new, singular room, which will contain Users and Entities that they
        // can receive updates from
        let main_room_key = server.make_room().key();

        // Create 4 Character entities, with a range of X and name values
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
                let character_key = server.spawn_entity().insert_component(&character).key();

                // Add the Character Entity to the main Room
                server.room_mut(&main_room_key).add_entity(&character_key);
            }
        }

        App {
            server,
            main_room_key,
            tick_count: 0,
        }
    }

    pub fn update(&mut self) {
        for event in self.server.receive() {
            match event {
                Ok(Event::Authorization(user_key, Protocol::Auth(auth_ref))) => {
                    let auth_message = auth_ref.borrow();
                    let username = auth_message.username.get();
                    let password = auth_message.password.get();
                    if username == "charlie" && password == "12345" {
                        // Accept incoming connection
                        self.server.accept_connection(&user_key);
                    } else {
                        // Reject incoming connection
                        self.server.reject_connection(&user_key);
                    }
                }
                Ok(Event::Connection(user_key)) => {
                    info!(
                        "Naia Server connected to: {}",
                        self.server.user(&user_key).address()
                    );
                    self.server
                        .room_mut(&self.main_room_key)
                        .add_user(&user_key);
                }
                Ok(Event::Disconnection(_, user)) => {
                    info!("Naia Server disconnected from: {:?}", user.address);
                }
                Ok(Event::Message(user_key, Protocol::StringMessage(message_ref))) => {
                    let message = message_ref.borrow();
                    let message_contents = message.contents.get();
                    info!(
                        "Server recv from ({}) <- {}",
                        self.server.user(&user_key).address(),
                        message_contents
                    );
                }
                Ok(Event::Tick) => {
                    // All game logic should happen here, on a tick event

                    // Message sending
                    let mut iter_vec: Vec<UserKey> = Vec::new();
                    for (user_key, _) in self.server.users_iter() {
                        iter_vec.push(user_key);
                    }
                    for user_key in iter_vec {
                        let new_message_contents = format!("Server Message ({})", self.tick_count);
                        info!(
                            "Server send to   ({}) -> {}",
                            self.server.user(&user_key).address(),
                            new_message_contents
                        );

                        let new_message = StringMessage::new(new_message_contents);
                        self.server.queue_message(&user_key, &new_message, true);
                    }

                    // Iterate through Characters, marching them from (0,0) to (20, N)
                    for entity_key in self.server.entities_iter() {
                        if let Some(character_ref) =
                            self.server.entity(&entity_key).component::<Character>()
                        {
                            character_ref.borrow_mut().step();
                        }
                    }

                    // Update scopes of entities
                    for (room_key, user_key, entity_key) in self.server.scopes() {
                        if let Some(character_ref) =
                            self.server.entity(&entity_key).component::<Character>()
                        {
                            let x = *character_ref.borrow().x.get();
                            if x >= 5 && x <= 15 {
                                self.server.accept_scope(room_key, user_key, entity_key);
                            } else {
                                self.server.reject_scope(room_key, user_key, entity_key);
                            }
                        }
                    }

                    // VERY IMPORTANT! Calling this actually sends all update data
                    // packets to all Clients that require it. If you don't call this
                    // method, the Server will never communicate with it's connected Clients
                    self.server.send_all_updates();

                    self.tick_count = self.tick_count.wrapping_add(1);
                }
                Err(error) => {
                    info!("Naia Server Error: {}", error);
                }
                _ => {}
            }
        }
    }
}
