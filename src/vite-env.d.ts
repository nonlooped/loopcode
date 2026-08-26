/// <reference types="vite/client" />

declare module "virtual:material-icon-manifest" {
  import type { MaterialIconMaps } from "./utils/material-icon-lookup";

  const manifest: Required<MaterialIconMaps>;
  export default manifest;
}
