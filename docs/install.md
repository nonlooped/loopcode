# Installing LoopCode (unsigned builds)

LoopCode v1 ships **unsigned** reproducible desktop bundles. OS trust prompts are expected; they are not a product defect. Prefer package managers when available.

## Windows (SmartScreen)

1. Download the MSI or NSIS installer from Releases (or build with `npm run tauri build`).
2. If **Windows protected your PC** (SmartScreen) appears:
   - Click **More info**
   - Click **Run anyway**
3. Optional channels: install via **winget** / Scoop when community packages exist (avoids some SmartScreen friction).
4. First launch may prompt for network access only when you connect a provider (keys stay in Core secret store).

## macOS (Gatekeeper)

1. Install via Homebrew cask when published, or open the `.dmg` / app from Releases.
2. If Gatekeeper blocks an unsigned app:
   - **Right-click** (or Control-click) the app → **Open**
   - Confirm **Open** in the dialog
3. Or remove quarantine after intentional download (advanced):

   ```bash
   xattr -dr com.apple.quarantine /Applications/LoopCode.app
   ```

4. Prefer signed Homebrew/cask packaging when the project publishes it.

## Linux (AppImage / distro packages)

1. **AppImage:** mark executable and run:

   ```bash
   chmod +x LoopCode-*.AppImage
   ./LoopCode-*.AppImage
   ```

2. Prefer **Flatpak**, distro packages, or your package manager when available.
3. Wayland/X11: use the platform WebView (WebKitGTK) packages your distro provides for Tauri.

## After install

- Open a local project folder from the cockpit (**Open folder…**).
- Connect a provider under **Providers** (onboarding).
- Accessibility: use **Theme → High contrast**, keyboard map (Ctrl+K palette), F6 focus regions, Esc to dismiss overlays.
- Updates: check is **off by default**; enable only under Advanced → Reliability.

## Build from source

```bash
npm install
npm run tauri build
```

See the root [README.md](../README.md) for prerequisites (Node, Rust, platform WebView, MSVC on Windows).
