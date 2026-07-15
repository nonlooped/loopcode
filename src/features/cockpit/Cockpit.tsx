import { useEffect } from "react";
import { TopBar } from "./TopBar";
import { LeftRail } from "./LeftRail";
import { Composer } from "./Composer";
import { SidebarCollapsedRail } from "./SidebarCollapsedRail";
import { SidebarResizeHandle } from "./SidebarResizeHandle";
import { Timeline } from "../timeline/Timeline";
import { RightPane } from "../surfaces/RightPane";
import { SettingsRail } from "../settings/SettingsRail";
import { SettingsView } from "../settings/SettingsView";
import { useProjects, useTimeline } from "../../ipc/hooks";
import { useKeyboardMap } from "../../hooks/useKeyboardMap";
import { useSession } from "../../store/session";
import { useUi } from "../../store/ui";
import type { TimelineItem } from "../../ipc/types";

/** Pending approval for the active run — shown on the composer, not the timeline. */
function pendingApprovalFor(
  projection: {
    activeRunId?: string | null;
    activeRunStatus?: string | null;
    items: TimelineItem[];
  } | null,
): TimelineItem | null {
  if (!projection || projection.activeRunStatus !== "approval_waiting") {
    return null;
  }
  const activeId = projection.activeRunId;
  return (
    projection.items.find(
      (i) =>
        i.needsApproval &&
        (!i.runId || !activeId || i.runId === activeId),
    ) ?? null
  );
}

/** Cockpit shell: top bar, left rail, center timeline + composer, right pane. */
export function Cockpit() {
  const { activeChatId, activeProjectId } = useSession();
  const setActiveRun = useSession((s) => s.setActiveRun);
  const setActiveProject = useSession((s) => s.setActiveProject);
  const leftCollapsed = useUi((s) => s.leftCollapsed);
  const rightCollapsed = useUi((s) => s.rightCollapsed);
  const leftWidth = useUi((s) => s.leftWidth);
  const rightWidth = useUi((s) => s.rightWidth);
  const setLeftWidth = useUi((s) => s.setLeftWidth);
  const setRightWidth = useUi((s) => s.setRightWidth);
  const settingsOpen = useUi((s) => s.settingsOpen);

  useKeyboardMap();
  const { data: projects = [] } = useProjects();
  const { data: projection = null } = useTimeline(activeChatId);

  // Keep the session's active run in sync with the Core projection so Stop/Esc work.
  useEffect(() => {
    setActiveRun(projection?.activeRunId ?? null);
  }, [projection?.activeRunId, setActiveRun]);

  // Returning users: open the first known project when none is selected yet.
  useEffect(() => {
    if (!activeProjectId && projects.length > 0) {
      setActiveProject(projects[0].id);
    }
  }, [activeProjectId, projects, setActiveProject]);

  // Panel collapse is keyboard-frequent (⌘B) — stay instant (no laggy chrome).
  // Settings mode swaps the left rail and center column, and hides the right pane.
  const showLeft = !leftCollapsed;
  const showRight = !rightCollapsed && !settingsOpen;
  const toggleLeft = useUi((s) => s.toggleLeft);
  const toggleRight = useUi((s) => s.toggleRight);

  return (
    <div className="flex h-screen flex-col bg-paper">
      <TopBar projection={projection} />

      <div className="flex min-h-0 flex-1">
        {showLeft ? (
          <div className="relative h-full shrink-0" style={{ width: leftWidth }}>
            {settingsOpen ? <SettingsRail /> : <LeftRail />}
            <SidebarResizeHandle
              side="left"
              width={leftWidth}
              onWidthChange={setLeftWidth}
              label="Resize left sidebar"
            />
          </div>
        ) : (
          !settingsOpen && (
            <SidebarCollapsedRail side="left" onExpand={toggleLeft} />
          )
        )}

        {settingsOpen ? (
          <SettingsView />
        ) : (
          <section
            id="region-timeline"
            tabIndex={-1}
            aria-label="Conversation"
            className="flex min-h-0 min-w-0 flex-1 flex-col bg-paper"
          >
            <div data-timeline-scroll className="min-h-0 flex-1 overflow-auto">
              <Timeline projection={projection} />
            </div>
            <Composer
              runStatus={projection?.activeRunStatus ?? null}
              pendingApproval={pendingApprovalFor(projection)}
            />
          </section>
        )}

        {showRight ? (
          <div
            id="region-surfaces"
            tabIndex={-1}
            aria-label="Project surfaces"
            className="relative h-full min-w-0 shrink-0 overflow-hidden"
            style={{ width: rightWidth }}
          >
            <SidebarResizeHandle
              side="right"
              width={rightWidth}
              onWidthChange={setRightWidth}
              label="Resize right sidebar"
            />
            <RightPane />
          </div>
        ) : (
          !settingsOpen && (
            <SidebarCollapsedRail side="right" onExpand={toggleRight} />
          )
        )}
      </div>
    </div>
  );
}
