use std::{ops::Deref, time::Duration};

use solana_client::rpc_client::{RpcClient, RpcClientConfig};
use solana_sdk::commitment_config::CommitmentConfig;

use super::custom_http_sender::CustomHttpSender;

pub struct SolanaClient(RpcClient);

/// # Explorer - origin: https://explorer.solana.com
/// https://explorer-api.mainnet-beta.solana.com/
///
/// # MagicEden - origin: https://magiceden.io
/// https://magicede-magicede-c0f1.mainnet.rpcpool.com/
/// https://lively-palpable-isle.quiknode.pro/
/// https://solitary-white-violet.solana-mainnet.quiknode.pro/
///
/// # Hyperspace - origin: https://hyperspace.xyz
/// https://hyperspace.rpcpool.com/
///
/// # SolanaFM - origin: https://solana.fm
/// https://qn.solana.fm/
/// 
/// # Cryptostarps - origin: https://cryptostraps.tools
/// https://alice.genesysgo.net/
/// https://pentacle.genesysgo.net/
impl SolanaClient {
    pub fn new<U: ToString>(url: U) -> Self {
        let timeout = Duration::from_secs(45);
        let config = RpcClientConfig {
            commitment_config: CommitmentConfig::confirmed(),
            confirm_transaction_initial_timeout: Some(timeout),
        };

        let sender = CustomHttpSender::new(url);
        let client = RpcClient::new_sender(sender, config);

        SolanaClient(client)
    }
}

impl Deref for SolanaClient {
    type Target = RpcClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
