use crate::*;
use near_sdk::{CryptoHash};
use std::mem::size_of;

//convert the royalty percentage and amount to pay into a payout (U128)
pub(crate) fn royalty_to_payout(royalty_percentage: u32, amount_to_pay: Balance) -> U128 {
    U128(royalty_percentage as u128 * amount_to_pay / 10_000u128)
}

//calculate how many bytes the account ID is taking up
pub(crate) fn bytes_for_approved_account_id(account_id: &AccountId) -> u64 {
    account_id.as_str().len() as u64 + 4 + size_of::<u64>() as u64
}

//refund the storage taken up by passed in approved account IDs and send the funds to the passed in account ID. 
pub(crate) fn refund_approved_account_ids_iter<'a, I>(
    account_id: AccountId,
    approved_account_ids: I, 
) -> Promise
where
    I: Iterator<Item = &'a AccountId>,
{
    //get the storage total by going through and summing all the bytes for each approved account IDs
    let storage_released: u64 = approved_account_ids.map(bytes_for_approved_account_id).sum();
    //transfer the account the storage that is released
    Promise::new(account_id).transfer(Balance::from(storage_released) * env::storage_byte_cost())
}

//refund a map of approved account IDs and send the funds to the passed in account ID
pub(crate) fn refund_approved_account_ids(
    account_id: AccountId,
    approved_account_ids: &HashMap<AccountId, u64>,
) -> Promise {
    refund_approved_account_ids_iter(account_id, approved_account_ids.keys())
}

//used to generate a unique prefix in our storage collections (this is to avoid data collisions)
pub(crate) fn hash_account_id(account_id: &AccountId) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
    hash
}

//used to make sure the user attached exactly 1 yoctoNEAR
pub(crate) fn assert_one_yocto() {
    assert_eq!(
        env::attached_deposit(),
        1,
        "Requires attached deposit of exactly 1 yoctoNEAR",
    )
}

//Assert that the user has attached at least 1 yoctoNEAR (for security reasons and to pay for storage)
pub(crate) fn assert_at_least_one_yocto() {
    assert!(
        env::attached_deposit() >= 1,
        "Requires attached deposit of at least 1 yoctoNEAR",
    )
}

//refund the initial deposit based on the amount of storage that was used up
pub(crate) fn refund_deposit(storage_used: u64) {
    let required_cost = env::storage_byte_cost() * Balance::from(storage_used);
    let attached_deposit = env::attached_deposit();

    assert!(
        required_cost <= attached_deposit,
        "Must attach {} yoctoNEAR to cover storage",
        required_cost,
    );

    let refund = attached_deposit - required_cost;

    if refund > 1 {
        Promise::new(env::predecessor_account_id()).transfer(refund);
    }
}

impl Contract {
    //add a token to the set of tokens an owner has
    pub(crate) fn internal_add_token_to_owner(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) {
        //get the set of tokens for the given account
        let mut tokens_set = self.tokens_per_owner.get(account_id).unwrap_or_else(|| {
            //if the account doesn't have any tokens, we create a new unordered set
            UnorderedSet::new(
                StorageKey::TokenPerOwnerInner {
                    //we get a new unique prefix for the collection
                    account_id_hash: hash_account_id(&account_id),
                }
                .try_to_vec()
                .unwrap(),
            )
        });

        tokens_set.insert(token_id);

        self.tokens_per_owner.insert(account_id, &tokens_set);
    }

    //remove a token from an owner (internal method and can't be called directly via CLI).
    pub(crate) fn internal_remove_token_from_owner(
        &mut self,
        account_id: &AccountId,
        token_id: &TokenId,
    ) {
        let mut tokens_set = self
            .tokens_per_owner
            .get(account_id)
            .expect("Token should be owned by the sender");

        tokens_set.remove(token_id);

        if tokens_set.is_empty() {
            self.tokens_per_owner.remove(account_id);
        } else {
            self.tokens_per_owner.insert(account_id, &tokens_set);
        }
    }

    //transfers the NFT to the receiver_id (internal method and can't be called directly via CLI).
    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> Token {
        //get the token object by passing in the token_id
        let token = self.tokens_by_id.get(token_id).expect("No token");

		if sender_id != &token.owner_id {
			//if the token's approved account IDs doesn't contain the sender, we panic
			if !token.approved_account_ids.contains_key(sender_id) {
				env::panic_str("Unauthorized");
			}

			if let Some(enforced_approval_id) = approval_id {
				let actual_approval_id = token
					.approved_account_ids
					.get(sender_id)
					.expect("Sender must be approved");

                assert_eq!(
					actual_approval_id, &enforced_approval_id,
					"The actual approval_id {} is different from the given approval_id {}",
					actual_approval_id, enforced_approval_id,
				);
			}
		}

        assert_ne!(
            &token.owner_id, receiver_id,
            "The token owner and the receiver should be different"
        );

        self.internal_remove_token_from_owner(&token.owner_id, token_id);
        self.internal_add_token_to_owner(receiver_id, token_id);

        let new_token = Token {
            owner_id: receiver_id.clone(),
            approved_account_ids: Default::default(),
            next_approval_id: token.next_approval_id,
            royalty: token.royalty.clone(),
        };
        self.tokens_by_id.insert(token_id, &new_token);

        if let Some(memo) = memo.as_ref() {
            env::log_str(&format!("Memo: {}", memo).to_string());
        }

        let mut authorized_id = None;
        if approval_id.is_some() {
            authorized_id = Some(sender_id.to_string());
        }

        let nft_transfer_log: EventLog = EventLog {
            // Standard name ("nep177").
            standard: NFT_STANDARD_NAME.to_string(),
            // Version of the standard ("nft-2.0.0").
            version: NFT_METADATA_SPEC.to_string(),
            event: EventLogVariant::NftTransfer(vec![NftTransferLog {
                authorized_id,
                old_owner_id: token.owner_id.to_string(),
                new_owner_id: receiver_id.to_string(),
                token_ids: vec![token_id.to_string()],
                memo,
            }]),
        };

        env::log_str(&nft_transfer_log.to_string());
        
        token
    }
} 