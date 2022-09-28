use std::str::FromStr;
use std::sync::Arc;

use crate::rpc::client::SolanaClient;
use borsh::BorshDeserialize;
use mpl_token_metadata::pda::find_metadata_account;
use mpl_token_metadata::state::Metadata;
use serde::Serialize;
use solana_account_decoder::parse_account_data::{ParsableAccount, PARSABLE_PROGRAM_IDS};
use solana_account_decoder::parse_token::{TokenAccountType, UiTokenAmount};
use solana_client::client_error::{ClientError, ClientErrorKind, Result as ClientResult};
use solana_client::rpc_request::{RpcRequest, TokenAccountsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;

#[derive(Serialize, Clone, PartialEq, Eq)]
pub struct TokenMetadata {
    pub update_authority: String,
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,
}

pub struct SolanaCrawler {
    client: Arc<SolanaClient>,
}

impl SolanaCrawler {
    pub fn new<U: ToString>(url: U) -> Self {
        let client = SolanaClient::new(url);
        Self {
            client: Arc::new(client),
        }
    }

    pub async fn get_version(&self) -> ClientResult<String> {
        let client = self.client.clone();
        let result = actix_web::rt::task::spawn_blocking(move || client.get_version())
            .await
            .map_err(|_| ClientError {
                request: None,
                kind: ClientErrorKind::Custom(
                    "get_version: fatal error in spawning blocking thread".into(),
                ),
            })?;

        let result = result.map(|x| x.solana_core);
        result
    }

    pub async fn get_nfts_for_owner(&self, addr: &str) -> ClientResult<Vec<TokenMetadata>> {
        let account = Pubkey::from_str(addr).map_err(|_| ClientError {
            request: None,
            kind: ClientErrorKind::Custom(
                "get_nfts_for_owner: fatal error in validating owner address".into(),
            ),
        })?;

        let client = self.client.clone();
        let result = actix_web::rt::task::spawn_blocking(move || {
            let output = client
                .get_token_accounts_by_owner(
                    &account,
                    TokenAccountsFilter::ProgramId(spl_token::id()),
                )
                .expect("msg");

            let mut nfts: Vec<TokenMetadata> = vec![];
            for i in output.iter() {
                let program_owner = Pubkey::from_str(i.account.owner.as_str()).unwrap();
                if !spl_token::check_id(&program_owner) {
                    continue;
                }

                let program_name = PARSABLE_PROGRAM_IDS.get(&program_owner).unwrap();
                let data = match &i.account.data {
                    solana_account_decoder::UiAccountData::Json(data) => Some(data.clone()),
                    _ => None,
                };

                if let Some(d) = data {
                    match program_name {
                        ParsableAccount::SplToken | ParsableAccount::SplToken2022 => {
                            let tmp = serde_json::from_value::<TokenAccountType>(d.parsed).unwrap();
                            match tmp {
                                TokenAccountType::Account(t) => {
                                    let UiTokenAmount {
                                        ui_amount,
                                        decimals,
                                        ..
                                    } = t.token_amount;

                                    if let Some(ui_amount_absolute) = ui_amount {
                                        let amount = spl_token::ui_amount_to_amount(
                                            ui_amount_absolute,
                                            decimals,
                                        );

                                        if amount == 1 {
                                            let mint_account = Pubkey::from_str(t.mint.as_str())
                                                .expect("msg");
                                            let (metadata_account, _) = find_metadata_account(&mint_account);
                                            let info = client.get_account_with_commitment(&metadata_account, CommitmentConfig::confirmed())
                                                .expect("msg");

                                            if info.value.is_none() {
                                                continue;
                                            }

                                            let account_data = info.value
                                                .ok_or(ClientError {
                                                    request: Some(RpcRequest::GetAccountInfo),
                                                    kind: ClientErrorKind::Custom(
                                                        "get_nfts_for_owner: fatal error retrieving account data".into(),
                                                    ),
                                                })
                                                .expect("msg");

                                            let mut sliced_data = account_data.data.as_slice();
                                            let meta = Metadata::deserialize(&mut sliced_data).unwrap();

                                            let uri = Some(meta.data.uri.trim_end_matches("\0").to_string());
                                            let uri = bincode::serialize(&uri)
                                                .expect("msg");

                                            let safe_uri = base64::encode(uri);

                                            nfts.push(TokenMetadata {
                                                update_authority: meta.update_authority.to_string(),
                                                mint: meta.mint.to_string(),
                                                name: meta.data.name.trim_end_matches("\0").to_string(),
                                                symbol: meta.data.symbol.trim_end_matches("\0").to_string(),
                                                uri: safe_uri,
                                            });
                                        }
                                    }
                                }
                                _ => panic!(),
                            }
                        }
                        _ => panic!(),
                    };
                }
            }

            nfts
        })
        .await
        .map_err(|_| ClientError {
            request: None,
            kind: ClientErrorKind::Custom(
                "get_nfts_for_owner: fatal error in spawning blocking thread".into(),
            ),
        });

        result
    }
}
