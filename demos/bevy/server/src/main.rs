use bevy::prelude::*;
use bevy::ecs::schedule::ShouldRun;
use bevy::log::LogPlugin;
use bevy::ecs::entity::Entity as BevyEntityKey;

use std::collections::HashMap;

use naia_server::{Event, EntityKey as NaiaEntityKey, Random, RoomKey, Server, ServerConfig, UserKey, Ref};

use naia_bevy_demo_shared::{
    behavior as shared_behavior, get_server_address, get_shared_config,
    protocol::{Color, ColorValue, Protocol, Position},
};

static ALL: &str = "all";

// Component definitions
struct Pawn;
struct NonPawn;
struct Key(NaiaEntityKey);

// Resource definitions
struct ServerResource {
    main_room_key: RoomKey,
    naia_to_bevy_key_map: HashMap<NaiaEntityKey, BevyEntityKey>,
    bevy_to_naia_key_map: HashMap<BevyEntityKey, NaiaEntityKey>,
    ticked: bool,
}

fn main() {
    let mut app = App::build();

    // Plugins
    app.add_plugins(MinimalPlugins)
       .add_plugin(LogPlugin::default())
       .add_stage_before(
        CoreStage::PreUpdate,
        ALL,
        SystemStage::single_threaded(),
    );

    // Naia Server initialization
    let shared_config = get_shared_config();
    let mut server_config = ServerConfig::default();
    server_config.socket_config.session_listen_addr = get_server_address();
    let mut server = Server::new(Protocol::load(), Some(server_config), shared_config);

    // Create a new, singular room, which will contain Users and Entities that they
    // can receive updates from
    let main_room_key = server.create_room();

    // Resources
    app.insert_non_send_resource(server);
    app.insert_resource(ServerResource {
        main_room_key,
        naia_to_bevy_key_map: HashMap::new(),
        bevy_to_naia_key_map: HashMap::new(),
        ticked: false,
    });

    // Systems
    app.add_system_to_stage(ALL, naia_server_update.system())
       .add_system_to_stage(ALL, on_tick.system()
                                                         .with_run_criteria(
                                                             did_consume_tick.system()))

    // Run
       .run();
}


fn naia_server_update(
    mut commands: Commands,
    mut server: NonSendMut<Server<Protocol>>,
    mut server_resource: ResMut<ServerResource>,
    mut c_q: Query<(&Ref<Position>)>,
) {
    for event in server.receive() {
        match event {
            Ok(Event::Authorization(user_key, Protocol::Auth(auth_ref))) => {
                let auth_message = auth_ref.borrow();
                let username = auth_message.username.get();
                let password = auth_message.password.get();
                if username == "charlie" && password == "12345" {
                    // Accept incoming connection
                    server.accept_connection(&user_key);
                } else {
                    // Reject incoming connection
                    server.reject_connection(&user_key);
                }
            }
            Ok(Event::Connection(user_key)) => {
                server.room_add_user(&server_resource.main_room_key, &user_key);
                if let Some(user) = server.get_user(&user_key) {
                    info!("Naia Server connected to: {}", user.address);

                    // Create new Square Entity in Naia
                    let naia_entity = server.register_entity();

                    // Create new Square Entity in Bevy
                    let mut bevy_entity = commands.spawn();

                    // Update sync map
                    server_resource.naia_to_bevy_key_map.insert(naia_entity, bevy_entity.id());
                    server_resource.bevy_to_naia_key_map.insert(bevy_entity.id(), naia_entity);

                    // Add Naia Entity to main Room
                    server.room_add_entity(&server_resource.main_room_key, &naia_entity);

                    // Color component
                    {
                        // create
                        let mut x = Random::gen_range_u32(0, 40) as i16;
                        let mut y = Random::gen_range_u32(0, 30) as i16;
                        x -= 20;
                        y -= 15;
                        x *= 16;
                        y *= 16;
                        let position_ref = Position::new(x, y);

                        // add to Naia
                        let _position_component_key = server.add_component_to_entity(&naia_entity, &position_ref);

                        // add to Bevy
                        bevy_entity.insert(Ref::clone(&position_ref));
                    }

                    // Color component
                    {
                        // create
                        let color_value = match server.get_users_count() % 3 {
                            0 => ColorValue::Yellow,
                            1 => ColorValue::Red,
                            _ => ColorValue::Blue,
                        };
                        let color_ref = Color::new(color_value);

                        // add to Naia
                        let _color_component_key = server.add_component_to_entity(&naia_entity, &color_ref);

                        // add to Bevy
                        bevy_entity.insert(Ref::clone(&color_ref));
                    }

                    // Assign as Pawn to User
                    server.assign_pawn_entity(&user_key, &naia_entity);
                }
            }
            Ok(Event::Disconnection(user_key, user)) => {
                info!("Naia Server disconnected from: {:?}", user.address);
//                server.room_remove_user(&server_resource.main_room_key, &user_key);
//                if let Some(object_key) = server_resource.user_to_pawn_map.remove(&user_key) {
//                    server
//                        .room_remove_object(&server_resource.main_room_key, &object_key);
//                    server.unassign_pawn(&user_key, &object_key);
//                    server.deregister_object(&object_key);
//                }
            }
            Ok(Event::CommandEntity(_, naia_entity, Protocol::KeyCommand(key_command_ref))) => {
                if let Some(bevy_entity) = server_resource.naia_to_bevy_key_map.get(&naia_entity) {
                    // TODO: use a query here to get at the entity...
                    if let Ok((position_ref)) = c_q.get_mut(*bevy_entity) {
                        shared_behavior::process_command(&key_command_ref, position_ref);
                    }
                }
            }
            Ok(Event::Tick) => {
                server_resource.ticked = true;
            }
            Err(error) => {
                info!("Naia Server error: {}", error);
            }
            _ => {}
        }
    }
}

fn did_consume_tick(
    mut server_resource: ResMut<ServerResource>,
) -> ShouldRun {
    if server_resource.ticked {
        server_resource.ticked = false;
        return ShouldRun::Yes;
    }
    return ShouldRun::No;
}

fn on_tick(
    mut server: NonSendMut<Server<Protocol>>,
    mut server_resource: ResMut<ServerResource>,
) {
    // All game logic should happen here, on a tick event
    //info!("tick");

    // Update scopes of objects
    for (room_key, user_key, object_key) in server.object_scope_sets() {
        server
            .object_set_scope(&room_key, &user_key, &object_key, true);
    }

    // VERY IMPORTANT! Calling this actually sends all update data
    // packets to all Clients that require it. If you don't call this
    // method, the Server will never communicate with it's connected Clients
    server.send_all_updates();
}