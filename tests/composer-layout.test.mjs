import assert from "node:assert/strict";
import test from "node:test";

import { composerLayoutKeyframes } from "../src/utils/composer-layout.ts";

void test("keeps composer keyframes finite while the current box is collapsed", () => {
  const [first] = composerLayoutKeyframes(
    { left: 10, top: 20, width: 300, height: 80 },
    { left: 10, top: 20, width: 0, height: 0 },
  );

  assert.equal(first.transform, "translate(0px, 0px) scale(300, 80)");
});
