use near_sdk::{require, ONE_YOCTO};

use crate::*;

use std::collections::HashMap;

const MINIMUM_DEPOSIT: u128 = 1000_000; // $1000000

const GAS_FOR_GET_PROMISE: Gas = Gas(10_000_000_000_000);
const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(35_000_000_000_000);
const GAS_FOR_HANDLE_ADD_LIQUIDITY_PROMISE: Gas = Gas(20_000_000_000_000);
const GAS_FOR_ADD_LIQUIDITY_PROMISE: Gas = Gas(10_000_000_000_000);

const USDT_DECIMALS: u8 = 6;

struct PoolConfig {
    pub ref_address: &'static str,
    pub usdt_address: &'static str,
    pub stable_pool_id: u64,
}

const CONFIG: PoolConfig = if cfg!(feature = "mainnet") {
    PoolConfig {
        ref_address: "v2.ref-finance.near",
        usdt_address: "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near",
        stable_pool_id: 3020,
    }
} else if cfg!(feature = "testnet") {
    PoolConfig {
        ref_address: "ref-finance-101.testnet",
        usdt_address: "usdt.fakes.testnet",
        stable_pool_id: 356,
    }
} else {
    PoolConfig {
        ref_address: "ref.test.near",
        usdt_address: "usdt.test.near",
        stable_pool_id: 0,
    }
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct StablePoolInfo {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    pub decimals: Vec<u8>,
    /// backend tokens.
    pub amounts: Vec<U128>,
    /// backend tokens in comparable precision
    pub c_amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
    pub amp: u64,
}

#[ext_contract(ext_ref_finance)]
trait RefFinance {
    fn get_stable_pool(&self, pool_id: u64) -> StablePoolInfo;

    fn get_deposits(&self, account_id: AccountId) -> HashMap<AccountId, U128>;

    #[payable]
    fn add_stable_liquidity(&mut self, pool_id: u64, amounts: Vec<U128>, min_shares: U128) -> U128;

    #[payable]
    fn remove_liquidity_by_tokens(
        &mut self,
        pool_id: u64,
        amounts: Vec<U128>,
        max_burn_shares: U128,
    ) -> U128;
}

#[ext_contract(ext_ft)]
trait Usdt {
    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise;
}

#[ext_contract(ext_pool_self)]
trait RefFinanceHandler {
    #[private]
    #[payable]
    fn handle_deposit_then_add_liquidity(
        &mut self,
        whole_amount: U128,
        #[callback] deposits: HashMap<AccountId, U128>,
        #[callback] info: StablePoolInfo,
    ) -> Promise;
}

trait RefFinanceHandler {
    fn handle_deposit_then_add_liquidity(
        &mut self,
        whole_amount: U128,
        deposits: HashMap<AccountId, U128>,
        info: StablePoolInfo,
    ) -> Promise;
}

#[near_bindgen]
impl RefFinanceHandler for Contract {
    #[private]
    #[payable]
    fn handle_deposit_then_add_liquidity(
        &mut self,
        whole_amount: U128,
        #[callback] deposits: HashMap<AccountId, U128>,
        #[callback] info: StablePoolInfo,
    ) -> Promise {
        let usn_addr = env::current_account_id();
        let usdt_addr = CONFIG.usdt_address.parse().unwrap();
        let ref_addr = CONFIG.ref_address.parse().unwrap();
        let usn_amount: u128 = extend_decimals(whole_amount.0, self.decimals());
        let usdt_amount: u128 = extend_decimals(whole_amount.0, USDT_DECIMALS);
        let usdt_deposit: u128 = deposits.get(&usdt_addr).unwrap_or(&0u128.into()).0;
        let usn_deposit: u128 = deposits.get(&usn_addr).unwrap_or(&0u128.into()).0;

        require!(
            usdt_deposit >= usdt_amount,
            &format!("Not enough USDT: {} < {},", usdt_deposit, usdt_amount)
        );
        require!(
            usn_deposit >= usn_amount,
            &format!("Not enough USN: {} < {},", usn_deposit, usn_amount)
        );

        // Preserve the sequence of token amounts.
        let token_amounts = info
            .token_account_ids
            .iter()
            .map(|token| {
                if *token == usdt_addr {
                    U128::from(usdt_amount)
                } else if *token == usn_addr {
                    U128::from(usn_amount)
                } else {
                    env::panic_str(&format!("Unexpected token in the pool: {}", token));
                }
            })
            .collect::<Vec<U128>>();

        let min_shares = NO_DEPOSIT.into();

        ext_ref_finance::add_stable_liquidity(
            CONFIG.stable_pool_id,
            token_amounts,
            min_shares,
            ref_addr,
            env::attached_deposit(),
            GAS_FOR_ADD_LIQUIDITY_PROMISE,
        )
    }
}

#[near_bindgen]
impl Contract {
    pub fn stable_pool_id(&self) -> u64 {
        CONFIG.stable_pool_id
    }

    #[payable]
    pub fn transfer_stable_liquidity(&mut self, whole_amount: U128) -> Promise {
        self.assert_owner();

        // 1st yoctoNEAR is for USDT ft_transfer_call.
        // More NEARs could be required for add_stable_liquidity.
        require!(
            env::attached_deposit() > 0,
            "Requires attached deposit of at least 1 yoctoNEAR"
        );

        require!(
            whole_amount.0 >= MINIMUM_DEPOSIT,
            &format!("The minimum deposit: ${}", MINIMUM_DEPOSIT)
        );

        let usn_addr = env::current_account_id();
        let usdt_addr = CONFIG.usdt_address.parse().unwrap();
        let ref_addr = CONFIG.ref_address.parse().unwrap();
        let usn_balance = self.token.internal_unwrap_balance_of(&usn_addr);
        let usn_amount: u128 = extend_decimals(whole_amount.0, self.decimals());
        let usdt_amount: u128 = extend_decimals(whole_amount.0, USDT_DECIMALS);

        // Mint necessary USN amount.
        if usn_balance < usn_amount {
            let yet_to_mint = usn_amount - usn_balance;
            self.token.internal_mint(&usn_addr, yet_to_mint);
            event::emit::ft_mint(&usn_addr, yet_to_mint, None);
        }

        // Do 2 transfers: "usn":USN -> ref-finance, and "usn":USDT -> ref-finance.
        // Ignoring transfer results: relying on the deposit state.
        self.token
            .internal_transfer_call(
                &usn_addr,
                &ref_addr,
                usn_amount,
                GAS_FOR_FT_TRANSFER_CALL,
                None,
                "".to_string(), // Empty message == deposit action on the ref-finance.
            )
            .then(ext_ft::ft_transfer_call(
                ref_addr.clone(),
                usdt_amount.into(),
                None,
                "".to_string(), // Empty message == deposit action on the ref-finance.
                usdt_addr,
                ONE_YOCTO,
                GAS_FOR_FT_TRANSFER_CALL,
            ))
            // Double-check deposits and pool configuration.
            .then(ext_ref_finance::get_deposits(
                usn_addr.clone(),
                ref_addr.clone(),
                NO_DEPOSIT,
                GAS_FOR_GET_PROMISE,
            ))
            .and(ext_ref_finance::get_stable_pool(
                CONFIG.stable_pool_id,
                ref_addr.clone(),
                0,
                GAS_FOR_GET_PROMISE,
            ))
            .then(ext_pool_self::handle_deposit_then_add_liquidity(
                whole_amount,
                usn_addr,
                env::attached_deposit() - 1,
                GAS_FOR_HANDLE_ADD_LIQUIDITY_PROMISE,
            ))
    }
}

fn extend_decimals(whole: u128, decimals: u8) -> u128 {
    whole * 10u128.pow(decimals as u32)
}