# Downloads the pinned PDFium build (bblanchon/pdfium-binaries) into vendor/pdfium/<platform>/.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts/fetch-pdfium.ps1 [-Platform windows-x64]
# Safe to run repeatedly.
param(
    [ValidateSet("windows-x64", "linux-x64", "macos-arm64", "macos-x64")]
    [string]$Platform = "windows-x64"
)
$ErrorActionPreference = "Stop"

$PdfiumTag = "chromium/8066"
$Root = Split-Path -Parent $PSScriptRoot

$archives = @{
    "windows-x64" = @("pdfium-win-x64.tgz", "bin/pdfium.dll")
    "linux-x64"   = @("pdfium-linux-x64.tgz", "lib/libpdfium.so")
    "macos-arm64" = @("pdfium-mac-arm64.tgz", "lib/libpdfium.dylib")
    "macos-x64"   = @("pdfium-mac-x64.tgz", "lib/libpdfium.dylib")
}
$archive, $member = $archives[$Platform]
$lib = Split-Path -Leaf $member
$dest = Join-Path $Root "vendor/pdfium/$Platform"
$tagFile = Join-Path $dest "TAG"

if ((Test-Path (Join-Path $dest $lib)) -and (Test-Path $tagFile) -and ((Get-Content $tagFile -Raw).Trim() -eq $PdfiumTag)) {
    Write-Host "PDFium $PdfiumTag already in $dest"
    exit 0
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("pdfium-" + [System.Guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp | Out-Null
try {
    $url = "https://github.com/bblanchon/pdfium-binaries/releases/download/$PdfiumTag/$archive"
    Write-Host "Downloading $url"
    $ProgressPreference = "SilentlyContinue"
    Invoke-WebRequest -Uri $url -OutFile (Join-Path $tmp $archive) -UseBasicParsing
    # tar.exe ships with Windows 10 1803+ and handles .tgz archives.
    tar -xzf (Join-Path $tmp $archive) -C $tmp $member LICENSE
    if ($LASTEXITCODE -ne 0) { throw "tar failed with exit code $LASTEXITCODE" }

    New-Item -ItemType Directory -Force $dest | Out-Null
    Copy-Item (Join-Path $tmp $member) (Join-Path $dest $lib) -Force
    Copy-Item (Join-Path $tmp "LICENSE") (Join-Path $dest "LICENSE-pdfium.txt") -Force
    Set-Content -Path $tagFile -Value $PdfiumTag -Encoding ascii
    Write-Host "PDFium $PdfiumTag installed in $dest"
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}
