
# Your official account
ID=usdt_golg.testnet

# Amount of tokens that have to be issued (total supply: 1000 tokens)
TOTAL_SUPPLY=1000000000000000000000000000

near login
near deploy --wasm-file target/wasm32-unknown-unknown/release/usdn.wasm \
            --account-id $ID \
            --master-account $ID \
            --initFunction "new_default_meta" \
            --initArgs '{"owner_id": "'$ID'", "total_supply": "'$TOTAL_SUPPLY'"}'

