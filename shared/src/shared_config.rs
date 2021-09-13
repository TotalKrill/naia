use std::time::Duration;

use naia_socket_shared::LinkConditionerConfig;

use crate::{Manifest, ProtocolType};

/// Contains Config properties which will be shared by Server and Client
#[derive(Debug)]
pub struct SharedConfig<P: ProtocolType> {
    /// The Manifest generated by the Protocol which handles Replication
    pub manifest: Manifest<P>,
    /// The duration between each tick
    pub tick_interval: Duration,
    /// Configuration used to simulate network conditions
    pub link_condition_config: Option<LinkConditionerConfig>,
}

impl<P: ProtocolType> SharedConfig<P> {
    /// Creates a new SharedConfig
    pub fn new(
        manifest: Manifest<P>,
        tick_interval: Duration,
        link_condition_config: Option<LinkConditionerConfig>,
    ) -> Self {
        SharedConfig {
            manifest,
            tick_interval,
            link_condition_config,
        }
    }
}
