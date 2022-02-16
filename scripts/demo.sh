
ID=test.near
RED="\033[0;31m"
NC='\033[0m'
SANDBOX=" --networkId sandbox --nodeUrl http://0.0.0.0:3030 --keyPath /tmp/near-usn-sandbox/validator_key.json"

# Amount of tokens that have to be issued (total supply: 1000 tokens)
TOTAL_SUPPLY=1000000000000000000000000000

near deploy --wasm-file target/wasm32-unknown-unknown/release/usn.wasm \
            --initFunction new \
            --initArgs '{"owner_id": "'${ID}'"}' \
            --account-id $ID \
            --master-account $ID \
            --force \
            $SANDBOX

echo -e "${NC}"
near create-account bob.$ID --masterAccount $ID --initialBalance 1 $SANDBOX
near call $ID storage_deposit '' --accountId bob.$ID --amount 0.00125 $SANDBOX

echo -e "\n${RED}BOB BUYS SOME TOKENS:${NC}"
near call $ID extend_guardians --accountId $ID --args '{"guardians": ["'bob.$ID'"]}' $SANDBOX
near call $ID buy --accountId bob.$ID --amount 0.1 $SANDBOX
near view $ID ft_balance_of --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}TOTAL SUPPLY:${NC}"
near view $ID ft_total_supply --args '{}' $SANDBOX

echo -e "\n${RED}TRANSFER:${NC}"
near call $ID ft_transfer --accountId bob.$ID --args '{"receiver_id": "'$ID'", "amount": "1"}' --amount 0.000000000000000000000001 $SANDBOX

echo -e "\n${RED}IS BOB IN THE BLACKLIST:${NC}"
near call $ID get_blacklist_status --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}BOB TRYING HIMSELF ADD TO THE BLACKLIST:${NC}"
near call $ID add_to_blacklist --accountId bob.$ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX
near call $ID get_blacklist_status --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}TEST.NEAR TRYING ADD BOB TO THE BLACKLIST:${NC}"
near call $ID add_to_blacklist --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX
near call $ID get_blacklist_status --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}BURN BANNED BOB FUNDS:${NC}"
near call $ID destroy_black_funds --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX
near view $ID ft_balance_of --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}UNBAN BOB:${NC}"
near call $ID remove_from_blacklist --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX
near call $ID get_blacklist_status --accountId $ID --args '{"account_id": "'bob.$ID'"}' $SANDBOX

echo -e "\n${RED}MAINTENANCE ON:${NC}"
near call $ID pause --accountId $ID --args '{}' $SANDBOX
near call $ID contract_status --accountId $ID --args '{}' $SANDBOX

echo -e "\n${RED}TRANSFER:${NC}"
near call $ID ft_transfer --accountId $ID --args '{"receiver_id": "'bob.$ID'", "amount": "1"}' --amount 0.000000000000000000000001 $SANDBOX

echo -e "\n${RED}MAINTENANCE OFF:${NC}"
near call $ID resume --accountId $ID --args '{}' $SANDBOX
near call $ID contract_status --accountId $ID --args '{}' $SANDBOX
