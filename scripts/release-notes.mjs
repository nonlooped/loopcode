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

export function nightlyNotes({ tag, shortSha, previousTag, commits }) {
  if (!/^v\d+\.\d+\.\d+-nightly\.\d{8}\.\d+$/.test(tag))
    throw new Error(`invalid nightly tag: ${tag}`);
  const list =
    commits.length > 0 ? commits.map((commit) => `- ${commit}`).join("\n") : "- No new commits.";
  return `Nightly \`${tag}\` (${shortSha}). Not a stable release.\n\n## Changes since ${previousTag}\n\n${list}\n`;
}

if (import.meta.main) {
  try {
    const tag = process.argv[2];
    if (tag?.includes("-nightly.")) {
      const commits = readFileSync(0, "utf8").split(/\r?\n/).filter(Boolean);
      process.stdout.write(
        nightlyNotes({
          tag,
          shortSha: process.argv[3],
          previousTag: process.argv[4],
          commits,
        }),
      );
    } else {
      process.stdout.write(releaseNotes(readFileSync("CHANGELOG.md", "utf8"), tag));
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
