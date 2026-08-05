# generate-checksums.ps1 - Generate SHA-256SUMS.txt for release artifacts
#
# Usage:
#   powershell -File scripts/generate-checksums.ps1 <artifact-dir>
#
# Hashes every file in <artifact-dir> and writes SHA-256SUMS.txt
# inside that directory with lines: "<hash>  <filename>"

param(
    [Parameter(Mandatory = $true)]
    [string]$Dir
)

if (-not (Test-Path $Dir -PathType Container)) {
    Write-Error "Directory not found: $Dir"
    exit 1
}

$resolved = (Resolve-Path $Dir).Path
$outFile = Join-Path $resolved "SHA-256SUMS.txt"
$lines = @()
$sha256 = [System.Security.Cryptography.SHA256]::Create()

try {
    Get-ChildItem -Path $resolved -File | Where-Object { $_.Name -ne 'SHA-256SUMS.txt' } | ForEach-Object {
        $stream = [System.IO.File]::OpenRead($_.FullName)
        try {
            $hashBytes = $sha256.ComputeHash($stream)
            $hex = [System.BitConverter]::ToString($hashBytes).Replace('-', '').ToLower()
            $lines += ("{0}  {1}" -f $hex, $_.Name)
        } finally {
            $stream.Dispose()
        }
    }
} finally {
    $sha256.Dispose()
}

$lines | Sort-Object | Set-Content -Path $outFile -Encoding ascii
Write-Host "[OK] Wrote $outFile ($($lines.Count) files)"
