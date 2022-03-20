/**
* Fungible Token NEP-141 Token contract
*
* The aim of the contract is to provide a basic implementation of the improved function token standard.
*
* lib.rs is the main entry point.
* fungible_token_core.rs implements NEP-146 standard
* storage_manager.rs implements NEP-145 standard for allocating storage per account
* fungible_token_metadata.rs implements NEP-148 standard for providing token-specific metadata.
* internal.rs contains internal methods for fungible token.
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap};
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, Promise, StorageUsage};

pub use crate::fungible_token_core::*;
pub use crate::fungible_token_metadata::*;
use crate::internal::*;
pub use crate::storage_manager::*;
use std::convert::TryInto;
use std::num::ParseIntError;

mod fungible_token_core;
mod fungible_token_metadata;
mod internal;
mod storage_manager;

#[global_allocator]
static ALLOC: near_sdk::wee_alloc::WeeAlloc<'_> = near_sdk::wee_alloc::WeeAlloc::INIT;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub owner_id: AccountId,

    /// AccountID -> Account balance.
    pub accounts: LookupMap<AccountId, Balance>,

    /// Total supply of the all token.
    pub total_supply: Balance,

    /// The storage size in bytes for one account.
    pub account_storage_usage: StorageUsage,

    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        total_supply: U128,
        version: String,
        name: String,
        symbol: String,
        reference: String,
        reference_hash: String,
        decimals: u8,
    ) -> Self {
        require!(!env::state_exists(), "Already initialized");

        let ref_hash_result: Result<Vec<u8>, ParseIntError> = (0..reference_hash.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&reference_hash[i..i + 2], 16))
            .collect();

        let ref_hash_fixed_bytes: [u8; 32] =
            ref_hash_result.unwrap().as_slice().try_into().unwrap();

        let mut this = Self {
            owner_id: owner_id.clone().into(),
            accounts: LookupMap::new(b"a".to_vec()),
            total_supply: total_supply.into(),
            account_storage_usage: 0,
            ft_metadata: LazyOption::new(
                b"m".to_vec(),
                Some(&FungibleTokenMetadata {
                    version,
                    name,
                    symbol,
                    reference,
                    reference_hash: ref_hash_fixed_bytes,
                    decimals,
                }),
            ),
        };

        // Determine cost of insertion into LookupMap

        let initial_storage_usage = env::storage_usage();
        let tmp_account_id = unsafe { String::from_utf8_unchecked(vec![b'a'; 64]) };
        this.accounts.insert(&tmp_account_id, &0u128);
        this.account_storage_usage = env::storage_usage() - initial_storage_usage;
        this.accounts.remove(&tmp_account_id);

        // Make owner have total supply

        let total_supply_u128: u128 = total_supply.into();
        this.accounts.insert(&owner_id.as_ref(), &total_supply_u128);
        this
    }

    /// Owner only methods

    pub fn mint(&mut self, amount: U128) {
        self.assert_owner(); // Only owner can call

        let amount: Balance = amount.into();
        let owner_id = self.owner_id.clone();

        if let Some(new_total_supply) = self.total_supply.checked_add(amount) {
            self.total_supply = new_total_supply;
        } else {
            env::panic(b"Total Supply Overflow");
        }

        self.internal_deposit(&owner_id, amount);

        // ToDo - Mint Event
    }

    pub fn ft_transfer_player_reward(
        &mut self,
        player_id: ValidAccountId,
        amount: U128,
        feat: Option<String>,
    ) {
        self.assert_owner();
        let amount: Balance = amount.into();

        require!(amount > 0, "The amount should be a positive number");

        let owner_id = self.owner_id.clone();
        let player_id: AccountId = player_id.into();

        self.internal_withdraw(&owner_id, amount);
        self.internal_deposit(&player_id, amount);

        // ToDo - Transfer Reward Event
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod fungible_token_tests {
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    use super::*;
    use near_sdk::json_types::ValidAccountId;
    use std::convert::TryFrom;

    const ZERO_U128: Balance = 0u128;

    fn alice() -> ValidAccountId {
        ValidAccountId::try_from("alice.near").unwrap()
    }
    fn bob() -> ValidAccountId {
        ValidAccountId::try_from("bob.near").unwrap()
    }
    fn carol() -> ValidAccountId {
        ValidAccountId::try_from("carol.near").unwrap()
    }
    fn dex() -> ValidAccountId {
        ValidAccountId::try_from("dex.near").unwrap()
    }

    fn get_context(predecessor_account_id: AccountId) -> VMContext {
        VMContext {
            current_account_id: "mike.near".to_string(),
            signer_account_id: "bob.near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id,
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 1000 * 10u128.pow(24),
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    fn create_contract() -> Contract {
        Contract::new(
            dex(),
            U128::from(1_000_000_000_000_000),
            String::from("0.1.0"),
            String::from("NEAR Test Token"),
            String::from("TEST"),
            String::from("https://github.com/near/core-contracts/tree/master/w-near-141"),
            "7c879fa7b49901d0ecc6ff5d64d7f673da5e4a5eb52a8d50a214175760d8919a".to_string(),
            24,
        )
    }

    #[test]
    fn contract_creation_with_new() {
        testing_env!(get_context(dex().as_ref().to_string()));

        let contract = create_contract();

        assert_eq!(contract.ft_total_supply().0, 1_000_000_000_000_000);
        assert_eq!(contract.ft_balance_of(alice()).0, ZERO_U128);
        assert_eq!(contract.ft_balance_of(bob().into()).0, ZERO_U128);
        assert_eq!(contract.ft_balance_of(carol().into()).0, ZERO_U128);
        assert_eq!(
            contract.ft_balance_of(dex().into()).0,
            1_000_000_000_000_000
        );
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn default_fails() {
        testing_env!(get_context(carol().into()));
        let _contract = Contract::default();
    }

    #[test]
    fn test_mint_success() {
        testing_env!(get_context(dex().as_ref().to_string()));

        let mut contract = create_contract();
        contract.mint(U128::from(5));

        assert_eq!(contract.ft_total_supply().0, 1_000_000_000_000_005);
        assert_eq!(
            contract.ft_balance_of(dex().into()).0,
            1_000_000_000_000_005
        );
    }

    #[test]
    #[should_panic(expected = "It is a owner only method")]
    fn test_mint_fail() {
        testing_env!(get_context(alice().as_ref().to_string()));
        let mut contract = create_contract();
        contract.mint(U128::from(5));
    }
}
