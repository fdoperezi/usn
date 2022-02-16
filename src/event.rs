pub mod emit {
    use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
    use near_sdk::serde_json::json;

    use crate::*;

    pub fn ft_mint(owner_id: &AccountId, amount: Balance, memo: Option<&str>) {
        (FtMint {
            owner_id: owner_id,
            amount: &amount.into(),
            memo: memo,
        })
        .emit();
    }

    pub fn ft_burn(owner_id: &AccountId, amount: Balance, memo: Option<&str>) {
        (FtBurn {
            owner_id: owner_id,
            amount: &amount.into(),
            memo: memo,
        })
        .emit();
    }

    pub fn storage_unregister(owner_id: AccountId, amount: Balance) {
        let event = json!({
            "standard": "nep145",
            "version": "1.0.0",
            "event": "storage_unregister",
            "data": [
                {"owner_id": owner_id, "amount": U128::from(amount)}
            ]
        });

        log!("EVENT_JSON:{}", event.to_string());
    }
}
