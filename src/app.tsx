import { useEffect, useRef } from "react";
import { OnboardingFlow } from "./features/onboarding/OnboardingFlow";
import { Cockpit } from "./features/cockpit/Cockpit";
import { applyTheme } from "./ipc/commands";
import { isTauri } from "./ipc/client";
import { useReadyProvider } from "./ipc/hooks";
import { useSuppressNativeContextMenu } from "./hooks/useSuppressNativeContextMenu";
import { useUi } from "./store/ui";
import { useSession } from "./store/session";

/** Top-level router: reflect theme, then show Onboarding or Cockpit. */
export function App() {
  const theme = useUi((s) => s.theme);
  const view = useSession((s) => s.view);
  const enterCockpit = useSession((s) => s.enterCockpit);
  const { data: readyProvider } = useReadyProvider();
  const autoEntered = useRef(false);

  // Replace the WebView2 default right-click menu with our own surface menus.
  useSuppressNativeContextMenu();

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    if (isTauri()) {
      void applyTheme(theme).catch(() => {
        /* Core a11y theme is best-effort */
      });
    }
  }, [theme]);

  // Returning users with a ready provider skip straight to the cockpit (once).
  useEffect(() => {
    if (!autoEntered.current && readyProvider) {
      autoEntered.current = true;
      enterCockpit();
    }
  }, [readyProvider, enterCockpit]);

  return view === "cockpit" ? <Cockpit /> : <OnboardingFlow />;
}
