import { useEffect } from "react";

/**
 * Suppress the WebView2 / OS-native right-click menu app-wide.
 *
 * The default menu ("Back / Forward / Reload / Inspect") is browser chrome that
 * makes a desktop app feel like a web page — off-brand for the cockpit, and its
 * items are inert or misleading here. We swallow the event at the window in the
 * bubble phase, so any custom Base UI context menu on an inner element still
 * runs first (and opens itself); only surfaces without their own menu fall
 * through to this listener and get nothing instead of the native menu.
 */
export function useSuppressNativeContextMenu() {
	useEffect(() => {
		const onContextMenu = (event: MouseEvent) => {
			// A custom menu marks the event handled by calling preventDefault();
			// leave those alone. Everything else: no native menu.
			if (!event.defaultPrevented) event.preventDefault();
		};
		window.addEventListener("contextmenu", onContextMenu);
		return () => window.removeEventListener("contextmenu", onContextMenu);
	}, []);
}
