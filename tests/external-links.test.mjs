import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { openExternalLinkFromClick } from "../src/utils/external-links.ts";

const markdownMessage = await readFile(
  new URL("../src/components/markdown/MarkdownMessage.svelte", import.meta.url),
  "utf8",
);

function clickOn(href, overrides = {}) {
  let prevented = false;
  const anchor = { href };
  return {
    event: {
      button: 0,
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      altKey: false,
      target: { closest: (selector) => (selector === "a[href]" ? anchor : null) },
      preventDefault: () => {
        prevented = true;
      },
      ...overrides,
    },
    wasPrevented: () => prevented,
  };
}

void test("opens a clicked markdown HTTP link outside the webview", async () => {
  const click = clickOn("https://example.com/docs");
  const opened = [];

  await openExternalLinkFromClick(click.event, async (url) => opened.push(url));

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(opened, ["https://example.com/docs"]);
});

void test("routes markdown clicks through the external link handler", () => {
  assert.match(markdownMessage, /import \{ openUrl \} from '@tauri-apps\/plugin-opener'/);
  assert.match(markdownMessage, /<div role="presentation" onclick=\{handleClick\}>/);
  assert.match(markdownMessage, /openExternalLinkFromClick\(event, openUrl\)/);
});

void test("leaves non-web links to their default behavior", async () => {
  const opened = [];
  const open = async (url) => opened.push(url);
  const relative = clickOn("/local/path");

  await openExternalLinkFromClick(relative.event, open);

  assert.equal(relative.wasPrevented(), false);
  assert.deepEqual(opened, []);
});
