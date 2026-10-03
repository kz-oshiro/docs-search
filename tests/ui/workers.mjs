import { availableParallelism, cpus } from 'node:os';
import { performance } from 'node:perf_hooks';
import { setTimeout as delay } from 'node:timers/promises';

export const workerPolicy = Object.freeze({
  maxWorkers: 6,
  cpuTarget: 0.8,
  cpuShare: 0.5,
  sampleIntervalMs: 200,
  sampleCount: 3,
});

// CPU times are cumulative milliseconds, including idle time, on Windows too.
// Capacity comes from availableParallelism(), rather than cpus().length.
export function cpuUsageBetween(before, after) {
  if (!Array.isArray(before) || before.length === 0
      || !Array.isArray(after) || before.length !== after.length) return null;
  let total = 0;
  let idle = 0;
  for (let index = 0; index < before.length; index++) {
    for (const key of ['user', 'nice', 'sys', 'idle', 'irq']) {
      const first = before[index]?.times?.[key];
      const last = after[index]?.times?.[key];
      if (!Number.isFinite(first) || first < 0 || !Number.isFinite(last) || last < first) return null;
      const elapsed = last - first;
      total += elapsed;
      if (key === 'idle') idle += elapsed;
    }
  }
  return Number.isFinite(total) && total > 0 ? (total - idle) / total : null;
}

export function selectWorkerCount(availableCpuCount, cpuUsage) {
  if (!Number.isInteger(availableCpuCount) || availableCpuCount < 1
      || !Number.isFinite(cpuUsage) || cpuUsage < 0 || cpuUsage > 1) return 1;
  const shareLimit = Math.max(1, Math.floor(availableCpuCount * workerPolicy.cpuShare));
  const idleBudget = Math.floor(availableCpuCount * Math.max(0, workerPolicy.cpuTarget - cpuUsage));
  return Math.max(1, Math.min(workerPolicy.maxWorkers, shareLimit, idleBudget));
}

export async function measureWorkerPlan({
  readCpus = cpus,
  readParallelism = availableParallelism,
  wait = delay,
} = {}) {
  const started = performance.now();
  const plan = {
    schemaVersion: 1,
    measuredAt: new Date().toISOString(),
    availableCpuCount: null,
    logicalCpuCount: null,
    policy: workerPolicy,
    cpuUsageSamples: [],
    averageCpuUsage: null,
    peakCpuUsage: null,
    durationMs: 0,
    workers: 1,
    fallbackReason: null,
  };
  try {
    plan.availableCpuCount = readParallelism();
    if (!Number.isInteger(plan.availableCpuCount) || plan.availableCpuCount < 1) {
      throw new Error('Available CPU capacity could not be measured');
    }
    let before = readCpus();
    plan.logicalCpuCount = Array.isArray(before) ? before.length : null;
    for (let sample = 0; sample < workerPolicy.sampleCount; sample++) {
      await wait(workerPolicy.sampleIntervalMs);
      const after = readCpus();
      const usage = cpuUsageBetween(before, after);
      if (usage === null) throw new Error('CPU time counters are unavailable or changed during sampling');
      plan.cpuUsageSamples.push(usage);
      before = after;
    }
    plan.averageCpuUsage = plan.cpuUsageSamples.reduce((total, usage) => total + usage, 0)
      / plan.cpuUsageSamples.length;
    plan.peakCpuUsage = Math.max(...plan.cpuUsageSamples);
    plan.workers = selectWorkerCount(plan.availableCpuCount, plan.peakCpuUsage);
  } catch (error) {
    plan.fallbackReason = error instanceof Error ? error.message : String(error);
  }
  plan.durationMs = performance.now() - started;
  return plan;
}

export function validateWorkerPlan(plan) {
  if (plan?.schemaVersion !== 1 || !Number.isInteger(plan.workers)
      || plan.workers !== selectWorkerCount(plan.availableCpuCount, plan.peakCpuUsage)) {
    throw new Error('Invalid UI worker plan; run cargo xtask ui to measure CPU usage again');
  }
  return plan.workers;
}
