# Desktop Release Builder (Windows PowerShell)
# Builds anonymous Desktop release binary with compiled executable and remapped paths.

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

Write-Host "Building anonymous Desktop release binary..." -ForegroundColor Cyan

cargo build --release --bin snifferlauncher
if ($LASTEXITCODE -ne 0) {
    Write-Host "Error: Cargo desktop build failed!" -ForegroundColor Red
    exit $LASTEXITCODE
}

$exePath = "target/release/snifferlauncher.exe"
$releasePluginsDir = "target/release/.plugins"

Write-Host "Bundling .plugins directory and plugin manifests..." -ForegroundColor Cyan
if (Test-Path ".plugins") {
    if (Test-Path $releasePluginsDir) {
        Remove-Item -Path $releasePluginsDir -Recurse -Force
    }
    Copy-Item -Path ".plugins" -Destination $releasePluginsDir -Recurse -Force
    Write-Host "Successfully bundled .plugins into target/release/.plugins!" -ForegroundColor Green
}

Write-Host "`n==========================================" -ForegroundColor Green
Write-Host "Build Complete!" -ForegroundColor Green
if (Test-Path $exePath) {
    Write-Host "Binary:  $exePath" -ForegroundColor Green
    Write-Host "Plugins: $releasePluginsDir" -ForegroundColor Green
} else {
    Write-Host "Binary created in target/release/" -ForegroundColor Green
}
Write-Host "==========================================" -ForegroundColor Green
