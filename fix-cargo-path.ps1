# Tilfoejer Cargo til din bruger-PATH permanent (saa cargo virker overalt)
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if (-not (Test-Path "$cargoBin\cargo.exe")) {
    Write-Host "Fejl: Cargo ikke fundet i $cargoBin - er Rust installeret?" -ForegroundColor Red
    exit 1
}

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -like "*$cargoBin*") {
    Write-Host "Cargo er allerede i din PATH." -ForegroundColor Green
} else {
    [Environment]::SetEnvironmentVariable("Path", $userPath + ";" + $cargoBin, "User")
    Write-Host "Tilfojet til PATH: $cargoBin" -ForegroundColor Green
}

# Opdater denne session saa cargo virker med det samme
$env:Path = "$cargoBin;$env:Path"
Write-Host ""
Write-Host "Tjek:" -ForegroundColor Cyan
cargo --version
Write-Host ""
Write-Host "Luk PowerShell og aabn en ny, saa virker 'cargo' overalt. Eller koer bygget nu:" -ForegroundColor Yellow
Write-Host "  cd '$PSScriptRoot'; npm run tauri build" -ForegroundColor Gray
