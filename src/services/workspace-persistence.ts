import { saveWorkspace } from "./native.ts";
import type { PersistedWorkspace } from "../types/index.ts";

export class WorkspacePersistence {
  #ready = false;
  #timer?: ReturnType<typeof setTimeout>;
  #pending?: PersistedWorkspace;
  #save?: Promise<void>;

  setReady() {
    this.#ready = true;
  }

  queue(workspace: PersistedWorkspace) {
    this.#pending = workspace;
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => {
      this.#timer = undefined;
      void this.flush().catch((cause: unknown) => {
        console.error("Could not persist LoopCode threads", cause);
      });
    }, 200);
  }

  async flush() {
    if (!this.#ready) return;
    if (this.#timer) {
      clearTimeout(this.#timer);
      this.#timer = undefined;
    }
    if (this.#save) await this.#save;
    const workspace = this.#pending;
    if (!workspace) return;
    this.#pending = undefined;
    this.#save = saveWorkspace(workspace);
    try {
      await this.#save;
    } catch (error) {
      this.#pending ??= workspace;
      throw error;
    } finally {
      this.#save = undefined;
    }
    if (this.#pending) await this.flush();
  }
}
