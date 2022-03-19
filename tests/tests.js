'use strict';

const assert = require('assert').strict;
const config = require('./sandbox-setup').config;
const BN = require('bn.js');

const ONE_NEAR = '1000000000000000000000000';
const ONE_YOCTO = '1';
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
    const owner = await global.aliceContract.get_owner();
    assert.equal(owner, config.usnId);
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
    assert.equal(await global.usnContract.get_owner(), config.aliceId);
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
    assert.equal(await global.aliceContract.get_owner(), config.usnId);
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

  before(async () => {
    await global.usnContract.extend_guardians({
      args: { guardians: [config.aliceId, config.bobId] },
    });
  });

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

  // Issue: https://github.com/near/sandbox/issues/27
  it.skip('sells USN with the current exchange rate', async () => {
    const near = await global.aliceContract.sell({
      args: {
        amount: '11032461000000000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '980198019801980198019801'); // 0.98 NEAR
  });

  // Issue: https://github.com/near/sandbox/issues/27
  it.skip('sells USN with slippage control', async () => {
    const near = await global.bobContract.sell({
      args: {
        amount: '11018670423750000000',
      },
      amount: ONE_YOCTO,
      gas: GAS_FOR_CALL,
    });
    assert.equal(near, '978972772277227722772277'); // 0.97 NEAR
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

  it.skip('should sell all USN to get unregistered', async () => {
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
        assert(err.message.includes('is not registered'));
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
    await global.usnContract.remove_guardians({
      args: { guardians: [config.aliceId, config.bobId] },
    });
  });
});
