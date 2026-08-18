param(
    [switch]$DevKit
)

$ErrorActionPreference = "Stop"

$homeDir = $env:USERPROFILE
if (-not $homeDir) { $homeDir = $env:HOME }
$homeFwd = $homeDir.Replace('\', '/')

$projectDir = (Get-Location).Path
$projectFwd = $projectDir.Replace('\', '/')

# Order: Specific project/cargo/rustup paths FIRST, generic home path LAST
$remapFlags = @(
    "--remap-path-prefix ${projectDir}=/src",
    "--remap-path-prefix ${projectFwd}=/src",
    "--remap-path-prefix ${homeDir}\.rustup\toolchains=/rust",
    "--remap-path-prefix ${homeFwd}/.rustup/toolchains=/rust",
    "--remap-path-prefix ${homeDir}\.rustup=/rust",
    "--remap-path-prefix ${homeFwd}/.rustup=/rust",
    "--remap-path-prefix ${homeDir}\.cargo=/cargo",
    "--remap-path-prefix ${homeFwd}/.cargo=/cargo",
    "--remap-path-prefix ${homeDir}=/anon",
    "--remap-path-prefix ${homeFwd}=/anon",
    "--remap-path-prefix .=/src",
    "-C strip=symbols"
) -join " "

$env:RUSTFLAGS = $remapFlags
$cflags = "-ffile-prefix-map=${projectDir}=/src -ffile-prefix-map=${projectFwd}=/src -ffile-prefix-map=${homeDir}=/anon -ffile-prefix-map=${homeFwd}=/anon -ffile-prefix-map=.=/src"
$env:CFLAGS = $cflags
$env:CXXFLAGS = $cflags

Write-Host "Building anonymous Android ARM64 release binary..." -ForegroundColor Cyan

if ($DevKit) {
    Write-Host "DevKit mode ENABLED (--features devkit)" -ForegroundColor Yellow
    cargo apk build --lib --release --features devkit
} else {
    cargo apk build --lib --release
}
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: cargo apk build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

Write-Host "Injecting .so library and packing assets into APK..." -ForegroundColor Cyan
pwsh -File scratch/pack_apk.ps1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: APK packing failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

$unsignedApk = "target/release/apk/snifferlauncher-android-arm64-release-unsigned.apk"
$signedApk = "target/release/apk/snifferlauncher-android-arm64-release-signed.apk"

Write-Host "`n==========================================" -ForegroundColor Green
Write-Host "Build Complete!" -ForegroundColor Green
Write-Host "Unsigned APK: $unsignedApk" -ForegroundColor Green
if (Test-Path $signedApk) {
    Write-Host "Signed APK:   $signedApk" -ForegroundColor Cyan
}
Write-Host "==========================================" -ForegroundColor Green
