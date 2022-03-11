'use strict';

const { globalAgent } = require('http');

const assert = require('assert').strict;
const config = require('./sandbox-setup').config;

describe('Anyone', function () {
  it('should get a version (smoke test)', async () => {
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
    assert.equal(spread, 10000);
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
    assert.rejects(async () => {
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

    assert.rejects(async () => {
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
