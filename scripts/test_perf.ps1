param(
    [int]$WaitTimeSeconds = 10
)

$ErrorActionPreference = "Stop"

$projectDir = (Get-Location).Path

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "1. Building Sniffer Launcher (Release + DevKit)..." -ForegroundColor Cyan
pwsh -File .\scripts\build_android_release.ps1 -DevKit
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

$apkPath = "target/release/apk/snifferlauncher-android-arm64-release-signed.apk"
if (-not (Test-Path $apkPath)) {
    # Try unsigned if signed is not available
    $apkPath = "target/release/apk/snifferlauncher-android-arm64-release-unsigned.apk"
    if (-not (Test-Path $apkPath)) {
        Write-Host "APK not found!" -ForegroundColor Red
        exit 1
    }
}

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host "2. Deploying APK to device..." -ForegroundColor Cyan
adb install -r $apkPath
if ($LASTEXITCODE -ne 0) {
    Write-Host "Failed to install APK! Is your device connected?" -ForegroundColor Red
    exit 1
}

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host "3. Preparing for test..." -ForegroundColor Cyan
# Kill the app if running
adb shell am force-stop com.greenkod.snifferlauncher
# Clear logcat
adb logcat -c

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host "4. Launching Sniffer Launcher (Cold Boot)..." -ForegroundColor Cyan
adb shell monkey -p com.greenkod.snifferlauncher -c android.intent.category.LAUNCHER 1 > $null

Write-Host "Waiting $WaitTimeSeconds seconds for the app to load and for you to interact (scroll)..." -ForegroundColor Yellow
for ($i = $WaitTimeSeconds; $i -gt 0; $i--) {
    Write-Host "Time remaining: $i seconds... (Please scroll the app now!)"
    Start-Sleep -Seconds 1
}

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host "5. Collecting Performance Metrics..." -ForegroundColor Cyan

# Gather gfxinfo framestats
$gfxInfo = adb shell dumpsys gfxinfo com.greenkod.snifferlauncher
$jankMatches = $gfxInfo | Select-String "Janky frames: (.+)"

if ($jankMatches) {
    Write-Host "--- Graphics Performance Summary ---" -ForegroundColor Green
    $gfxInfo | Select-String "Total frames rendered:" | ForEach-Object { Write-Host $_.Line }
    $gfxInfo | Select-String "Janky frames:" | ForEach-Object { Write-Host $_.Line -ForegroundColor Red }
    $gfxInfo | Select-String "90th percentile:" | ForEach-Object { Write-Host $_.Line }
    $gfxInfo | Select-String "95th percentile:" | ForEach-Object { Write-Host $_.Line }
    $gfxInfo | Select-String "99th percentile:" | ForEach-Object { Write-Host $_.Line -ForegroundColor Red }
} else {
    Write-Host "Could not find graphics info. Ensure the app is running and hardware acceleration is enabled." -ForegroundColor Red
}

Write-Host "`n--- Recent Logcat (Errors/Warnings/Stats) ---" -ForegroundColor Green
adb logcat -d -s Rust:V | Select-String "FPS" -Context 0,2
adb logcat -d -e "EGL|Render|FPS|snifferlauncher" | Select-Object -Last 20

Write-Host "`n==========================================" -ForegroundColor Cyan
Write-Host "Test complete." -ForegroundColor Green
