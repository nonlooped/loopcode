import assert from "node:assert/strict";
import test from "node:test";

import { openExternalLinkFromClick } from "../src/utils/external-links.ts";

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

void test("opens a link when the click target is a text node", async () => {
  const click = clickOn("https://example.com/docs", {
    target: { nodeType: 3 },
    composedPath: () => [
      { nodeType: 3 },
      { closest: () => ({ href: "https://example.com/docs" }) },
    ],
  });
  const opened = [];

  await openExternalLinkFromClick(click.event, async (url) => opened.push(url));

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(opened, ["https://example.com/docs"]);
});

void test("leaves non-web links to their default behavior", async () => {
  const opened = [];
  const open = async (url) => opened.push(url);
  const relative = clickOn("/local/path");

  await openExternalLinkFromClick(relative.event, open);

  assert.equal(relative.wasPrevented(), false);
  assert.deepEqual(opened, []);
});
