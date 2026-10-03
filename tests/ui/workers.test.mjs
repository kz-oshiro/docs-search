import assert from 'node:assert/strict';
import { test } from 'node:test';
import { cpuUsageBetween, measureWorkerPlan, selectWorkerCount, validateWorkerPlan } from './workers.mjs';

const cpu = (user, idle, sys = 0) => ({ times: { user, nice: 0, sys, idle, irq: 0 } });

test('CPU usage uses counter differences and includes every logical CPU', () => {
  assert.equal(cpuUsageBetween(
    [cpu(100, 900), cpu(100, 900)],
    [cpu(150, 950), cpu(125, 975)],
  ), 0.375);
  assert.equal(cpuUsageBetween([cpu(100, 900)], [cpu(100, 1000)]), 0);
  assert.equal(cpuUsageBetween([cpu(100, 900)], [cpu(200, 900)]), 1);
});

test('Missing, reset, unchanged or inconsistent CPU counters do not imply idle CPUs', () => {
  for (const [before, after] of [
    [[], []],
    [undefined, []],
    [[cpu(100, 900)], []],
    [[cpu(100, 900)], [cpu(90, 1000)]],
    [[cpu(100, 900)], [cpu(100, 900)]],
    [[cpu(100, 900)], [{ times: { user: 200, idle: 1000 } }]],
    [[cpu(100, 900)], [cpu(NaN, 1000)]],
    [[cpu(0, 0)], [cpu(Number.MAX_VALUE, Number.MAX_VALUE)]],
  ]) assert.equal(cpuUsageBetween(before, after), null);
});

test('Workers decrease with load and leave capacity for the concurrent backend', () => {
  assert.equal(selectWorkerCount(8, 0), 4);
  assert.equal(selectWorkerCount(8, 0.5), 2);
  assert.equal(selectWorkerCount(8, 0.75), 1);
  assert.equal(selectWorkerCount(8, 1), 1);
  assert.equal(selectWorkerCount(32, 0), 6);
  assert.equal(selectWorkerCount(1, 0), 1);
  assert.equal(selectWorkerCount(2, 0), 1);
  assert.equal(selectWorkerCount(4, 0), 2);
});

test('Unknown CPU load or capacity falls back to one worker', () => {
  for (const count of [0, -1, 1.5, null, NaN]) assert.equal(selectWorkerCount(count, 0), 1);
  for (const usage of [null, undefined, NaN, -0.1, 1.1]) assert.equal(selectWorkerCount(8, usage), 1);
});

test('A transient busy interval limits workers even when the average load is low', async () => {
  const snapshots = [[cpu(0, 0)], [cpu(0, 100)], [cpu(90, 110)], [cpu(90, 210)]];
  const waits = [];
  const plan = await measureWorkerPlan({
    readCpus: () => snapshots.shift(),
    readParallelism: () => 8,
    wait: async ms => { waits.push(ms); },
  });
  assert.deepEqual(waits, [200, 200, 200]);
  assert.deepEqual(plan.cpuUsageSamples, [0, 0.9, 0]);
  assert.equal(plan.peakCpuUsage, 0.9);
  assert.ok(Math.abs(plan.averageCpuUsage - 0.3) < 1e-12);
  assert.equal(plan.workers, 1);
  assert.equal(plan.fallbackReason, null);
  assert.equal(validateWorkerPlan(plan), 1);
});

test('Sampling failure records its reason and selects one worker', async () => {
  for (const snapshots of [
    [[], []],
    [[cpu(0, 0)], [cpu(0, 100)], [cpu(0, 90)]],
  ]) {
    const plan = await measureWorkerPlan({
      readCpus: () => snapshots.shift(), readParallelism: () => 8, wait: async () => {},
    });
    assert.equal(plan.workers, 1);
    assert.equal(plan.peakCpuUsage, null);
    assert.match(plan.fallbackReason, /CPU time counters/);
    assert.equal(validateWorkerPlan(plan), 1);
  }
});

test('OS capacity lookup failure is also a recorded fallback', async () => {
  const plan = await measureWorkerPlan({ readParallelism: () => { throw new Error('capacity lookup failed'); } });
  assert.equal(plan.workers, 1);
  assert.equal(plan.availableCpuCount, null);
  assert.equal(plan.fallbackReason, 'capacity lookup failed');
  assert.equal(validateWorkerPlan(plan), 1);
});

test('A changed worker count or incompatible plan cannot override the CPU policy', () => {
  const plan = { schemaVersion: 1, availableCpuCount: 8, peakCpuUsage: 0, workers: 4 };
  assert.equal(validateWorkerPlan(plan), 4);
  for (const invalid of [undefined, {}, { ...plan, schemaVersion: 2 }, { ...plan, workers: 6 }, { ...plan, workers: '4' }]) {
    assert.throws(() => validateWorkerPlan(invalid), /Invalid UI worker plan/);
  }
});
