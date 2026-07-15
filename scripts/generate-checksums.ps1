# Produce SHA256 checksums for primary frontend/package artifacts (and release
# binaries when present after tauri build).

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$outDir = Join-Path $root "dist-release"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$checksumsPath = Join-Path $outDir "SHA256SUMS.txt"
$lines = New-Object System.Collections.Generic.List[string]

function Get-Sha256Hex {
  param([string]$Path)
  # Portable: some stripped PowerShell hosts lack Get-FileHash.
  $sha = [System.Security.Cryptography.SHA256]::Create()
  try {
    $fs = [System.IO.File]::OpenRead($Path)
    try {
      $bytes = $sha.ComputeHash($fs)
      return ([System.BitConverter]::ToString($bytes) -replace '-', '').ToLowerInvariant()
    } finally {
      $fs.Dispose()
    }
  } finally {
    $sha.Dispose()
  }
}

function Add-Hash {
  param([string]$RelativePath)
  $full = Join-Path $root $RelativePath
  if (-not (Test-Path $full)) {
    Write-Host "skip (missing): $RelativePath"
    return
  }
  $hash = Get-Sha256Hex -Path $full
  $norm = $RelativePath -replace '\\', '/'
  $lines.Add("$hash  $norm")
  Write-Host "hashed $norm"
}

# Primary frontend package outputs
Add-Hash "dist\index.html"
Get-ChildItem -Path (Join-Path $root "dist\assets") -File -ErrorAction SilentlyContinue | ForEach-Object {
  $rel = "dist\assets\$($_.Name)"
  Add-Hash $rel
}

# SBOM + package manifests
Add-Hash "sbom\loopcode-sbom.json"
Add-Hash "package.json"
Add-Hash "package-lock.json"
Add-Hash "src-tauri\Cargo.toml"
Add-Hash "src-tauri\Cargo.lock"
Add-Hash "LICENSE"

# Optional Tauri release binary (env-dependent)
$candidates = @(
  "src-tauri\target\release\loopcode.exe",
  "src-tauri\target\release\loopcode",
  "src-tauri\target\release\bundle\msi\*.msi",
  "src-tauri\target\release\bundle\nsis\*.exe"
)
foreach ($pat in $candidates) {
  $fullPat = Join-Path $root $pat
  Get-Item $fullPat -ErrorAction SilentlyContinue | ForEach-Object {
    $rel = $_.FullName.Substring($root.Length).TrimStart('\', '/')
    Add-Hash $rel
  }
}

if ($lines.Count -eq 0) {
  throw "no artifacts found to checksum; run npm run build first"
}

# UTF-8 without BOM: PowerShell's `utf8` encoding emits U+FEFF which makes the
# first hash token 65 chars and breaks standard sha256sum-style verification.
$utf8NoBom = New-Object System.Text.UTF8Encoding $false
$body = ($lines -join "`n") + "`n"
[System.IO.File]::WriteAllText($checksumsPath, $body, $utf8NoBom)
Write-Host "Wrote $($lines.Count) entries -> $checksumsPath"
Get-Content $checksumsPath
