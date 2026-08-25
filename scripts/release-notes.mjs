#!/usr/bin/env node
import { readFileSync } from "node:fs";

export function releaseNotes(changelog, tag) {
  const version = /^v(\d+\.\d+\.\d+)$/.exec(tag)?.[1];
  if (!version) throw new Error(`invalid release tag: ${tag}`);

  const section = changelog.trim().split(/\n(?=## \[)/)[0];
  if (!section.startsWith(`## [${version}] - `))
    throw new Error(`CHANGELOG.md does not start with ${tag}`);

  return `${section.trimEnd()}\n`;
}

if (import.meta.main) {
  try {
    const tag = process.argv[2];
    process.stdout.write(releaseNotes(readFileSync("CHANGELOG.md", "utf8"), tag));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
