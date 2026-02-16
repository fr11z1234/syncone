# Syncone build-script: tilfoejer Cargo til PATH og koerer tauri build
$cargoPath = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path "$cargoPath\cargo.exe")) {
    Write-Host "Rust/Cargo ikke fundet i $cargoPath" -ForegroundColor Red
    Write-Host "Koer rustup fra https://rustup.rs og gennemfoer installationen." -ForegroundColor Yellow
    Write-Host "Efter install: luk PowerShell, aabn en ny, og koer denne fil igen." -ForegroundColor Yellow
    exit 1
}
$env:Path = "$cargoPath;$env:Path"
Set-Location $PSScriptRoot
npm install
npm run tauri build
Write-Host ""
Write-Host "Færdig. EXE: src-tauri\target\release\Syncone.exe" -ForegroundColor Green
