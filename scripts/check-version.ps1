$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$cargoManifestPath = Join-Path $repositoryRoot "Cargo.toml"
$packagePath = Join-Path $repositoryRoot "apps/desktop/package.json"
$packageLockPath = Join-Path $repositoryRoot "apps/desktop/package-lock.json"
$tauriConfigPath = Join-Path $repositoryRoot "apps/desktop/src-tauri/tauri.conf.json"

$cargoManifest = Get-Content -Raw -LiteralPath $cargoManifestPath
$workspaceMatch = [regex]::Match(
  $cargoManifest,
  '(?ms)^\[workspace\.package\]\s*.*?^version\s*=\s*"(?<version>[^"]+)"'
)
if (-not $workspaceMatch.Success) {
  throw "Workspace version not found in Cargo.toml."
}

$package = Get-Content -Raw -LiteralPath $packagePath | ConvertFrom-Json
$packageLock = Get-Content -Raw -LiteralPath $packageLockPath | ConvertFrom-Json -AsHashtable
$tauriConfig = Get-Content -Raw -LiteralPath $tauriConfigPath | ConvertFrom-Json
$lockRoot = $packageLock["packages"][""]

$versions = [ordered]@{
  "Cargo.toml" = $workspaceMatch.Groups["version"].Value
  "package.json" = $package.version
  "package-lock.json" = $packageLock["version"]
  "package-lock root" = $lockRoot["version"]
  "tauri.conf.json" = $tauriConfig.version
}

$expectedVersion = $versions["Cargo.toml"]
$mismatches = @($versions.GetEnumerator() | Where-Object { $_.Value -ne $expectedVersion })
if ($mismatches.Count -gt 0) {
  $details = ($versions.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }) -join ", "
  throw "Version mismatch: $details"
}

if ($env:GITHUB_REF -like "refs/tags/v*") {
  $tagVersion = $env:GITHUB_REF.Substring("refs/tags/v".Length)
  if ($tagVersion -ne $expectedVersion) {
    throw "Tag v$tagVersion does not match repository version $expectedVersion."
  }
}

Write-Output "M/OS version $expectedVersion is consistent."
