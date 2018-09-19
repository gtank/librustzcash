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

impl ChainState for ChainManager {
    fn consensus_branch_id(&self) -> u32 {
        // TODO: Don't just return Sapling
        0x76b809bb
    }
}
