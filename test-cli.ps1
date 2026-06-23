# Test script for AgentHub CLI

Write-Host "Testing AgentHub CLI..." -ForegroundColor Cyan

# Check if cargo is available
Write-Host "`nChecking Cargo availability..." -ForegroundColor Yellow
try {
    $cargoVersion = rustup run 1.94.1-x86_64-pc-windows-msvc cargo --version 2>&1
    Write-Host "Cargo available: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "Cargo not available" -ForegroundColor Red
    exit 1
}

# Build CLI
Write-Host "`nBuilding CLI..." -ForegroundColor Yellow
Set-Location agenthub-cli
rustup run 1.94.1-x86_64-pc-windows-msvc cargo build 2>&1 | Write-Host

if ($LASTEXITCODE -eq 0) {
    Write-Host "`nBuild successful!" -ForegroundColor Green
    
    # Test commands
    Write-Host "`nTesting 'list' command (all agents)..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe list 2>&1 | Write-Host
    
    Write-Host "`nTesting 'list --type cli' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe list --type cli 2>&1 | Write-Host
    
    Write-Host "`nTesting 'list --type desktop' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe list --type desktop 2>&1 | Write-Host
    
    Write-Host "`nTesting 'search codex' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe search codex 2>&1 | Write-Host
    
    Write-Host "`nTesting 'search cursor --type desktop' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe search cursor --type desktop 2>&1 | Write-Host
    
    Write-Host "`nTesting 'info codex' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe info codex 2>&1 | Write-Host
    
    Write-Host "`nTesting 'info cursor' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe info cursor 2>&1 | Write-Host
    
    # Test batch operations
    Write-Host "`nTesting batch operations..." -ForegroundColor Yellow
    Write-Host "`nTesting 'install --help' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe install --help 2>&1 | Write-Host
    
    Write-Host "`nTesting 'uninstall --help' command..." -ForegroundColor Yellow
    .\target\debug\agenthub-cli.exe uninstall --help 2>&1 | Write-Host
    
    Write-Host "`nBatch operations help test complete!" -ForegroundColor Green
} else {
    Write-Host "`nBuild failed!" -ForegroundColor Red
}

Set-Location ..
