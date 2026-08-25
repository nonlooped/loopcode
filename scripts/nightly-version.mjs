#!/usr/bin/env node

export function nightlyVersion(base, date, runNumber) {
  if (!/^\d+\.\d+\.\d+$/.test(base)) throw new Error(`invalid base version: ${base}`);
  if (!/^\d{8}$/.test(date)) throw new Error(`invalid nightly date: ${date}`);
  if (!/^[1-9]\d*$/.test(String(runNumber))) throw new Error(`invalid run number: ${runNumber}`);
  return `${base}-nightly.${date}.${runNumber}`;
}

if (import.meta.main) {
  try {
    process.stdout.write(`${nightlyVersion(process.argv[2], process.argv[3], process.argv[4])}\n`);
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
