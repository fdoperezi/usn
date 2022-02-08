# USDN

The source code of USDN contract.

## Build

Add Rust `wasm32` target:
```bash
rustup target add wasm32-unknown-unknown
```
Build the contract:

```bash
cargo build --target wasm32-unknown-unknown --release
```

```bash
cargo test
```

## Deploy

### On `sandbox`:

Install sandbox:

```bash
npm install -g near-sandbox
near-sandbox --home /tmp/near-sandbox init
near-sandbox --home /tmp/near-sandbox run
```

Deploy:

```bash
$ near deploy --wasmFile target/wasm32-unknown-unknown/release/usdn.wasm --initFunction new_default_meta --initArgs '{"owner_id": "test.near", "1000000000000000000"}' --accountId test.near --networkId sandbox --nodeUrl http://0.0.0.0:3030 --keyPath /tmp/near-sandbox/validator_key.json
```

### On `mainnet`:

```bash
$ near deploy --wasmFile target/wasm32-unknown-unknown/release/usdn.wasm --initFunction new_default_meta --initArgs '{"owner_id": "usdn.near", "1000000000000000000"}' --accountId=usdn.near --networkId=mainnet --nodeUrl=https://rpc.mainnet.near.org

```
