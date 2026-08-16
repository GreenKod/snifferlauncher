# Run Desktop Preview (Windows PowerShell)
# Builds and launches the Sniffer Launcher desktop preview window.

$ErrorActionPreference = "Stop"

$projectDir = (Get-Location).Path

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "Starting Sniffer Launcher Desktop Preview..." -ForegroundColor Cyan
Write-Host "==========================================" -ForegroundColor Cyan

# Ensure target/debug/.plugins is up to date
$debugPluginsDir = "target/debug/.plugins"
if (Test-Path ".plugins") {
    if (-not (Test-Path "target/debug")) {
        New-Item -ItemType Directory -Path "target/debug" -Force | Out-Null
    }
    Copy-Item -Path ".plugins" -Destination $debugPluginsDir -Recurse -Force
}

cargo run --bin snifferlauncher-desktop
