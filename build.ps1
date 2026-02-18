# Syncone build-script: tilfoejer Cargo til PATH og koerer tauri build
$cargoPath = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path "$cargoPath\cargo.exe")) {
    Write-Host "Rust/Cargo ikke fundet i $cargoPath" -ForegroundColor Red
    Write-Host "Koer rustup fra https://rustup.rs og gennemfoer installationen." -ForegroundColor Yellow
    Write-Host "Efter install: luk PowerShell, aabn en ny, og koer denne fil igen." -ForegroundColor Yellow
    exit 1
}

$keyPath = "$env:USERPROFILE\.tauri\syncone.key"
if (-not (Test-Path $keyPath)) {
    Write-Host "Signing key not found at $keyPath" -ForegroundColor Red
    Write-Host "Run: npx tauri signer generate -w `"$keyPath`"" -ForegroundColor Yellow
    exit 1
}
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $keyPath -Raw
if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = Read-Host "Enter signing key password"
}

$env:Path = "$cargoPath;$env:Path"
Set-Location $PSScriptRoot
npm install
npm run tauri build

# Tauri outputs versioned names (Syncone_1.7.0_x64-setup.exe); we also produce fixed Syncone.exe for stable download URL
$nsisDir = Join-Path $PSScriptRoot "src-tauri\target\release\bundle\nsis"
$confPath = Join-Path $PSScriptRoot "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw | ConvertFrom-Json
$version = $conf.version
$productName = $conf.productName
$versionedExe = "${productName}_${version}_x64-setup.exe"
$versionedSig = "${versionedExe}.sig"
$fixedExeName = "Syncone.exe"
$fixedSigName = "Syncone.exe.sig"

$versionedExePath = Join-Path $nsisDir $versionedExe
$versionedSigPath = Join-Path $nsisDir $versionedSig
$fixedExePath = Join-Path $nsisDir $fixedExeName
$fixedSigPath = Join-Path $nsisDir $fixedSigName

if (Test-Path $versionedExePath) {
    Copy-Item $versionedExePath $fixedExePath -Force
    Write-Host "Copied $versionedExe -> $fixedExeName" -ForegroundColor Green
}
if (Test-Path $versionedSigPath) {
    Copy-Item $versionedSigPath $fixedSigPath -Force
    Write-Host "Copied $versionedSig -> $fixedSigName" -ForegroundColor Green
}

# latest.json: use stable URL so /releases/latest/download/Syncone.exe always gets the newest
$endpoint = $conf.plugins.updater.endpoints[0]
$downloadBase = $endpoint -replace "/releases/latest/download/latest\.json$", "/releases/latest/download"
$url = "$downloadBase/$fixedExeName"
$sigPath = $fixedSigPath
if (Test-Path $sigPath) {
    $signature = (Get-Content $sigPath -Raw).Trim()
    $pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    $latest = @{
        version = $version
        notes   = ""
        pub_date = $pubDate
        platforms = @{
            "windows-x86_64" = @{
                signature = $signature
                url       = $url
            }
        }
    } | ConvertTo-Json -Depth 4 -Compress
    $latestPath = Join-Path $nsisDir "latest.json"
    [System.IO.File]::WriteAllText($latestPath, $latest)
    Write-Host "Generated latest.json (version $version, url: .../Syncone.exe)" -ForegroundColor Green
} else {
    Write-Host "Warning: $sigPath not found; latest.json not updated." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Faerdig! Installer + signature + latest.json:" -ForegroundColor Green
Write-Host "  src-tauri\target\release\bundle\nsis\" -ForegroundColor Cyan
Write-Host ""
Write-Host "Upload the .exe, .sig and latest.json to a new GitHub Release." -ForegroundColor Yellow
