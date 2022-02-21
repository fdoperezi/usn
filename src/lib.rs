mod event;
mod oracle;
mod owner;

use near_contract_standards::fungible_token::core::FungibleTokenCore;
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::resolver::FungibleTokenResolver;
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, LookupMap, UnorderedSet};
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, log, near_bindgen, sys, AccountId, Balance,
    BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseOrValue,
};

use std::convert::TryFrom;
use std::fmt::Debug;

use oracle::{ExchangeRate, Oracle};

const NO_DEPOSIT: Balance = 0;
const TOKEN_DECIMAL: u8 = 18;
const GAS_FOR_PROMISE: Gas = Gas(5_000_000_000_000);

const DEFAULT_SPREAD: Balance = 10_000; // 0.01 (10^4 / 10^6) = 1%
const MAX_SPREAD: Balance = 50_000; // 0.05 = 5%
const SPREAD_DECIMAL: u8 = 6;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    Guardians,
    Token,
    TokenMetadata,
    Blacklist,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum BlackListStatus {
    // An address might be using
    Allowable,
    // All acts with an address have to be banned
    Banned,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ContractStatus {
    Working,
    Paused,
}

impl std::fmt::Display for ContractStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractStatus::Working => write!(f, "working"),
            ContractStatus::Paused => write!(f, "paused"),
        }
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner_id: AccountId,
    guardians: UnorderedSet<AccountId>,
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
    black_list: LookupMap<AccountId, BlackListStatus>,
    status: ContractStatus,
    oracle: Oracle,
    spread: Balance,
}

const DATA_IMAGE_SVG_NEAR_ICON: &str =
    "data:image/svg+xml;charset=UTF-8,%3csvg width='245' height='245' viewBox='0 0 245 245' fill='none' xmlns='http://www.w3.org/2000/svg'%3e%3ccircle cx='122.5' cy='122.5' r='122.5' fill='white'/%3e%3cpath d='M78 179V67H93.3342L152.668 154.935V67H167V179H151.666L92.3325 90.9891V179H78Z' fill='black'/%3e%3cpath d='M150 104C147.239 104 145 106.239 145 109C145 111.761 147.239 114 150 114V104ZM171 114C173.761 114 176 111.761 176 109C176 106.239 173.761 104 171 104V114ZM150 114H171V104H150V114Z' fill='black'/%3e%3cpath d='M150 125C147.239 125 145 127.239 145 130C145 132.761 147.239 135 150 135V125ZM171 135C173.761 135 176 132.761 176 130C176 127.239 173.761 125 171 125V135ZM150 135H171V125H150V135Z' fill='black'/%3e%3c/svg%3e";

#[ext_contract(ext_self)]
trait ContractCallback {
    #[private]
    fn buy_with_rate_callback(&self, near: Balance, #[callback] rate: ExchangeRate) -> Balance;

    #[private]
    fn sell_with_rate_callback(&self, tokens: Balance, #[callback] rate: ExchangeRate) -> Balance;
}

#[near_bindgen]
impl Contract {
    #[private]
    pub fn buy_with_rate_callback(
        &mut self,
        near: Balance,
        #[callback] rate: ExchangeRate,
    ) -> Balance {
        self.finish_buy(near, rate)
    }

    #[private]
    pub fn sell_with_rate_callback(
        &mut self,
        tokens: Balance,
        #[callback] rate: ExchangeRate,
    ) -> Balance {
        self.finish_sell(tokens, rate)
    }
}

#[near_bindgen]
impl Contract {
    /// Initializes the contract owned by the given `owner_id` with default metadata.
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        let metadata = FungibleTokenMetadata {
            spec: FT_METADATA_SPEC.to_string(),
            name: "USD Near".to_string(),
            symbol: "USN".to_string(),
            icon: Some(DATA_IMAGE_SVG_NEAR_ICON.to_string()),
            reference: None,
            reference_hash: None,
            decimals: TOKEN_DECIMAL,
        };

        let mut this = Self {
            owner_id: owner_id.clone(),
            guardians: UnorderedSet::new(StorageKey::Guardians),
            token: FungibleToken::new(StorageKey::Token),
            metadata: LazyOption::new(StorageKey::TokenMetadata, Some(&metadata)),
            black_list: LookupMap::new(StorageKey::Blacklist),
            status: ContractStatus::Working,
            oracle: Oracle::default(),
            spread: DEFAULT_SPREAD,
        };

        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, NO_DEPOSIT);
        this
    }

    pub fn upgrade_name_symbol(&mut self, name: String, symbol: String) {
        self.assert_owner();
        self.abort_if_pause();
        let metadata = self.metadata.take();
        if let Some(mut metadata) = metadata {
            metadata.name = name;
            metadata.symbol = symbol;
            self.metadata.replace(&metadata);
        }
    }

    pub fn upgrade_icon(&mut self, data: String) {
        self.assert_owner();
        self.abort_if_pause();
        let metadata = self.metadata.take();
        if let Some(mut metadata) = metadata {
            metadata.icon = Some(data);
            self.metadata.replace(&metadata);
        }
    }

    pub fn blacklist_status(&self, account_id: &AccountId) -> BlackListStatus {
        return match self.black_list.get(account_id) {
            Some(x) => x.clone(),
            None => BlackListStatus::Allowable,
        };
    }

    pub fn add_to_blacklist(&mut self, account_id: &AccountId) {
        self.assert_owner();
        self.abort_if_pause();
        self.black_list.insert(account_id, &BlackListStatus::Banned);
    }

    pub fn remove_from_blacklist(&mut self, account_id: &AccountId) {
        self.assert_owner();
        self.abort_if_pause();
        self.black_list
            .insert(account_id, &BlackListStatus::Allowable);
    }

    pub fn destroy_black_funds(&mut self, account_id: &AccountId) {
        self.assert_owner();
        self.abort_if_pause();
        assert_eq!(
            self.blacklist_status(&account_id),
            BlackListStatus::Banned
        );
        let black_balance = self.ft_balance_of(account_id.clone());
        if black_balance.0 <= 0 {
            env::panic_str("The account doesn't have enough balance");
        }
        self.token.accounts.insert(account_id, &0u128);
        self.token.total_supply = self
            .token
            .total_supply
            .checked_sub(u128::from(black_balance))
            .expect("Failed to decrease total supply");
    }

    /// Pauses the contract. Only can be called by owner or guardians.
    #[payable]
    pub fn pause(&mut self) {
        assert_one_yocto();
        self.assert_owner();
        self.status = ContractStatus::Paused;
    }

    /// Resumes the contract. Only can be called by owner.
    pub fn resume(&mut self) {
        self.assert_owner();
        self.status = ContractStatus::Working;
    }

    /// Buys USN tokens for NEAR tokens.
    /// Can make cross-contract call to an oracle.
    /// Returns amount of purchased USN tokens.
    #[payable]
    pub fn buy(&mut self) -> PromiseOrValue<Balance> {
        self.assert_owner_or_guardian();
        self.abort_if_pause();
        self.abort_if_blacklisted();

        let near = env::attached_deposit();
        let exchange_rate = self.oracle.get_exchange_rate();

        match exchange_rate {
            PromiseOrValue::Value(rate) => PromiseOrValue::Value(self.finish_buy(near, rate)),
            PromiseOrValue::Promise(rate) => {
                PromiseOrValue::Promise(rate.then(ext_self::buy_with_rate_callback(
                    near,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_PROMISE,
                )))
            }
        }
    }

    /// Completes the purchase (NEAR -> USN). It is called in 2 cases:
    /// 1. Direct call from the `buy` method if the exchange rate cache is valid.
    /// 2. Indirect callback from the cross-contract call after getting a fresh exchange rate.
    fn finish_buy(&mut self, near: Balance, rate: ExchangeRate) -> Balance {
        // An account of the original request. In a case of cross-contract call,
        // it's the account of the original `buy` operation.
        let buyer = env::signer_account_id();

        let spread = 10u128.pow(SPREAD_DECIMAL as u32) - self.spread; // 1 - 0.01
        let amount = near * rate.multiplier() * spread
            / 10u128.pow(u32::from(rate.decimals() - TOKEN_DECIMAL + SPREAD_DECIMAL));

        if amount == 0 {
            env::panic_str("Not enough NEAR: attached deposit exchanges to 0 tokens");
        }

        self.token.internal_deposit(&buyer, amount);

        event::emit::ft_mint(&buyer, amount, None);

        amount
    }

    /// Sells USN tokens getting NEAR tokens.
    /// Return amount of purchased NEAR tokens.
    pub fn sell(&mut self, amount: U128) -> PromiseOrValue<Balance> {
        self.assert_owner_or_guardian();
        self.abort_if_pause();
        self.abort_if_blacklisted();

        let amount = Balance::from(amount);

        if amount == 0 {
            env::panic_str("Not enough tokens to sell: exchange more than 0 tokens");
        }

        let exchange_rate = self.oracle.get_exchange_rate();

        match exchange_rate {
            PromiseOrValue::Value(rate) => PromiseOrValue::Value(self.finish_sell(amount, rate)),
            PromiseOrValue::Promise(rate) => rate
                .then(ext_self::sell_with_rate_callback(
                    amount,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_FOR_PROMISE,
                ))
                .into(),
        }
    }

    /// Finishes the sell (USN -> NEAR). It is called in 2 cases:
    /// 1. Direct call from the `sell` method if the exchange rate cache is valid.
    /// 2. Indirect callback from the cross-contract call after getting a fresh exchange rate.
    fn finish_sell(&mut self, amount: Balance, rate: ExchangeRate) -> Balance {
        // An account of the original request. In a case of cross-contract call,
        // it's the account of the original `sell` operation.
        let seller = env::signer_account_id();

        let spread = 10u128.pow(SPREAD_DECIMAL as u32) + self.spread; // 1 + 0.01
        let deposit = amount
            * 10u128.pow(u32::from(rate.decimals() - TOKEN_DECIMAL + SPREAD_DECIMAL))
            / (rate.multiplier() * spread);

        self.token.internal_withdraw(&seller, amount);

        event::emit::ft_burn(&seller, amount, None);

        Promise::new(seller).transfer(deposit);

        deposit
    }

    pub fn contract_status(&self) -> ContractStatus {
        self.status.clone()
    }

    /// Returns the name of the token.
    pub fn name(&self) -> String {
        let metadata = self.metadata.get();
        metadata.expect("Unable to get decimals").name
    }

    /// Returns the symbol of the token.
    pub fn symbol(&self) -> String {
        let metadata = self.metadata.get();
        metadata.expect("Unable to get decimals").symbol
    }

    /// Returns the decimals places of the token.
    pub fn decimals(&self) -> u8 {
        let metadata = self.metadata.get();
        metadata.expect("Unable to get decimals").decimals
    }

    pub fn spread(&self) -> u128 {
        self.spread
    }

    pub fn set_spread(&mut self, spread: Balance) {
        self.assert_owner();
        self.abort_if_pause();
        if spread > MAX_SPREAD {
            env::panic_str(&format!(
                "Spread limit is {}%",
                MAX_SPREAD / 10u128.pow(SPREAD_DECIMAL as u32)
            ));
        }
        self.spread = spread;
    }

    pub fn version(&self) -> String {
        format!("{}:{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
    }

    /// This is NOOP implementation. KEEP IT if you haven't changed contract state.
    /// Should only be called by this contract on migration.
    /// This method is called from `update()` method.
    /// For next version upgrades, change this function.
    #[init(ignore_state)]
    #[private]
    pub fn migrate() -> Self {
        let this: Contract = env::state_read().expect("Contract is not initialized");
        this
    }

    fn abort_if_pause(&self) {
        if self.status == ContractStatus::Paused {
            env::panic_str("The contract is under maintenance")
        }
    }

    fn abort_if_blacklisted(&self) {
        let account_id = env::predecessor_account_id();
        if self.blacklist_status(&account_id) != BlackListStatus::Allowable {
            env::panic_str(&format!("Account '{}' is banned", account_id));
        }
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        event::emit::storage_unregister(account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        event::emit::ft_burn(&account_id, amount, None)
    }
}

#[no_mangle]
pub fn upgrade() {
    env::setup_panic_hook();

    let contract: Contract = env::state_read().expect("Contract is not initialized");
    contract.assert_owner();

    const MIGRATE_METHOD_NAME: &[u8; 7] = b"migrate";
    const UPDATE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

    unsafe {
        // Load code into register 0 result from the input argument if factory call or from promise if callback.
        sys::input(0);
        // Create a promise batch to update current contract with code from register 0.
        let promise_id = sys::promise_batch_create(
            env::current_account_id().as_bytes().len() as u64,
            env::current_account_id().as_bytes().as_ptr() as u64,
        );
        // Deploy the contract code from register 0.
        sys::promise_batch_action_deploy_contract(promise_id, u64::MAX, 0);
        // Call promise to migrate the state.
        // Batched together to fail upgrade if migration fails.
        sys::promise_batch_action_function_call(
            promise_id,
            MIGRATE_METHOD_NAME.len() as u64,
            MIGRATE_METHOD_NAME.as_ptr() as u64,
            0,
            0,
            0,
            (env::prepaid_gas() - env::used_gas() - UPDATE_GAS_LEFTOVER).0,
        );
        sys::promise_return(promise_id);
    }
}

/// The core methods for a basic fungible token. Extension standards may be
/// added in addition to this macro.

#[near_bindgen]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.abort_if_pause();
        self.abort_if_blacklisted();
        let sender_id = AccountId::try_from(env::signer_account_id())
            .expect("Couldn't validate sender address");
        assert!(u128::from(self.ft_balance_of(sender_id)) >= u128::from(amount));
        self.token.ft_transfer(receiver_id.clone(), amount, memo);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.abort_if_pause();
        self.abort_if_blacklisted();
        let sender_id = AccountId::try_from(env::signer_account_id())
            .expect("Couldn't validate sender address");
        assert!(u128::from(self.ft_balance_of(sender_id)) >= u128::from(amount));
        self.token
            .ft_transfer_call(receiver_id.clone(), amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let sender_id: AccountId = sender_id.into();
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            self.on_tokens_burned(sender_id, burned_amount);
        }
        used_amount.into()
    }
}

near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance};

    use super::*;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        const TOTAL_SUPPLY: Balance = 0;
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new(accounts(1));
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        const AMOUNT: Balance = 3_000_000_000_000_000_000_000_000;

        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(2));
        contract.token.internal_deposit(&accounts(2), AMOUNT);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = AMOUNT / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(
            contract.ft_balance_of(accounts(2)).0,
            (AMOUNT - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }

    #[test]
    fn test_blacklist() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));

        // Act as a user.
        testing_env!(context
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(2))
            .build());

        assert_eq!(
            contract.blacklist_status(&accounts(1)),
            BlackListStatus::Allowable
        );

        contract.storage_deposit(None, None);
        contract.token.internal_deposit(&accounts(2), 1000);
        assert_eq!(contract.ft_balance_of(accounts(2)), U128::from(1000));

        // Act as owner.
        testing_env!(context.predecessor_account_id(accounts(1)).build());

        contract.add_to_blacklist(&accounts(2));
        assert_eq!(
            contract.blacklist_status(&accounts(2)),
            BlackListStatus::Banned
        );

        contract.remove_from_blacklist(&accounts(2));
        assert_eq!(
            contract.blacklist_status(&accounts(2)),
            BlackListStatus::Allowable
        );

        contract.add_to_blacklist(&accounts(2));
        let total_supply_before = contract.token.total_supply;

        assert_ne!(contract.ft_balance_of(accounts(2)), U128::from(0));

        contract.destroy_black_funds(&accounts(2));
        assert_ne!(total_supply_before, contract.token.total_supply);

        assert_eq!(contract.ft_balance_of(accounts(2)), U128::from(0));
    }

    #[test]
    #[should_panic]
    fn test_user_cannot_destroy_black_funds() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(2));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .predecessor_account_id(accounts(1))
            .build());

        contract.add_to_blacklist(&accounts(1));
    }

    #[test]
    fn test_maintenance() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(1))
            .current_account_id(accounts(1))
            .signer_account_id(accounts(1))
            .build());
        assert_eq!(contract.contract_status(), ContractStatus::Working);
        contract.pause();
        assert_eq!(contract.contract_status(), ContractStatus::Paused);
        contract.resume();
        assert_eq!(contract.contract_status(), ContractStatus::Working);
        contract.pause();
        contract.ft_total_supply();
    }

    #[test]
    #[should_panic]
    fn test_extend_guardians_by_user() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        testing_env!(context.predecessor_account_id(accounts(2)).build());
        contract.extend_guardians(vec![accounts(3)]);
    }

    #[test]
    fn test_guardians() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.extend_guardians(vec![accounts(2)]);
        assert!(contract.guardians.contains(&accounts(2)));
        contract.remove_guardians(vec![accounts(2)]);
        assert!(!contract.guardians.contains(&accounts(2)));
    }

    #[test]
    #[should_panic]
    fn test_cannot_remove_guardians() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.extend_guardians(vec![accounts(2)]);
        assert!(contract.guardians.contains(&accounts(2)));
        contract.remove_guardians(vec![accounts(3)]);
    }

    #[test]
    #[should_panic]
    fn test_cannot_buy_sell() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        testing_env!(context.predecessor_account_id(accounts(2)).build());
        contract.buy();
    }

    #[test]
    fn test_buy_sell() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = Contract::new(accounts(1));
        contract.extend_guardians(vec![accounts(2)]);

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .build());
        contract.storage_deposit(None, None);

        const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(ONE_NEAR)
            .build());

        contract
            .oracle
            .set_exchange_rate(ExchangeRate::test_fresh_rate());

        match contract.buy() {
            PromiseOrValue::Value(v) => assert_eq!(v, 11032461000000000000),
            _ => panic!("Must return a value"),
        };
        match contract.sell(U128::from(11032461000000000000)) {
            PromiseOrValue::Value(v) => assert!(v < ONE_NEAR && v > (ONE_NEAR * 8 / 10)),
            _ => panic!("Must return a value"),
        };

        contract
            .oracle
            .set_exchange_rate(ExchangeRate::test_old_rate());

        match contract.buy() {
            PromiseOrValue::Value(_) => panic!("Must return a promise"),
            _ => {}
        };
        match contract.sell(U128::from(9900000000000000000)) {
            PromiseOrValue::Value(_) => panic!("Must return a promise"),
            _ => {}
        };
    }

    #[test]
    #[should_panic(expected = "Account 'charlie' is banned")]
    fn test_cannot_buy() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = Contract::new(accounts(1));
        contract.extend_guardians(vec![accounts(2)]);
        contract.add_to_blacklist(&accounts(2)); // It'll cause panic on buy.

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .build());
        contract.storage_deposit(None, None);

        const ONE_NEAR: Balance = 1_000_000_000_000_000_000_000_000;

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(ONE_NEAR)
            .build());
        contract.buy();
    }

    #[test]
    #[should_panic(expected = "Account 'charlie' is banned")]
    fn test_cannot_sell() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = Contract::new(accounts(1));
        contract.extend_guardians(vec![accounts(2)]);
        contract.add_to_blacklist(&accounts(2)); // It'll cause panic on sell.

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .build());
        contract.storage_deposit(None, None);

        testing_env!(context.predecessor_account_id(accounts(2)).build());
        contract.sell(U128::from(1));
    }

    #[test]
    #[should_panic(expected = "Not enough NEAR")]
    fn test_cannot_buy_zero() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = Contract::new(accounts(1));
        contract.extend_guardians(vec![accounts(2)]);

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .build());
        contract.storage_deposit(None, None);

        const TOO_LESS_NEAR: Balance = 90_000;

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(TOO_LESS_NEAR)
            .build());

        contract
            .oracle
            .set_exchange_rate(ExchangeRate::test_fresh_rate());
        contract.buy();
    }

    #[test]
    #[should_panic(expected = "Not enough tokens to sell")]
    fn test_cannot_sell_zero() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());

        let mut contract = Contract::new(accounts(1));
        contract.extend_guardians(vec![accounts(2)]);

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .build());
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .signer_account_id(accounts(2))
            .build());

        contract.token.internal_deposit(&accounts(2), 1);
        contract
            .oracle
            .set_exchange_rate(ExchangeRate::test_fresh_rate());
        contract.sell(U128::from(0));
    }

    #[test]
    fn test_spread() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1));
        assert_eq!(contract.spread(), DEFAULT_SPREAD);
        contract.set_spread(MAX_SPREAD);
        assert_eq!(contract.spread(), MAX_SPREAD);
        let res = std::panic::catch_unwind(move || contract.set_spread(MAX_SPREAD + 1));
        assert!(res.is_err());
    }
}
