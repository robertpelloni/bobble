<#
  OmniCore unified launcher (Windows / PowerShell) — supersedes the five
  single-purpose scripts in this folder (build/test/dev/run/bundle .sh).
  The .sh originals are kept for Linux/macOS and archived under scripts/archive/.

  Usage:
    .\scripts\omnicore.ps1 build    # frontend + Rust workspace (below-normal priority)
    .\scripts\omnicore.ps1 test     # frontend build + cargo test --workspace
    .\scripts\omnicore.ps1 dev      # Vite preview on http://localhost:1420
    .\scripts\omnicore.ps1 run      # Tauri desktop app (system tray included)
    .\scripts\omnicore.ps1 bundle all|desktop|android|ios
    .\scripts\omnicore.ps1 status   # versions, tools, git state

  Notes:
  - SQLCipher (rusqlite bundled-sqlcipher) needs OpenSSL on Windows; this
    script auto-detects C:\Program Files\OpenSSL-Win64 when OPENSSL_DIR is unset.
  - Heavy builds are lowered to BelowNormal priority so the machine stays usable.
#>
param([Parameter(Position = 0)][string]$Command = 'status', [Parameter(Position = 1)][string]$Target = 'all')
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

function Set-OpenSslIfNeeded {
  if (-not $env:OPENSSL_DIR -and (Test-Path 'C:\Program Files\OpenSSL-Win64\include\openssl\ssl.h')) {
    $env:OPENSSL_DIR = 'C:\Program Files\OpenSSL-Win64'
    # The linker needs the VC lib dir specifically (libcrypto.lib lives in
    # lib\VC\x64\MD, not lib) — without this a fresh target dir fails with
    # LNK1181 even though OPENSSL_DIR is set.
    if (-not $env:OPENSSL_LIB_DIR) { $env:OPENSSL_LIB_DIR = 'C:\Program Files\OpenSSL-Win64\lib\VC\x64\MD' }
    if (-not $env:OPENSSL_INCLUDE_DIR) { $env:OPENSSL_INCLUDE_DIR = 'C:\Program Files\OpenSSL-Win64\include' }
    Write-Host "OPENSSL_DIR auto-set to $env:OPENSSL_DIR (required by bundled SQLCipher)"
  }
}
function Invoke-BelowNormal([string]$File, [string[]]$ArgList) {
  # NOTE: the parameter must not be named $Args — that is PowerShell's
  # automatic variable and Start-Process then receives a null ArgumentList.
  # Uses System.Diagnostics.Process directly because Start-Process handles
  # intermittently report a null ExitCode after WaitForExit in PS 5.1.
  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $File
  $psi.Arguments = ($ArgList | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }) -join ' '
  $psi.UseShellExecute = $false
  $p = [System.Diagnostics.Process]::Start($psi)
  try { $p.PriorityClass = 'BelowNormal' } catch { Write-Warning "Could not lower priority: $_" }
  $p.WaitForExit()
  if ($p.ExitCode -ne 0) { throw "$File $($ArgList -join ' ') failed with exit code $($p.ExitCode)" }
}

switch ($Command) {
  'build' {
    Set-OpenSslIfNeeded
    # Vite writes timing warnings to stderr; with ErrorActionPreference=Stop
    # PowerShell turns those into terminating errors, so run npm with
    # Continue and gate on the exit code instead.
    $prevEAP = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    Push-Location omnicore-ui/ui; try { npm install; if ($LASTEXITCODE -ne 0) { throw 'npm install failed' }; npm run build; if ($LASTEXITCODE -ne 0) { throw 'frontend build failed' } } finally { Pop-Location; $ErrorActionPreference = $prevEAP }
    Invoke-BelowNormal 'cargo' @('build', '--workspace')
    Write-Host '=== OmniCore build successful ==='
  }
  'test' {
    Set-OpenSslIfNeeded
    $prevEAP = $ErrorActionPreference; $ErrorActionPreference = 'Continue'
    Push-Location omnicore-ui/ui; try { npm run build; if ($LASTEXITCODE -ne 0) { throw 'frontend build failed' } } finally { Pop-Location; $ErrorActionPreference = $prevEAP }
    Invoke-BelowNormal 'cargo' @('test', '--workspace')
    Write-Host '=== All OmniCore tests passed ==='
  }
  'dev' { Push-Location omnicore-ui/ui; try { npm run dev } finally { Pop-Location } }
  'run' { Set-OpenSslIfNeeded; Invoke-BelowNormal 'cargo' @('run', '-p', 'omnicore-ui') }
  'bundle' { & bash scripts/bundle.sh $Target }
  'status' {
    Write-Host "OmniCore version: $(Get-Content VERSION.md)"
    node --version; npm --version; cargo --version
    git status -sb; git log --oneline -3
  }
  default { Write-Host 'Usage: omnicore.ps1 build|test|dev|run|bundle [target]|status'; exit 1 }
}
