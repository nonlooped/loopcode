# Targeted regression filters for RC (security, suspend, updates, single-instance).

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

Write-Host "=== LoopCode spot regression ==="

function Run-Cargo {
  param([string]$ArgsLine)
  Write-Host "cargo test $ArgsLine"
  Invoke-Expression "cargo test --manifest-path src-tauri/Cargo.toml $ArgsLine"
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}

# Security: path boundary + shell approval
Run-Cargo "--test security_slice3"
# Suspend / reconcile honesty
Run-Cargo "--test runtime_slice2 crash_suspend"
Run-Cargo "--lib runtime::engine"
# Reliability: signature reject + suspend limits
Run-Cargo "--test reliability_slice9 update_check"
Run-Cargo "--test reliability_slice9 suspend_reconcile"
# Exclusive single-instance
Run-Cargo "--test a11y_slice10 single_instance"
Run-Cargo "--lib a11y::single_instance"

Write-Host "=== SPOT REGRESSION PASSED ===" -ForegroundColor Green
exit 0
