use std::net::SocketAddr;

use chain::ChainManager;
use constants::COIN_TYPE_TEST;
use keystore::LocalKeyStore;
use prover::MockTxProver;
use sender::MockTxSender;
use types::{KeyStore, TxProver, TxSender};
use Wallet;

pub struct Builder {
    coin_type: u32,
    ks: Option<Box<KeyStore>>,
    csd: Option<SocketAddr>,
    prover: Option<Box<TxProver>>,
    sender: Option<Box<TxSender>>,
}

impl Builder {
    /// Create a blank Builder.
    pub fn new() -> Self {
        Builder {
            coin_type: COIN_TYPE_TEST,
            csd: None,
            ks: None,
            prover: None,
            sender: None,
        }
    }

    /// Use a local KeyStore with the provided seed.
    pub fn local_key_store(self, seed: &[u8]) -> Self {
        self.key_store(Box::new(LocalKeyStore::from_seed(seed)))
    }

    /// Use the provided KeyStore.
    pub fn key_store(mut self, ks: Box<KeyStore>) -> Self {
        self.ks = Some(ks);
        self
    }

    /// Synchronise chain state using the provided server.
    pub fn chain_sync(mut self, csd: SocketAddr) -> Self {
        self.csd = Some(csd);
        self
    }

    /// Configure the transaction proof generator.
    pub fn tx_prover(mut self, prover: Box<TxProver>) -> Self {
        self.prover = Some(prover);
        self
    }

    /// Configure how transactions will be sent to the network.
    pub fn tx_sender(mut self, sender: Box<TxSender>) -> Self {
        self.sender = Some(sender);
        self
    }

    /// Build a Wallet.
    pub fn build(self) -> Wallet {
        let ks = self.ks.expect("A KeyStore must be provided");

        let cs = Box::new(ChainManager::new(
            self.csd
                .expect("A chain synchronisation server must be specified"),
        ));

        let prover = match self.prover {
            Some(prover) => prover,
            None => Box::new(MockTxProver {}),
        };

        let sender = match self.sender {
            Some(sender) => sender,
            None => Box::new(MockTxSender {}),
        };

        Wallet::new(self.coin_type, ks, cs, prover, sender)
    }
}
