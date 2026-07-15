# Minimal license gate: root LICENSE is Apache-2.0, and package manifests agree.
# Dependency license policy is enforced separately via check-sbom-licenses.ps1.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot

$failures = [System.Collections.Generic.List[string]]::new()
$reports = [System.Collections.Generic.List[string]]::new()

$licensePath = Join-Path $root "LICENSE"
if (-not (Test-Path $licensePath)) {
  $failures.Add("Missing: LICENSE")
} else {
  $license = Get-Content $licensePath -Raw
  if ($license -notmatch 'Apache License' -or $license -notmatch 'Version 2\.0') {
    $failures.Add("LICENSE does not look like Apache-2.0 text")
  } else {
    $reports.Add("license=Apache-2.0")
  }
}

$cargoPath = Join-Path $root "src-tauri\Cargo.toml"
if (-not (Test-Path $cargoPath)) {
  $failures.Add("Missing: src-tauri/Cargo.toml")
} else {
  $cargo = Get-Content $cargoPath -Raw
  if ($cargo -notmatch 'license\s*=\s*"Apache-2\.0"') {
    $failures.Add('Cargo.toml missing license = "Apache-2.0"')
  } else {
    $reports.Add("cargo_license=Apache-2.0")
  }
}

$pkgPath = Join-Path $root "package.json"
if (-not (Test-Path $pkgPath)) {
  $failures.Add("Missing: package.json")
} else {
  $pkg = Get-Content $pkgPath -Raw | ConvertFrom-Json
  if ($pkg.license -ne "Apache-2.0") {
    $failures.Add("package.json license is not Apache-2.0")
  } else {
    $reports.Add("npm_license=Apache-2.0")
  }
}

# Ceremony files that used to be required — fail if someone reintroduces them.
foreach ($stale in @("DCO", "NOTICE", "REUSE.toml")) {
  if (Test-Path (Join-Path $root $stale)) {
    $failures.Add("Stale license ceremony file present (remove it): $stale")
  }
}

$reports | ForEach-Object { Write-Output $_ }

if ($failures.Count -gt 0) {
  Write-Output "FAIL count=$($failures.Count)"
  $failures | ForEach-Object { Write-Output "FAIL: $_" }
  exit 1
}

Write-Output "PASS license scan clean"
exit 0
