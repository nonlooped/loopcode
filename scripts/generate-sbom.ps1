# Generate a complete CycloneDX inventory for the JavaScript WebView and Rust Core.
# cdxgen consumes the committed npm and Cargo lockfiles; it must never install or
# resolve a different dependency set while preparing a release artifact.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root "sbom"
$outPath = Join-Path $outDir "loopcode-sbom.json"
$cdxgen = Join-Path $root "node_modules\@cyclonedx\cdxgen\bin\cdxgen.js"

if (-not (Test-Path $cdxgen)) {
  throw "Missing @cyclonedx/cdxgen. Restore the locked dependencies with npm ci before generating an SBOM."
}
if (-not (Test-Path (Join-Path $root "package-lock.json"))) {
  throw "Missing package-lock.json; refusing to generate an unverifiable npm inventory."
}
if (-not (Test-Path (Join-Path $root "src-tauri\Cargo.lock"))) {
  throw "Missing src-tauri/Cargo.lock; refusing to generate an unverifiable Cargo inventory."
}

New-Item -ItemType Directory -Force -Path $outDir | Out-Null

# Specify both project types. Recurse is required because Cargo.toml lives in
# src-tauri rather than beside package.json. --fail-on-error keeps an extractor
# failure from producing a deceptively partial release artifact.
& node $cdxgen `
  --type nodejs `
  --type rust `
  --recurse `
  --no-install-deps `
  --fail-on-error `
  --validate `
  --spec-version 1.6 `
  --output $outPath `
  $root
if ($LASTEXITCODE -ne 0) {
  throw "cdxgen failed with exit code $LASTEXITCODE"
}

$sbom = Get-Content $outPath -Raw | ConvertFrom-Json
if ($sbom.bomFormat -ne "CycloneDX" -or $sbom.specVersion -ne "1.6") {
  throw "cdxgen produced an invalid CycloneDX 1.6 document"
}

$components = @($sbom.components)
if ($components.Count -eq 0) {
  throw "cdxgen produced no components"
}

$purls = @($components | ForEach-Object { [string]$_.purl })
if (-not ($purls -match '^pkg:npm/')) {
  throw "SBOM does not contain an npm component"
}
if (-not ($purls -match '^pkg:cargo/')) {
  throw "SBOM does not contain a Cargo component"
}

Write-Output "PASS sbom-generated components=$($components.Count) path=$outPath"
