use std::marker::PhantomData;

use bevy::ecs::{
    system::{SystemMeta, SystemParamFetch, SystemParamState},
    world::World,
};

use naia_client::shared::Protocolize;

use super::client::Client;

// State

pub struct State<P: Protocolize> {
    phantom_p: PhantomData<P>,
}

// SAFE: only local state is accessed
unsafe impl<P: Protocolize> SystemParamState for State<P> {
    type Config = ();

    fn init(_world: &mut World, _system_state: &mut SystemMeta, _config: Self::Config) -> Self {
        State {
            phantom_p: PhantomData,
        }
    }

    fn apply(&mut self, _world: &mut World) {}

    fn default_config() {}
}

impl<'world, 'state, P: Protocolize> SystemParamFetch<'world, 'state> for State<P> {
    type Item = Client<'world, P>;

    #[inline]
    unsafe fn get_param(
        _state: &'state mut Self,
        _system_meta: &SystemMeta,
        world: &'world World,
        _change_tick: u32,
    ) -> Self::Item {
        Client::new(world)
    }
}
