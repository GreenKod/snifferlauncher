$aapt = "C:\Users\greenkod\AppData\Local\Android\Sdk\build-tools\37.0.0\aapt.exe"
$androidJar = "C:\Users\greenkod\AppData\Local\Android\Sdk\platforms\android-35\android.jar"
if (-not (Test-Path $androidJar)) {
    $androidJar = "C:\Users\greenkod\AppData\Local\Android\Sdk\platforms\android-34\android.jar"
}
if (-not (Test-Path $androidJar)) {
    $androidJar = "C:\Users\greenkod\AppData\Local\Android\Sdk\platforms\android-33\android.jar"
}

$manifest = "android/AndroidManifest.xml"
$tempApk = "scratch/temp.apk"

& $aapt package -M $manifest -S "android/res" -I $androidJar -F $tempApk
if ($LASTEXITCODE -ne 0) {
    Write-Host "aapt failed"
    exit $LASTEXITCODE
}

Add-Type -AssemblyName "System.IO.Compression.FileSystem"
$zip = [System.IO.Compression.ZipFile]::Open($tempApk, [System.IO.Compression.ZipArchiveMode]::Read)
$entry = $zip.GetEntry("AndroidManifest.xml")
[System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, "scratch/AndroidManifest.xml", $true)
$zip.Dispose()

Write-Host "Success"
