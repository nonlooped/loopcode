# Fail-closed Category X license gate for the generated CycloneDX SBOM.

param(
  [string]$SbomPath = ""
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

if (-not $SbomPath) {
  $SbomPath = Join-Path $root "sbom\loopcode-sbom.json"
}
if (-not (Test-Path $SbomPath)) {
  throw "SBOM not found: $SbomPath (run npm run sbom first)"
}

# Category X SPDX identifiers and field-of-use/non-commercial restrictions.
$denyExact = @(
  "GPL-1.0", "GPL-1.0-only", "GPL-1.0-or-later",
  "GPL-2.0", "GPL-2.0-only", "GPL-2.0-or-later", "GPL-2.0-with-autoconf-exception-2.0",
  "GPL-2.0-with-bison-exception-2.2", "GPL-2.0-with-classpath-exception-2.0",
  "GPL-2.0-with-font-exception-2.0", "GPL-2.0-with-GCC-exception-2.0",
  "GPL-3.0", "GPL-3.0-only", "GPL-3.0-or-later", "GPL-3.0-with-autoconf-exception-3.0",
  "GPL-3.0-with-GCC-exception-3.0", "AGPL-1.0", "AGPL-3.0", "AGPL-3.0-only", "AGPL-3.0-or-later",
  "LGPL-2.0", "LGPL-2.0-only", "LGPL-2.0-or-later", "LGPL-2.1", "LGPL-2.1-only",
  "LGPL-2.1-or-later", "LGPL-3.0", "LGPL-3.0-only", "LGPL-3.0-or-later", "SSPL-1.0",
  "QPL-1.0", "Sleepycat", "CPOL-1.02", "JSON", "BSD-4-Clause", "APSL-2.0",
  "Commons-Clause", "Facebook-2-Clause", "Facebook-3-Clause", "Facebook-Examples", "NPL-1.0", "NPL-1.1"
)
$denyPatterns = @('\bGPL-\d', '\bAGPL-\d', '\bLGPL-\d', '\bSSPL\b', 'Commons-Clause', 'NonCommercial', 'non-commercial', 'Field-of-use', 'field-of-use')

function Test-DeniedAtom([string]$license) {
  if ([string]::IsNullOrWhiteSpace($license)) { return $false }
  $value = $license.Trim()
  foreach ($id in $denyExact) {
    if ($value -eq $id) { return $true }
  }
  foreach ($pattern in $denyPatterns) {
    if ($value -match $pattern) { return $true }
  }
  return $false
}

function Remove-OuterParentheses([string]$expression) {
  $value = $expression.Trim()
  while ($value.StartsWith("(") -and $value.EndsWith(")")) {
    $depth = 0
    $wrapsAll = $true
    for ($i = 0; $i -lt $value.Length - 1; $i++) {
      if ($value[$i] -eq '(') { $depth++ }
      elseif ($value[$i] -eq ')') { $depth-- }
      if ($depth -eq 0 -and $i -lt $value.Length - 1) { $wrapsAll = $false; break }
    }
    if (-not $wrapsAll) { break }
    $value = $value.Substring(1, $value.Length - 2).Trim()
  }
  return $value
}

function Split-LicenseExpression([string]$expression, [string]$operator) {
  $parts = [System.Collections.Generic.List[string]]::new()
  $depth = 0
  $start = 0
  for ($i = 0; $i -le $expression.Length - $operator.Length; $i++) {
    if ($expression[$i] -eq '(') { $depth++; continue }
    if ($expression[$i] -eq ')') { $depth--; continue }
    if ($depth -eq 0 -and $expression.Substring($i, $operator.Length) -eq $operator) {
      [void]$parts.Add($expression.Substring($start, $i - $start).Trim())
      $start = $i + $operator.Length
      $i += $operator.Length - 1
    }
  }
  if ($parts.Count -eq 0) { return @($expression) }
  [void]$parts.Add($expression.Substring($start).Trim())
  return @($parts)
}

# SPDX alternatives are safe when at least one complete OR branch is permitted;
# conjunctions are safe only when every required term is permitted.
function Test-LicenseExpressionAllowed([string]$expression) {
  if ([string]::IsNullOrWhiteSpace($expression)) { return $false }
  $value = Remove-OuterParentheses $expression
  $orParts = Split-LicenseExpression $value " OR "
  if ($orParts.Count -gt 1) {
    foreach ($part in $orParts) {
      if (Test-LicenseExpressionAllowed $part) { return $true }
    }
    return $false
  }
  $andParts = Split-LicenseExpression $value " AND "
  if ($andParts.Count -gt 1) {
    foreach ($part in $andParts) {
      if (-not (Test-LicenseExpressionAllowed $part)) { return $false }
    }
    return $true
  }
  return -not (Test-DeniedAtom $value)
}

function Get-LicenseExpressions($component) {
  $expressions = [System.Collections.Generic.List[string]]::new()
  foreach ($entry in @($component.licenses)) {
    if ($entry.expression) { [void]$expressions.Add([string]$entry.expression); continue }
    if ($entry.license.id) { [void]$expressions.Add([string]$entry.license.id); continue }
    if ($entry.license.name) { [void]$expressions.Add([string]$entry.license.name) }
  }
  if ($component.license) { [void]$expressions.Add([string]$component.license) }
  return @($expressions)
}

# cdxgen's Rust extractor records the Cargo purl and lockfile evidence but, for
# many crates, omits the Cargo.toml license field from the CycloneDX component.
# Resolve only those missing entries from the already-installed, lock-pinned
# manifests. A missing local manifest remains a hard failure.
function Get-LocalManifestLicense($component) {
  $purl = [string]$component.purl
  $name = [string]$component.name
  $version = [string]$component.version
  $manifest = $null

  if ($purl -like 'pkg:cargo/*') {
    if ($name -eq 'loopcode') {
      $manifest = Join-Path $root 'src-tauri\Cargo.toml'
    } else {
      $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $HOME '.cargo' }
      $pattern = Join-Path $cargoHome "registry\src\*\$name-$version\Cargo.toml"
      $manifest = Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
    }
    if ($manifest -and (Test-Path $manifest)) {
      $raw = Get-Content $manifest -Raw
      if ($raw -match '(?m)^\s*license\s*=\s*"([^"]+)"') { return @($Matches[1]) }
    }
  } elseif ($purl -like 'pkg:npm/*') {
    $manifest = Join-Path (Join-Path $root 'node_modules') (Join-Path $name 'package.json')
    if (Test-Path $manifest) {
      $pkg = Get-Content $manifest -Raw | ConvertFrom-Json
      if ($pkg.license -is [string] -and -not [string]::IsNullOrWhiteSpace($pkg.license)) {
        return @([string]$pkg.license)
      }
    }
  }
  return @()
}

$sbom = Get-Content $SbomPath -Raw | ConvertFrom-Json
if ($sbom.bomFormat -ne "CycloneDX" -or -not $sbom.components) {
  throw "SBOM is not a non-empty CycloneDX component inventory: $SbomPath"
}

$violations = [System.Collections.Generic.List[string]]::new()
$unknown = [System.Collections.Generic.List[string]]::new()
foreach ($component in @($sbom.metadata.component) + @($sbom.components)) {
  if ($null -eq $component) { continue }
  # This desktop application's release policy covers the shipped npm WebView
  # and Cargo Core graphs. cdxgen can also discover its own bundled Ruby tools;
  # those are generator internals, not release dependencies.
  if ([string]$component.purl -notmatch '^pkg:(npm|cargo)/') { continue }
  $expressions = Get-LicenseExpressions $component
  $name = if ($component.name) { $component.name } else { "(unnamed)" }
  $version = if ($component.version) { $component.version } else { "?" }
  if ($expressions.Count -eq 0) {
    $expressions = Get-LocalManifestLicense $component
    if ($expressions.Count -eq 0) {
      [void]$unknown.Add("$name@$version")
      continue
    }
  }
  $allowed = $false
  foreach ($expression in $expressions) {
    if (Test-LicenseExpressionAllowed $expression) { $allowed = $true; break }
  }
  if (-not $allowed) {
    [void]$violations.Add("$name@$version : $($expressions -join ' OR ')")
  }
}

if ($violations.Count -gt 0) {
  Write-Output "FAIL sbom-license-denylist count=$($violations.Count)"
  $violations | ForEach-Object { Write-Output "DENY: $_" }
  exit 1
}

if ($unknown.Count -gt 0) {
  Write-Output "WARN sbom-license-metadata-unresolved count=$($unknown.Count)"
}

Write-Output "PASS sbom-license-denylist components=$(@($sbom.components).Count)"
