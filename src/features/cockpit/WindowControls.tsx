import { useEffect, useState } from "react";
import { IconMinus, IconSquare, IconSquares, IconX } from "@tabler/icons-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { cn } from "../../design";
import { isTauri } from "../../ipc/client";

const winBtn =
  "inline-flex h-full w-11 shrink-0 items-center justify-center p-0 rounded-none text-ink opacity-100 outline-none " +
  "transition-[background-color,color] duration-[var(--duration-press)] ease-[var(--ease-out)] " +
  "focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent " +
  "fine-hover:bg-line/70 fine-hover:text-ink " +
  "active:bg-line";

const winBtnClose =
  "fine-hover:bg-clay fine-hover:text-clay-foreground active:bg-clay active:text-clay-foreground";

/** Native minimize / maximize / close chrome for frameless Tauri windows. */
export function WindowControls({ className }: { className?: string }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;

    const win = getCurrentWindow();
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void win.isMaximized().then((value) => {
      if (!disposed) setMaximized(value);
    });

    void win
      .onResized(() => {
        void win.isMaximized().then((value) => {
          if (!disposed) setMaximized(value);
        });
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  async function run(action: "minimize" | "toggleMaximize" | "close") {
    if (!isTauri()) return;
    const win = getCurrentWindow();
    if (action === "minimize") await win.minimize();
    else if (action === "toggleMaximize") await win.toggleMaximize();
    else await win.close();
  }

  return (
    <div
      role="group"
      aria-label="Window"
      className={cn("flex h-full shrink-0 items-stretch", className)}
    >
      <button
        type="button"
        aria-label="Minimize"
        className={winBtn}
        onClick={() => void run("minimize")}
      >
        <IconMinus size={14} stroke={1.75} className="text-ink" aria-hidden />
      </button>
      <button
        type="button"
        aria-label={maximized ? "Restore" : "Maximize"}
        className={winBtn}
        onClick={() => void run("toggleMaximize")}
      >
        {maximized ? (
          <IconSquares size={13} stroke={1.75} className="text-ink" aria-hidden />
        ) : (
          <IconSquare size={12} stroke={1.75} className="text-ink" aria-hidden />
        )}
      </button>
      <button
        type="button"
        aria-label="Close"
        className={cn(winBtn, winBtnClose)}
        onClick={() => void run("close")}
      >
        <IconX size={15} stroke={1.75} className="text-ink" aria-hidden />
      </button>
    </div>
  );
}
