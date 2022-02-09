
# Sandbox account
ID=test.near

# Amount of tokens that have to be issued (total supply: 1000 tokens)
TOTAL_SUPPLY=1000000000000000000000000000

near deploy --wasm-file target/wasm32-unknown-unknown/release/usdn.wasm \
            --account-id $ID \
            --master-account $ID \
            --networkId sandbox \
            --nodeUrl http://0.0.0.0:3030 \
            --keyPath /tmp/near-usdn-sandbox/validator_key.json \
            --force

near call $ID new \
            --account-id $ID \
            --args '{"total_supply": "'$TOTAL_SUPPLY'"}' \
            --networkId sandbox \
            --nodeUrl http://0.0.0.0:3030 \
            --keyPath /tmp/near-usdn-sandbox/validator_key.json
