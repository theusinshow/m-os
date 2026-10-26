# Carimba a versao do M/OS nos cinco lugares que `check-version.ps1` confere.
#
#     ./scripts/set-version.ps1 0.7.15
#
# Roda no CI, no checkout do build, e o carimbo NAO volta para o repositorio:
# o numero de cada release automatico nasce da ultima tag publicada (ADR-076),
# e commitar o bump a cada push faria o proprio push disparar outro release.
param(
  [Parameter(Mandatory = $true)]
  [ValidatePattern('^\d+\.\d+\.\d+$')]
  [string]$Version
)

$ErrorActionPreference = "Stop"

$repositoryRoot = Split-Path -Parent $PSScriptRoot
$cargoManifestPath = Join-Path $repositoryRoot "Cargo.toml"
$packagePath = Join-Path $repositoryRoot "apps/desktop/package.json"
$packageLockPath = Join-Path $repositoryRoot "apps/desktop/package-lock.json"
$tauriConfigPath = Join-Path $repositoryRoot "apps/desktop/src-tauri/tauri.conf.json"

function Set-FileText([string]$Path, [string]$Text) {
  [System.IO.File]::WriteAllText($Path, $Text, [System.Text.UTF8Encoding]::new($false))
}

# Cargo.toml: so a versao de [workspace.package], e nao a de alguma dependencia.
$cargo = Get-Content -Raw -LiteralPath $cargoManifestPath
$cargo = [regex]::Replace(
  $cargo,
  '(?ms)(^\[workspace\.package\]\s*.*?^version\s*=\s*")[^"]+(")',
  "`${1}$Version`${2}",
  1
)
Set-FileText $cargoManifestPath $cargo

# package.json e tauri.conf.json: o primeiro "version" do arquivo e o do pacote.
foreach ($path in @($packagePath, $tauriConfigPath)) {
  $text = Get-Content -Raw -LiteralPath $path
  $text = ([regex]'"version"\s*:\s*"[^"]+"').Replace($text, "`"version`": `"$Version`"", 1)
  Set-FileText $path $text
}

# package-lock.json: a raiz e o pacote "" — os dois primeiros "version".
$lock = Get-Content -Raw -LiteralPath $packageLockPath
$lock = ([regex]'"version"\s*:\s*"[^"]+"').Replace($lock, "`"version`": `"$Version`"", 2)
Set-FileText $packageLockPath $lock

Write-Output "M/OS version stamped as $Version."
