$env:NEXA_PUBLIC_KEY = Get-Content "$PSScriptRoot\nexa_verify.pub"

Write-Host "Building nexa-updater..." -ForegroundColor Cyan
cargo build -p nexa-updater --release
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "Building Nexa..." -ForegroundColor Cyan
cargo build -p Nexa --release
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "Done." -ForegroundColor Green