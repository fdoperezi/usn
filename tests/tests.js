'use strict';

const assert = require('assert').strict;
const config = require('./sandbox-setup').config;

const ONE_NEAR = '1000000000000000000000000';
const ONE_YOCTO = '1';
const HUNDRED_NEARS = '100000000000000000000000000';
const GAS_FOR_CALL = '200000000000000'; // 200 TGas

describe('Smoke Test', function () {
  it('should get a version', async () => {
    const version = await global.aliceContract.version();
    assert.match(version, /0\.1\./);
  });
});

describe('Anyone', function () {
  it('should get a name', async () => {
    const name = await global.aliceContract.name();
    assert.equal(name, 'USN');
  });

  it('should get a symbol', async () => {
    const symbol = await global.aliceContract.symbol();
    assert.equal(symbol, 'USN');
  });

  it('should get decimals', async () => {
    const decimals = await global.aliceContract.decimals();
    assert.equal(decimals, 18);
  });

  it('should get a spread', async () => {
    const spread = await global.aliceContract.spread();
    assert.equal(spread, '5000');
  });

  it('should get contract status', async () => {
    const status = await global.aliceContract.contract_status();
    assert.equal(status, 'Working');
  });

  it('should get an owner', async () => {
    const owner = await global.aliceContract.owner();
    assert.equal(owner, config.usnId);
  });

  it('should get a fake storage balance', async () => {
    const storage_balance = await global.aliceContract.storage_balance_of({
      account_id: 'fake.near',
    });
    assert.deepEqual(storage_balance, {
      total: '1250000000000000000000',
      available: '0',
    });
  });
});

describe('Owner', function () {
  this.timeout(5000);

  it('should be able to assign guardians', async () => {
    await assert.doesNotReject(async () => {
      await global.usnContract.extend_guardians({
        args: { guardians: [config.aliceId] },
      });
    });
  });

  it('should be able to remove guardians', async () => {
    await assert.doesNotReject(async () => {
      await global.usnContract.extend_guardians({
        args: { guardians: [config.aliceId] },
      });
      await global.usnContract.remove_guardians({
        args: { guardians: [config.aliceId] },
      });
    });
  });
});

describe('Owner', function () {
  this.timeout(5000);

  before(async () => {
    await global.usnContract.set_owner({
      args: { owner_id: config.aliceId },
    });
    assert.equal(await global.usnContract.owner(), config.aliceId);
  });

  it('can change ownership', async () => {
    await assert.rejects(async () => {
      await global.usnContract.set_owner({ args: { owner_id: config.usnId } });
    });
  });

  after(async () => {
    await global.aliceContract.set_owner({
      args: { owner_id: config.usnId },
    });
    assert.equal(await global.aliceContract.owner(), config.usnId);
  });
});

describe('Guardian', function () {
  this.timeout(5000);

  before(async () => {
    await global.usnContract.extend_guardians({
      args: { guardians: [config.aliceId] },
    });
  });

  it('should be able to pause the contract', async () => {
    assert.doesNotReject(async () => {
      await global.aliceContract.pause({ args: {} });
      assert.equal(await global.aliceContract.contract_status(), 'Paused');
    });

    await assert.rejects(async () => {
      await global.aliceContract.ft_transfer({
        args: { receiver_id: 'any', amount: '1' },
      });
    });
  });

  after(async () => {
    await global.usnContract.remove_guardians({
      args: { guardians: [config.aliceId] },
    });
  });
});

describe('User', async function () {
  this.timeout(15000);

  it('should NOT sell before buying', async () => {
    await assert.rejects(async () => {
      await global.aliceContract.sell({ args: { amount: 1 } });
    });
  });

  it('should buy USN to get registered', async () => {
    const amount = await global.aliceContract.buy({
      args: {},
      amount: ONE_NEAR,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '11088180500000000000'); // no storage fee

    const expected_amount = await global.aliceContract.ft_balance_of({
      account_id: config.aliceId,
    });
    assert.equal(amount, expected_amount);
  });

  it('can buy USN with the expected rate', async () => {
    const amount = await global.aliceContract.buy({
      args: {
        expected: { multiplier: '111439', slippage: '10', decimals: 28 },
      },
      amount: ONE_NEAR,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '11088180500000000000');
  });

  it('should NOT register the recipient having not enough money to buy USN', async () => {
    await assert.rejects(
      async () => {
        await global.aliceContract.buy({
          args: {
            expected: { multiplier: '111439', slippage: '10', decimals: 28 },
            to: config.bobId,
          },
          amount: ONE_YOCTO, // very small attached deposit
          gas: GAS_FOR_CALL,
        });
      },
      (err) => {
        assert(err.message.includes('attached deposit exchanges to 0 tokens'));
        return true;
      }
    );
  });

  it('can buy USN for unregistered user (the recipient gets auto-registered)', async () => {
    const amount = await global.aliceContract.buy({
      args: {
        to: config.bobId,
      },
      amount: ONE_NEAR,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '11088180500000000000'); // no storage fee

    const expected_amount = await global.bobContract.ft_balance_of({
      account_id: config.bobId,
    });
    assert.equal(amount, expected_amount);
  });

  it('can NOT buy with slippage control in place', async () => {
    await assert.rejects(
      async () => {
        await global.aliceContract.buy({
          args: {
            expected: { multiplier: '111428', slippage: '10', decimals: 28 },
          },
          amount: ONE_NEAR,
          gas: GAS_FOR_CALL,
        });
      },
      (err) => {
        assert(err.message.includes('Slippage error'));
        return true;
      }
    );
  });

  it('sells USN with the current exchange rate', async () => {
    const near = await global.aliceContract.sell({
      args: {
        amount: '11032461000000000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '985050000000000000000000'); // 0.98 NEAR
  });

  it('sells USN with slippage control', async () => {
    const near = await global.bobContract.sell({
      args: {
        amount: '11032461000000000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '985050000000000000000000'); // 0.97 NEAR
  });

  it('spends gas and gets the rest back in case of error', async () => {
    const balance = (await global.aliceContract.account.getAccountBalance())
      .available;
    await assert.rejects(async () => {
      await global.aliceContract.buy({
        args: {
          expected: { multiplier: '111428', slippage: '10', decimals: 28 },
        },
        amount: ONE_NEAR,
        gas: GAS_FOR_CALL,
      });
    });
    const balance2 = (await global.aliceContract.account.getAccountBalance())
      .available;
    assert.equal(balance.length, balance2.length);
    // 9.99 NEAR -> 9.97 NEAR
    // 5.71 NEAR -> 5.68 NEAR
    const near_before = parseInt(balance.substring(0, 3));
    const near_after = parseInt(balance2.substring(0, 3));
    // Should be less than 3-4, but it's 6 (0.06, ~$0.6) because of the sandbox issue.
    assert(near_before - near_after < 6);
  });

  it('should sell all USN to get unregistered', async () => {
    await global.aliceContract.sell({
      args: {
        amount: await global.aliceContract.ft_balance_of({
          account_id: config.aliceId,
        }),
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });

    assert.equal(
      await global.aliceContract.ft_balance_of({
        account_id: config.aliceId,
      }),
      '0'
    );

    await assert.rejects(
      async () => {
        await global.aliceContract.ft_transfer({
          args: { receiver_id: 'any', amount: '1' },
          amount: ONE_YOCTO,
        });
      },
      (err) => {
        assert.match(err.message, /The account doesn't have enough balance/);
        return true;
      }
    );

    await global.bobContract.sell({
      args: {
        amount: await global.bobContract.ft_balance_of({
          account_id: config.bobId,
        }),
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
  });

  after(async () => {
    const aliceBalance = await global.aliceContract.ft_balance_of({
      account_id: config.aliceId,
    });

    const bobBalance = await global.bobContract.ft_balance_of({
      account_id: config.bobId,
    });

    // Flush balances and force registration removal.

    if (aliceBalance != '0') {
      await global.aliceContract.ft_transfer({
        args: {
          receiver_id: 'any',
          amount: aliceBalance,
        },
        amount: ONE_YOCTO,
      });
    }

    if (bobBalance != '0') {
      await global.bobContract.ft_transfer({
        args: {
          receiver_id: 'any',
          amount: bobBalance,
        },
        amount: ONE_YOCTO,
      });
    }
  });
});

describe('Adaptive Spread', async function () {
  this.timeout(15000);

  it('should be used to buy USN', async () => {
    const amount = await global.aliceContract.buy({
      args: {},
      amount: HUNDRED_NEARS,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '1108854824870000000000'); // ~$1108

    const expected_amount = await global.aliceContract.ft_balance_of({
      account_id: config.aliceId,
    });
    assert.equal(amount, expected_amount);
  });

  it('should be used to sell USN', async () => {
    const near = await global.aliceContract.sell({
      args: {
        amount: '1108854824870000000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '99009067108900000000000000'); // 0.99 NEAR
  });

  it('should be configurable', async () => {
    await global.usnContract.set_adaptive_spread({
      args: { params: { min: 0.002, max: 0.006, scaler: 0.0001 } },
    });

    const amount = await global.aliceContract.buy({
      args: {},
      amount: ONE_NEAR,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '11077081175600000000'); // ~$11.08
  });

  it('should be in limits', async () => {
    // min <= max
    await assert.rejects(async () => {
      await global.usnContract.set_adaptive_spread({
        args: { params: { min: 0.006, max: 0.002, scaler: 0.0001 } },
      });
    });

    // min < 0.05
    await assert.rejects(async () => {
      await global.usnContract.set_adaptive_spread({
        args: { params: { min: 0.06, max: 0.01, scaler: 0.0001 } },
      });
    });

    // max < 0.05
    await assert.rejects(async () => {
      await global.usnContract.set_adaptive_spread({
        args: { params: { min: 0.01, max: 0.06, scaler: 0.0001 } },
      });
    });

    // scaler < 0.4
    await assert.rejects(async () => {
      await global.usnContract.set_adaptive_spread({
        args: { params: { min: 0.01, max: 0.03, scaler: 0.5 } },
      });
    });

    // only positive
    await assert.rejects(async () => {
      await global.usnContract.set_adaptive_spread({
        args: { params: { min: 0.001, max: 0.003, scaler: -0.4 } },
      });
    });
  });
});

describe('Fixed Spread', async function () {
  this.timeout(15000);

  before(async () => {
    await global.usnContract.set_fixed_spread({ args: { spread: '10000' } }); // 1%
  });

  it('should be used to buy USN', async () => {
    const amount = await global.aliceContract.buy({
      args: {},
      amount: HUNDRED_NEARS,
      gas: GAS_FOR_CALL,
    });
    assert.equal(amount, '1103246100000000000000'); // ~$1103
  });

  it('should be used to sell USN', async () => {
    const near = await global.aliceContract.sell({
      args: {
        amount: '1103246100000000000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '98010000000000000000000000'); // 98.01 NEAR
  });

  after(async () => {
    await global.usnContract.set_adaptive_spread({ args: {} });
  });
});

describe('Stable Pool (USDT/USN)', async function () {
  this.timeout(10000);

  it('should be initialized with a single call', async () => {
    await assert.doesNotReject(async () => {
      await global.usnContract.transfer_stable_liquidity({
        args: { whole_amount: '1000000' },
        amount: "780000000000000000001",
        gas: GAS_FOR_CALL,
      });
    });
  });
});
