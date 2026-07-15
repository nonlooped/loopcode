import {
  useCallback,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { cn } from "../../design";
import {
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
} from "../../store/ui";

type SidebarSide = "left" | "right";

/**
 * Drag handle on the inner edge of a sidebar.
 * left: drag right widens; right: drag left widens.
 */
export function SidebarResizeHandle({
  side,
  width,
  onWidthChange,
  label,
}: {
  side: SidebarSide;
  width: number;
  onWidthChange: (width: number) => void;
  label: string;
}) {
  const dragging = useRef(false);
  const startX = useRef(0);
  const startWidth = useRef(0);
  const [active, setActive] = useState(false);

  const endDrag = useCallback((target: HTMLElement, pointerId: number) => {
    if (!dragging.current) return;
    dragging.current = false;
    setActive(false);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    try {
      target.releasePointerCapture(pointerId);
    } catch {
      /* already released */
    }
  }, []);

  const onPointerDown = useCallback(
    (e: PointerEvent<HTMLDivElement>) => {
      if (e.button !== 0) return;
      e.preventDefault();
      dragging.current = true;
      startX.current = e.clientX;
      startWidth.current = width;
      setActive(true);
      e.currentTarget.setPointerCapture(e.pointerId);
      document.body.style.cursor = "col-resize";
      document.body.style.userSelect = "none";
    },
    [width],
  );

  const onPointerMove = useCallback(
    (e: PointerEvent<HTMLDivElement>) => {
      if (!dragging.current) return;
      const dx = e.clientX - startX.current;
      const next = side === "left" ? startWidth.current + dx : startWidth.current - dx;
      onWidthChange(next);
    },
    [onWidthChange, side],
  );

  const onPointerUp = useCallback(
    (e: PointerEvent<HTMLDivElement>) => {
      endDrag(e.currentTarget, e.pointerId);
    },
    [endDrag],
  );

  const onKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      const step = e.shiftKey ? 24 : 8;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        onWidthChange(side === "left" ? width - step : width + step);
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        onWidthChange(side === "left" ? width + step : width - step);
      } else if (e.key === "Home") {
        e.preventDefault();
        onWidthChange(SIDEBAR_MIN_WIDTH);
      } else if (e.key === "End") {
        e.preventDefault();
        onWidthChange(SIDEBAR_MAX_WIDTH);
      }
    },
    [onWidthChange, side, width],
  );

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={label}
      aria-valuenow={width}
      aria-valuemin={SIDEBAR_MIN_WIDTH}
      aria-valuemax={SIDEBAR_MAX_WIDTH}
      tabIndex={0}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
      className={cn(
        "group/handle absolute inset-y-0 z-10 w-3 touch-none cursor-col-resize outline-none",
        "focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-accent",
        side === "left" ? "right-0 translate-x-1/2" : "left-0 -translate-x-1/2",
      )}
    >
      {/* Visual line: lights up on hover / drag / keyboard focus. */}
      <span
        aria-hidden
        className={cn(
          "pointer-events-none absolute inset-y-0 left-1/2 w-px -translate-x-1/2 bg-transparent",
          "transition-[background-color,box-shadow] duration-[var(--duration-press)] ease-[var(--ease-out)]",
          "group-hover/handle:bg-accent/45 group-focus-visible/handle:bg-accent",
          active && "bg-accent shadow-[0_0_0_1px_color-mix(in_srgb,var(--accent)_35%,transparent)]",
        )}
      />
    </div>
  );
}
