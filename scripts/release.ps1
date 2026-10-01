# release.ps1 — build Crumbtrail, pack it with Velopack, publish a GitHub Release.
#
#   pwsh -NoProfile -File scripts\release.ps1            # build + pack + publish
#   pwsh -NoProfile -File scripts\release.ps1 -NoUpload  # build + pack only (test the installer locally)
#
# Needs: vpk (dotnet tool install -g vpk), gh (logged in), Rust + Node.
# The version comes from src-tauri/tauri.conf.json; the release notes from the
# matching "## X.Y.Z" section of CHANGELOG.md. Installed apps poll the latest
# GitHub Release (channel "win") and update themselves; see src-tauri/src/updater.rs.
param(
    [switch]$NoUpload,
    [string]$Repo = "https://github.com/Blazzer10200/crumbtrail"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

# packId must NOT be "Crumbtrail": Velopack installs to %LOCALAPPDATA%\<packId>
# and wipes that folder on uninstall, which is where snapshots and logs live.
$packId = "Crumbtrail.App"
$version = (Get-Content src-tauri\tauri.conf.json -Raw | ConvertFrom-Json).version
$work = Join-Path $root ".release"
$publish = Join-Path $work "publish"
$out = Join-Path $work "out"
$notes = Join-Path $work "notes.md"

foreach ($tool in "vpk", "gh", "cargo", "npm") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) { throw "$tool is not on PATH" }
}

# Release notes: the CHANGELOG section for this version, minus its heading.
$log = Get-Content CHANGELOG.md -Raw
$m = [regex]::Match($log, "(?ms)^## $([regex]::Escape($version))\b[^\n]*\n(.*?)(?=^## |\z)")
if (-not $m.Success) { throw "CHANGELOG.md has no '## $version' section" }
if ($m.Value -match "unreleased") { throw "CHANGELOG.md still marks $version as unreleased; date it first" }

Write-Output "== Crumbtrail $version =="
if (Test-Path $work) { Remove-Item $work -Recurse -Force }
New-Item -ItemType Directory -Force $publish, $out | Out-Null
Set-Content $notes $m.Groups[1].Value.Trim() -Encoding utf8

Write-Output "-- build"
npm run tauri build -- --no-bundle
if ($LASTEXITCODE) { throw "tauri build failed" }
$target = (cargo metadata --format-version 1 --no-deps --manifest-path src-tauri\Cargo.toml | ConvertFrom-Json).target_directory
Copy-Item (Join-Path $target "release\crumbtrail.exe") $publish

if (-not $NoUpload) {
    $token = gh auth token
    if ($LASTEXITCODE -or -not $token) { throw "gh isn't logged in" }
    # Previous Velopack release, so vpk can build a small delta package. The
    # first Velopack release has none; that's fine.
    Write-Output "-- fetch previous release (for deltas)"
    vpk download github --repoUrl $Repo --token $token -o $out
    if ($LASTEXITCODE) { Write-Warning "no previous Velopack release found; full package only" }
}

Write-Output "-- pack"
vpk pack -u $packId -v $version -p $publish -e crumbtrail.exe `
    --packTitle Crumbtrail --packAuthors Blazzer `
    -i src-tauri\icons\icon.ico --releaseNotes $notes `
    --framework webview2 -o $out
if ($LASTEXITCODE) { throw "vpk pack failed" }

if ($NoUpload) {
    Write-Output "Packed (not uploaded): $out"
    Get-ChildItem $out -File | Format-Table Name, Length -AutoSize
    exit 0
}

Write-Output "-- publish v$version"
vpk upload github --repoUrl $Repo --token $token -o $out `
    --publish --tag "v$version" --releaseName "Crumbtrail $version" --targetCommitish main
if ($LASTEXITCODE) { throw "vpk upload failed" }
$body = Join-Path $work "body.md"
Set-Content $body -Encoding utf8 -Value (
    "**Install:** download ``$packId-win-Setup.exe`` below and run it. " +
    "Crumbtrail keeps itself up to date after that.`n`n" + (Get-Content $notes -Raw))
gh release edit "v$version" --repo $Repo --notes-file $body
if ($LASTEXITCODE) { throw "gh release edit failed" }

Write-Output "Released: $Repo/releases/tag/v$version"
