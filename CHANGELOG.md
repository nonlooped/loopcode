## [0.7.0] - 2026-08-24

### 🚀 Features

- _(ui)_ Improve workspace feedback (#66)

### 🐛 Bug Fixes

- _(providers)_ Preserve Claude brand icon (#65)

## [0.6.0] - 2026-08-24

### 🚀 Features

- _(transcript)_ Improve work and file review
- _(sidebar)_ Group threads by attention
- _(git)_ Add workspace controls and change review

### 🐛 Bug Fixes

- _(release)_ Grant provenance check access
- _(release)_ Serialize release publishing
- _(release)_ Require releases in version order
- _(provider)_ Protect lifecycle and availability state
- _(workspace)_ Reuse matching empty threads

## [0.5.0] - 2026-08-23

### 🚀 Features

- _(settings)_ Add visual setting previews

## [0.4.0] - 2026-08-23

### 🚀 Features

- _(git)_ Add branch and worktree commands
- _(composer)_ Add Git branch and worktree controls
- Add Git branch and worktree controls
- _(settings)_ Add color mode and theme preferences
- _(theme)_ Add light mode and color palettes
- Add light mode and color themes

### 🐛 Bug Fixes

- _(git)_ Preserve branch names when refs collide
- _(git)_ Protect established checkouts and worktrees

## [0.3.0] - 2026-08-23

### 🚀 Features

- _(settings)_ Reorganize preferences and controls

### 🐛 Bug Fixes

- _(release)_ List commits in release notes
- _(release)_ Link listed commits

## [0.2.0] - 2026-08-22

### 🚀 Features

- _(settings)_ Add persisted app preferences
- _(workspace)_ Configure empty thread reuse
- _(composer)_ Support configurable send shortcuts
- _(settings)_ Add categorized settings interface
- _(settings)_ Add categorized app preferences
- _(providers)_ Add configurable ACP providers
- _(dev)_ Add browser preview
- Add configurable ACP providers
- _(settings)_ Streamline preferences and add defaults

### 🐛 Bug Fixes

- _(ci)_ Compile Windows-specific code
- _(ci)_ Format metadata files
- _(settings)_ Honor transcript and thread defaults
- _(acp)_ Ignore events after stream shutdown
- _(providers)_ Keep unavailable providers manageable
- _(persistence)_ Retry transient save failures
- _(persistence)_ Serialize atomic workspace saves
- _(git)_ Bound branch lookup
- _(composer)_ Tolerate missing model catalog
- _(acp)_ Guard connection setup
- _(provider)_ Drop defunct connection after stop failure
- _(files)_ Reload viewer when project changes
- _(sessions)_ Keep fallback stream ids stable
- _(providers)_ Reconnect after argument changes
- Guard collapsed composer animation
- Preserve streamed message invariants
- Validate image clipboard support
- Open links clicked through text nodes
- _(native)_ Bound untrusted file and RPC data
- _(diagnostics)_ Move log writes off async tasks
- _(native)_ Share frontend generation state
- _(native)_ Force terminal shutdown after timeout
- _(files)_ Cap active project watchers
- _(providers)_ Await replaced process shutdown
- _(workspace)_ Report repaired provider ids
- _(release)_ Format generated changelog

## [0.1.0] - 2026-08-21

### 🚀 Features

- Restore private provider sessions
- Support per-model reasoning and Fast mode
- Improve desktop window interactions
- Support agent questions
- Strengthen ACP runtime diagnostics
- Add project explorer and desktop context menus
- _(ui)_ Add motion transitions with reduced motion support
- _(thread)_ Confirm thread deletion with dedicated dialog
- _(ui)_ Add file previews and empty thread welcome
- _(ui)_ Add webview zoom with indicator
- _(provider)_ Add Cursor ACP support
- _(provider)_ Add Cursor ACP support
- _(provider)_ Support rich agent questions
- _(composer)_ Add prompt references
- _(composer)_ Add prompt references
- _(ui)_ Refresh application icons
- Add Linux platform support
- Add Linux platform support
- Add integrated terminal drawer
- Add integrated terminal drawer
- _(linux)_ Add adjustable shell transparency
- Enhance terminal drawer with transition handling and layout readiness
- Integrate atomic file writing for persistence management
- Add tempfile dependency and refactor tests to utilize temporary directories
- _(ui)_ Adopt Bits UI primitives
- _(ui)_ Adopt Bits UI primitives

### 🐛 Bug Fixes

- Retain substantive timeline responses
- Scope streaming state to the active turn
- _(ui)_ Wrap long lines in file viewer
- _(test)_ Satisfy clippy path lint
- _(ui)_ Address design audit findings
- _(a11y)_ Restore project tree tab stop
- _(ui)_ Remove collapsed explorer strip
- _(ui)_ Show compact explorer toggle
- _(ui)_ Address design audit findings
- _(provider)_ Answer every Cursor question
- _(composer)_ Refresh file completions
- Preserve Windows command shims for npx and agent
- _(projects)_ Ignore file access events
- _(ci)_ Upgrade npm before cache access
- _(linux)_ Use opaque window shell
- Kill harness descendants on stop via process groups
- _(ui)_ Preserve picker and dialog focus
- _(release)_ Point cargo metadata at the src-tauri manifest
- _(release)_ Preserve file formatting when bumping versions
