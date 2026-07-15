import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("./client", () => ({ invoke }));

import {
	backupDb,
	clipboardReadText,
	clipboardWriteText,
	connectProvider,
} from "./commands";

describe("Core IPC wrappers", () => {
	beforeEach(() => invoke.mockReset());

	it("routes provider connections to Core without a fixture-success override", () => {
		connectProvider("openai", "key");
		expect(invoke).toHaveBeenCalledWith("onboarding_connect_provider", {
			providerId: "openai",
			apiKey: "key",
		});
	});

	it("routes clipboard access through Core", () => {
		clipboardWriteText("copied");
		clipboardReadText();
		expect(invoke).toHaveBeenNthCalledWith(1, "clipboard_write_text", {
			text: "copied",
		});
		expect(invoke).toHaveBeenNthCalledWith(2, "clipboard_read_text");
	});

	it("does not expose a caller-selected backup path", () => {
		backupDb();
		expect(invoke).toHaveBeenCalledWith("reliability_backup_db");
	});
});
