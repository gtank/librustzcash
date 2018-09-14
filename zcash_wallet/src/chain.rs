use std::net::SocketAddr;

use types::ChainState;

pub(crate) struct ChainManager {
    server: SocketAddr,
}

impl ChainManager {
    pub(crate) fn new(server: SocketAddr) -> Self {
        ChainManager { server }
    }
}

impl ChainState for ChainManager {}
