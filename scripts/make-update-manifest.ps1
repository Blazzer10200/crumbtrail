# make-update-manifest.ps1 — build the updater's latest.json from a fresh
# `npm run tauri build`. Run AFTER building the signed installer, then upload the
# NSIS setup + latest.json to the matching GitHub Release.
#
#   pwsh -NoProfile -File scripts\make-update-manifest.ps1 -Version 0.4.0 -Notes "What changed"
#
# The app's updater (tauri.conf.json > plugins.updater.endpoints) fetches
# https://github.com/<repo>/releases/latest/download/latest.json, compares the
# version, and — if newer — downloads the signed setup and verifies it against the
# baked-in public key before installing.

param(
  [Parameter(Mandatory)][string]$Version,
  [string]$Notes = "",
  [string]$Repo = "Blazzer10200/crumbtrail",
  [string]$BundleDir = ""
)
$ErrorActionPreference = 'Stop'

if (-not $BundleDir) {
  $candidates = @(
    (Join-Path $PSScriptRoot "..\src-tauri\target\release\bundle\nsis"),
    "C:\cargo-targets\release\bundle\nsis"
  )
  $BundleDir = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $BundleDir) { throw "No NSIS bundle dir found — run 'npm run tauri build' first." }

$setup = Get-ChildItem (Join-Path $BundleDir "*-setup.exe") -ErrorAction SilentlyContinue | Select-Object -First 1
$sig = Get-ChildItem (Join-Path $BundleDir "*-setup.exe.sig") -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $setup) { throw "No *-setup.exe in $BundleDir" }
if (-not $sig) { throw "No *-setup.exe.sig in $BundleDir — did you build with TAURI_SIGNING_PRIVATE_KEY set?" }

$signature = (Get-Content -LiteralPath $sig.FullName -Raw).Trim()
$url = "https://github.com/$Repo/releases/download/v$Version/$($setup.Name)"
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$manifest = [ordered]@{
  version   = $Version
  notes     = $Notes
  pub_date  = $pubDate
  platforms = [ordered]@{
    "windows-x86_64" = [ordered]@{
      signature = $signature
      url       = $url
    }
  }
}

$out = Join-Path $BundleDir "latest.json"
$manifest | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $out -Encoding UTF8
Write-Output "Wrote $out"
Write-Output ""
Write-Output "Next: create GitHub Release tag 'v$Version' and upload BOTH:"
Write-Output "  - $($setup.FullName)"
Write-Output "  - $out"
