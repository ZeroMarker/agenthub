# Test script for AgentHub GUI

Write-Host "Testing AgentHub GUI..." -ForegroundColor Cyan

# Check if Node.js is available
Write-Host "`nChecking Node.js availability..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version 2>&1
    Write-Host "Node.js available: $nodeVersion" -ForegroundColor Green
} catch {
    Write-Host "Node.js not available" -ForegroundColor Red
    exit 1
}

# Install dependencies
Write-Host "`nInstalling dependencies..." -ForegroundColor Yellow
Set-Location agenthub-ui
npm install 2>&1 | Write-Host

# Build frontend
Write-Host "`nBuilding frontend..." -ForegroundColor Yellow
npm run build 2>&1 | Write-Host

if ($LASTEXITCODE -eq 0) {
    Write-Host "`nFrontend build successful!" -ForegroundColor Green
    Write-Host "`nFrontend is ready for Tauri integration." -ForegroundColor Green
    Write-Host "`nTo run the GUI, you need to:" -ForegroundColor Yellow
    Write-Host "1. Install Visual Studio Build Tools" -ForegroundColor Yellow
    Write-Host "2. Run 'npm run tauri dev'" -ForegroundColor Yellow
} else {
    Write-Host "`nFrontend build failed!" -ForegroundColor Red
}

Set-Location ..
