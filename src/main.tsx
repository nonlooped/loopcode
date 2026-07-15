import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Tooltip } from "@base-ui/react/tooltip";
import { App } from "./app";
import { useUi } from "./store/ui";
import "./styles/theme.css";

// Seed theme from the OS preference before first paint; the in-app switch takes over after.
const prefersDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
const initialTheme = prefersDark ? "dark" : "light";
useUi.getState().setTheme(initialTheme);
document.documentElement.setAttribute("data-theme", initialTheme);

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false, refetchOnWindowFocus: false, staleTime: 5_000 },
  },
});

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");

createRoot(root).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <Tooltip.Provider>
        <App />
      </Tooltip.Provider>
    </QueryClientProvider>
  </StrictMode>,
);
