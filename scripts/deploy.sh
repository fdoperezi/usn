
# Sandbox account
ID=test.near

# Amount of tokens that have to be issued (total supply: 1000 tokens)
TOTAL_SUPPLY=1000000000000000000000000000

near deploy --wasm-file target/wasm32-unknown-unknown/release/usdt_gold.wasm \
            --account-id $ID \
            --master-account $ID \
            --networkId sandbox \
            --nodeUrl http://0.0.0.0:3030 \
            --keyPath /tmp/near-sandbox/validator_key.json

near call $ID new_default_meta \
            --account-id $ID \
            --args '{"owner_id": "'$ID'", "total_supply": "'$TOTAL_SUPPLY'"}' \
            --networkId sandbox \
            --nodeUrl http://0.0.0.0:3030 \
            --keyPath /tmp/near-sandbox/validator_key.json
