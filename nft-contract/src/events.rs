use std::fmt;

use near_sdk::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
#[serde(crate = "near_sdk::serde")]
#[non_exhaustive]
pub enum EventLogVariant {
    NftMint(Vec<NftMintLog>),
    NftTransfer(Vec<NftTransferLog>),
}

/// Interface to capture data about an event
///
/// Arguments:
/// * `standard`: name of standard e.g. nep177
/// * `version`: e.g. 2.0.0
/// * `event`: associate event data
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct EventLog {
    pub standard: String,
    pub version: String,

    // `flatten` to not have "event": {<EventLogVariant>} in the JSON, just have the contents of {<EventLogVariant>}.
    #[serde(flatten)] // check it
    pub event: EventLogVariant,
}

impl fmt::Display for EventLog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "EVENT_JSON:{}",
            &serde_json::to_string(self).map_err(|_| fmt::Error)?
        ))
    }
}

/// An event log to capture token minting
///
/// Arguments
/// * `owner_id`: "account.near"
/// * `token_ids`: ["1", "abc"]
/// * `memo`: optional message
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct NftMintLog {
    pub owner_id: String,
    pub token_ids: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Arguments
/// * `authorized_id`: approved account to transfer
/// * `old_owner_id`: "owner.near"
/// * `new_owner_id`: "receiver.near"
/// * `token_ids`: ["1", "12345abc"]
/// * `memo`: optional message
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct NftTransferLog {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<String>,

    pub old_owner_id: String,
    pub new_owner_id: String,
    pub token_ids: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nep_format_vector() {
        let expected = r#"EVENT_JSON:{"standard":"nep177","version":"2.0.0","event":"nft_mint","data":[{"owner_id":"bdrv7.testnet","token_ids":["test_token","abc"]},{"owner_id":"bdrv7.testnet","token_ids":["test_token_1"]}]}"#;
        let log = EventLog {
            standard: "nep177".to_string(),
            version: "2.0.0".to_string(),
            event: EventLogVariant::NftMint(vec![
                NftMintLog {
                    owner_id: "bdrv7.testnet".to_owned(),
                    token_ids: vec!["test_token".to_string(), "abc".to_string()],
                    memo: None,
                },
                NftMintLog {
                    owner_id: "bdrv7.testnet".to_owned(),
                    token_ids: vec!["test_token_1".to_string()],
                    memo: None,
                },
            ]),
        };
        assert_eq!(expected, log.to_string());
    }

    #[test]
    fn nep_format_mint() {
        let expected = r#"EVENT_JSON:{"standard":"nep177","version":"2.0.0","event":"nft_mint","data":[{"owner_id":"bdrv7.testnet","token_ids":["test_token","abc"]}]}"#;
        let log = EventLog {
            standard: "nep177".to_string(),
            version: "2.0.0".to_string(),
            event: EventLogVariant::NftMint(vec![NftMintLog {
                owner_id: "bdrv7.testnet".to_owned(),
                token_ids: vec!["test_token".to_string(), "abc".to_string()],
                memo: None,
            }]),
        };
        assert_eq!(expected, log.to_string());
    }

    #[test]
    fn nep_format_transfer_all_fields() {
        let expected = r#"EVENT_JSON:{"standard":"nep177","version":"2.0.0","event":"nft_transfer","data":[{"authorized_id":"market.bdrv7.testnet","old_owner_id":"old.bdrv7.testnet","new_owner_id":"new.bdrv7.testnet","token_ids":["test_token"],"memo":"test_memo"}]}"#;
        let log = EventLog {
            standard: "nep177".to_string(),
            version: "2.0.0".to_string(),
            event: EventLogVariant::NftTransfer(vec![NftTransferLog {
                authorized_id: Some("market.bdrv7.testnet".to_string()),
                old_owner_id: "old.bdrv7.testnet".to_string(),
                new_owner_id: "new.bdrv7.testnet".to_string(),
                token_ids: vec!["test_token".to_string()],
                memo: Some("test_memo".to_owned()),
            }]),
        };
        assert_eq!(expected, log.to_string());
    }
} 