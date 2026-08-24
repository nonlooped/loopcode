import assert from "node:assert/strict";
import test from "node:test";

import { openMarkdownLinkFromClick } from "../src/utils/external-links.ts";

function clickOn(href, overrides = {}, resolvedHref = href) {
  let prevented = false;
  const anchor = {
    href: resolvedHref,
    getAttribute: (name) => (name === "href" ? href : null),
  };
  return {
    anchor,
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

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async (url) => opened.push(url),
  });

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(opened, ["https://example.com/docs"]);
});

void test("opens a link when the click target is a text node", async () => {
  const click = clickOn("https://example.com/docs", {
    target: { nodeType: 3 },
    composedPath: () => [{ nodeType: 3 }, { closest: () => click.anchor }],
  });
  const opened = [];

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async (url) => opened.push(url),
  });

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(opened, ["https://example.com/docs"]);
});

void test("opens relative file links in the project file viewer", async () => {
  const click = clickOn(
    "src/components/My%20File.svelte#L12",
    {},
    "http://localhost:1420/src/components/My%20File.svelte#L12",
  );
  const files = [];
  const urls = [];

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async (url) => urls.push(url),
    fileLinks: {
      projectRoot: "/workspace/project",
      open: (path) => files.push(path),
    },
  });

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(files, ["/workspace/project/src/components/My File.svelte"]);
  assert.deepEqual(urls, []);
});

void test("opens absolute file URLs in the project file viewer", async () => {
  const click = clickOn("file:///workspace/project/README.md#intro");
  const files = [];

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async () => {},
    fileLinks: {
      projectRoot: "/workspace/project",
      open: (path) => files.push(path),
    },
  });

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(files, ["/workspace/project/README.md"]);
});

void test("resolves relative file links against Windows project roots", async () => {
  const click = clickOn("src/App.svelte#L10");
  const files = [];

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async () => {},
    fileLinks: {
      projectRoot: "C:\\workspace\\project",
      open: (path) => files.push(path),
    },
  });

  assert.equal(click.wasPrevented(), true);
  assert.deepEqual(files, ["C:\\workspace\\project\\src\\App.svelte"]);
});

void test("leaves fragment links to their default behavior", async () => {
  const click = clickOn("#details", {}, "http://localhost:1420/#details");
  const files = [];
  const urls = [];

  await openMarkdownLinkFromClick(click.event, {
    openUrl: async (url) => urls.push(url),
    fileLinks: {
      projectRoot: "/workspace/project",
      open: (path) => files.push(path),
    },
  });

  assert.equal(click.wasPrevented(), false);
  assert.deepEqual(files, []);
  assert.deepEqual(urls, []);
});
