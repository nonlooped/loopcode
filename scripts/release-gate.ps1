# Release gate. Invoked via npm run release-gate.
# Exit 0 only when all checks, including the complete SBOM license policy, succeed.

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "=== LoopCode release gate ===" -ForegroundColor Cyan
Write-Host "root=$root"
Write-Host "date=$((Get-Date).ToString('o'))"

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Action
  )
  Write-Host ""
  Write-Host "--- $Name ---" -ForegroundColor Yellow
  & $Action
  if ($LASTEXITCODE -ne 0 -and $null -ne $LASTEXITCODE) {
    throw "step failed: $Name (exit $LASTEXITCODE)"
  }
  Write-Host "OK: $Name" -ForegroundColor Green
}

# 1) TypeScript + full Core unit/integration suite + capability + root license
Invoke-Step "npm test" {
  npm test
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# 2) Lint (tsc already in npm test; clippy is part of lint)
Invoke-Step "npm run lint" {
  npm run lint
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# 3) Complete SBOM and Category X policy
Invoke-Step "npm run sbom:check" {
  npm run sbom:check
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $sbom = Join-Path $root "sbom\loopcode-sbom.json"
  if (-not (Test-Path $sbom)) { throw "SBOM missing: $sbom" }
  if ((Get-Item $sbom).Length -lt 1024) { throw "SBOM unexpectedly small" }
}

# 4) Checksums for primary package outputs
Invoke-Step "npm run checksums" {
  npm run checksums
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

Write-Host ""
Write-Host "=== RELEASE GATE PASSED ===" -ForegroundColor Green
exit 0
