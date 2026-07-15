# Enforces the deliberately narrow Tauri capability contract.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$capPath = Join-Path $root "src-tauri\capabilities\default.json"

if (-not (Test-Path $capPath)) {
  Write-Error "Missing capability file: $capPath"
  exit 1
}

$raw = Get-Content $capPath -Raw
$json = $raw | ConvertFrom-Json

$failures = [System.Collections.Generic.List[string]]::new()
$reports = [System.Collections.Generic.List[string]]::new()

$reports.Add("capability_file=$capPath")
$reports.Add("identifier=$($json.identifier)")

if (-not $json.permissions) {
  $failures.Add("capabilities/default.json has no permissions array")
} else {
  $perms = @($json.permissions | ForEach-Object { [string]$_ })
  $reports.Add("permissions=$($perms -join ',')")

  $allowedPermissions = @(
    'core:default',
    'dialog:allow-open',
    'core:window:allow-close',
    'core:window:allow-minimize',
    'core:window:allow-maximize',
    'core:window:allow-unmaximize',
    'core:window:allow-toggle-maximize',
    'core:window:allow-start-dragging'
  )
  foreach ($p in $perms) {
    if ($allowedPermissions -notcontains $p) {
      $failures.Add("Permission is not in the explicit allowlist: $p")
    }
  }
  foreach ($p in $allowedPermissions) {
    if ($perms -notcontains $p) {
      $failures.Add("Required narrow permission is missing: $p")
    }
  }

  # Explicitly require no opener plugin residual in Cargo.toml either
  $cargo = Get-Content (Join-Path $root "src-tauri\Cargo.toml") -Raw
  if ($cargo -match 'tauri-plugin-opener') {
    $failures.Add("src-tauri/Cargo.toml still depends on tauri-plugin-opener")
  } else {
    $reports.Add("opener_plugin=absent")
  }

  $pkg = Get-Content (Join-Path $root "package.json") -Raw
  if ($pkg -match '@tauri-apps/plugin-opener') {
    $failures.Add("package.json still depends on @tauri-apps/plugin-opener")
  } else {
    $reports.Add("opener_npm=absent")
  }

  $lib = Get-Content (Join-Path $root "src-tauri\src\lib.rs") -Raw
  if ($lib -match 'tauri_plugin_opener') {
    $failures.Add("lib.rs still initializes tauri_plugin_opener")
  } else {
    $reports.Add("opener_init=absent")
  }
}

# Window label must be main (matches capabilities.windows)
$confPath = Join-Path $root "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw | ConvertFrom-Json
$windows = @($conf.app.windows)
if ($windows.Count -lt 1) {
  $failures.Add("tauri.conf.json has no windows")
} else {
  $label = $windows[0].label
  $title = $windows[0].title
  $reports.Add("window_label=$label")
  $reports.Add("window_title=$title")
  if ($title -ne "LoopCode") {
    $failures.Add("Expected window title 'LoopCode', got '$title'")
  }
  $capWindows = @($json.windows)
  if ($capWindows -contains "*") {
    $failures.Add("Capability windows must name the main window; wildcard is forbidden")
  }
  if ($label -and ($capWindows -notcontains $label)) {
    $failures.Add("Capability windows list does not include window label '$label'")
  }
}

$reports | ForEach-Object { Write-Output $_ }

if ($failures.Count -gt 0) {
  Write-Output "FAIL count=$($failures.Count)"
  $failures | ForEach-Object { Write-Output "FAIL: $_" }
  exit 1
}

Write-Output "PASS capability-locked empty shell"
exit 0
