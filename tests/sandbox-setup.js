'use strict';
const nearAPI = require('near-api-js');
const BN = require('bn.js');
const fs = require('fs').promises;
const isReachable = require('is-reachable');

process.env.NEAR_NO_LOGS = 'defined';

const config = {
  networkId: 'sandbox',
  nodeUrl: 'http://0.0.0.0:3030',
  keyPath: '/tmp/near-usn-test-sandbox/validator_key.json',
  usnPath: './target/wasm32-unknown-unknown/sandbox/usn.wasm',
  priceoraclePath: './tests/priceoracle.wasm',
  amount: new BN('10000000000000000000000000', 10), // 25 digits, 10 NEAR
  masterId: 'test.near',
  usnId: 'usn.test.near',
  oracleId: 'priceoracle.test.near',
  aliceId: 'alice.test.near',
  bobId: 'bob.test.near',
};

const usnMethods = {
  viewMethods: [
    'version',
    'name',
    'symbol',
    'decimals',
    'spread',
    'contract_status',
    'get_owner',
    'ft_balance_of',
  ],
  changeMethods: [
    'new',
    'upgrade_name_symbol',
    'upgrade_icon',
    'blacklist_status',
    'add_to_blacklist',
    'remove_from_blacklist',
    'set_owner',
    'set_spread',
    'extend_guardians',
    'remove_guardians',
    'destroy_black_funds',
    'pause',
    'resume',
    'buy',
    'sell',
    'ft_transfer',
    'storage_deposit',
    'storage_unregister'
  ],
};

const oracleMethods = {
  changeMethods: ['new', 'add_asset', 'report_prices'],
};

async function sandboxSetup() {
  if (!(await isReachable(config.nodeUrl))) {
    throw new Error('Run sandbox first: `npm run sandbox`!');
  }

  const keyFile = require(config.keyPath);
  const privKey = nearAPI.utils.KeyPair.fromString(keyFile.secret_key);
  const pubKey = nearAPI.utils.PublicKey.fromString(keyFile.public_key);

  const keyStore = new nearAPI.keyStores.InMemoryKeyStore();
  keyStore.setKey(config.networkId, config.masterId, privKey);

  const near = await nearAPI.connect({
    deps: {
      keyStore,
    },
    networkId: config.networkId,
    nodeUrl: config.nodeUrl,
  });

  // Setup a global test context before anything else failed.
  global.near = near;

  let masterAccount = new nearAPI.Account(near.connection, config.masterId);

  // Create test accounts.
  await masterAccount.createAccount(config.usnId, pubKey, config.amount);
  await masterAccount.createAccount(config.oracleId, pubKey, config.amount);
  await masterAccount.createAccount(config.aliceId, pubKey, config.amount);
  await masterAccount.createAccount(config.bobId, pubKey, config.amount);
  keyStore.setKey(config.networkId, config.usnId, privKey);
  keyStore.setKey(config.networkId, config.oracleId, privKey);
  keyStore.setKey(config.networkId, config.aliceId, privKey);
  keyStore.setKey(config.networkId, config.bobId, privKey);

  // Deploy the USN contract.
  const wasm = await fs.readFile(config.usnPath);
  const usnAccount = new nearAPI.Account(near.connection, config.usnId);
  await usnAccount.deployContract(wasm);

  // Initialize the contract.
  const usnContract = new nearAPI.Contract(usnAccount, config.usnId, usnMethods);
  await usnContract.new({ args: { owner_id: config.usnId } });

  // Deploy the priceoracle contract.
  const wasmPriceoracle = await fs.readFile(config.priceoraclePath);
  const oracleAccount = new nearAPI.Account(near.connection, config.oracleId);
  await oracleAccount.deployContract(wasmPriceoracle);

  // Initialize the Oracle contract.
  const oracleContract = new nearAPI.Contract(
    oracleAccount,
    config.oracleId,
    oracleMethods
  );
  await oracleContract.new({ args: { recency_duration_sec: 360 } });
  await oracleContract.add_asset({ args: { asset_id: 'wrap.test.near' } });
  await oracleContract.report_prices({
    args: {
      prices: [
        {
          asset_id: 'wrap.test.near',
          price: { multiplier: '111439', decimals: 28 },
        },
      ],
    },
  });

  // Initialize other accounts connected to the contract for all test cases.
  const aliceAccount = new nearAPI.Account(near.connection, config.aliceId);
  const bobAccount = new nearAPI.Account(near.connection, config.bobId);
  const aliceContract = new nearAPI.Contract(aliceAccount, config.usnId, usnMethods);
  const bobContract = new nearAPI.Contract(bobAccount, config.usnId, usnMethods);

  // Setup a global test context.
  global.usnAccount = usnAccount;
  global.usnContract = usnContract;
  global.priceoracleContract = oracleContract;
  global.aliceAccount = aliceAccount;
  global.aliceContract = aliceContract;
  global.bobContract = bobContract;
}

async function sandboxTeardown() {
  const near = global.near;

  const alice = new nearAPI.Account(near.connection, config.aliceId);
  const bob = new nearAPI.Account(near.connection, config.bobId);
  const usn = new nearAPI.Account(near.connection, config.usnId);
  const oracle = new nearAPI.Account(near.connection, config.oracleId);

  await alice.deleteAccount(config.masterId);
  await bob.deleteAccount(config.masterId);
  await usn.deleteAccount(config.masterId);
  await oracle.deleteAccount(config.masterId);
}

module.exports = { config, sandboxSetup, sandboxTeardown };

module.exports.mochaHooks = {
  beforeAll: async function () {
    this.timeout(30000);
    await sandboxSetup();
  },
  afterAll: async function () {
    this.timeout(10000);
    await sandboxTeardown();
  },
};
