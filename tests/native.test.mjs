import assert from "node:assert/strict";
import test from "node:test";

void test("native project data is validated once at the IPC boundary", async () => {
  const callbacks = new Map();
  let nextCallbackId = 1;
  globalThis.window = {
    __TAURI_INTERNALS__: {
      transformCallback(callback) {
        const id = nextCallbackId++;
        callbacks.set(id, callback);
        return id;
      },
      unregisterCallback(id) {
        callbacks.delete(id);
      },
      async invoke(command, args) {
        if (command === "list_composer_completions") {
          if (args.projectRoot !== "/workspace") {
            return [
              { kind: "file", name: "main.ts", path: "/valid/main.ts", relativePath: "main.ts" },
            ];
          }
          return [{ kind: "file", name: "bad", path: 7, relativePath: "bad" }];
        }
        if (command === "read_project_directory") {
          return [{ name: "src", path: "/workspace/src", isDirectory: "yes", isSymlink: false }];
        }
        if (command === "start_project_file_watcher") {
          args.onChange.onmessage({ paths: [7] });
          args.onChange.onmessage({ paths: ["/workspace/src/main.ts"] });
          return 4;
        }
        throw new Error(`Unexpected command: ${command}`);
      },
    },
  };

  const { listComposerCompletions, readProjectDirectory, startProjectFileWatcher } =
    await import("../src/services/native.ts");

  await assert.rejects(listComposerCompletions("/workspace"));
  assert.deepEqual(await listComposerCompletions("/valid"), [
    { kind: "file", name: "main.ts", path: "/valid/main.ts", relativePath: "main.ts" },
  ]);
  await assert.rejects(readProjectDirectory("/workspace", "/workspace"));
  const changes = [];
  assert.equal(await startProjectFileWatcher("/workspace", (change) => changes.push(change)), 4);
  assert.deepEqual(changes, [{ paths: ["/workspace/src/main.ts"] }]);
});
