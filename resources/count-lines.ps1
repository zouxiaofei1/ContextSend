param(
    [string]$Path = (Join-Path $PSScriptRoot "..\contextsend"),
    [switch]$ByFile,
    [switch]$ByDir
)

$root = Resolve-Path -LiteralPath $Path

$excludeDirs = @('node_modules', 'target', 'dist', '.git', 'icons', 'gen')

$includeExts = @(
    '.rs', '.ts', '.vue', '.css', '.html', '.js', '.mjs',
    '.json', '.toml',  '.xml'
)

$totalLines = 0
$totalFiles = 0
$stats = @{}

foreach ($ext in $includeExts) {
    $stats[$ext] = @{ Files = 0; Lines = 0 }
}

$allFiles = @(Get-ChildItem -LiteralPath $root -Recurse -File)

foreach ($file in $allFiles) {
    $skip = $false
    foreach ($dir in $excludeDirs) {
        if ($file.FullName -match "[\\/]$dir[\\/]") {
            $skip = $true
            break
        }
    }
    if ($skip) { continue }

    $ext = $file.Extension.ToLower()

    if ($ext -eq '.json' -and $file.Name -eq 'package-lock.json') { continue }

    if ($ext -eq '' -or $ext -eq '.lock' -or $ext -eq '.ignore') {
        if ($file.Name -match '\.lock$') { $ext = '.lock' }
        elseif ($file.Name -match 'ignore$') { $ext = '.ignore' }
    }

    if ($ext -eq '.prettierignore' -or $file.Name -eq '.prettierrc.json') {
        $ext = '.prettierignore'
    }

    if (-not $stats.ContainsKey($ext)) { continue }

    try {
        $lines = @(Get-Content -LiteralPath $file.FullName -ErrorAction Stop).Count
    } catch {
        continue
    }

    $stats[$ext].Files++
    $stats[$ext].Lines += $lines
    $totalFiles++
    $totalLines += $lines
}

Write-Host "`n=== Code Statistics for: $root ===" -ForegroundColor Cyan
Write-Host ""

if ($ByFile) {
    Write-Host ("{0,-8} {1,-8} {2,-10} {3}" -f 'Lines', 'Type', 'Dir', 'File') -ForegroundColor Yellow
    Write-Host ('-' * 80)

    foreach ($file in $allFiles) {
        $skip = $false
        foreach ($dir in $excludeDirs) {
            if ($file.FullName -match "[\\/]$dir[\\/]") {
                $skip = $true
                break
            }
        }
        if ($skip) { continue }

        $ext = $file.Extension.ToLower()
        if ($ext -eq '.prettierignore' -or $file.Name -eq '.prettierrc.json') { $ext = '.prettierignore' }
        if ($ext -eq '' -and $file.Name -match '\.lock$') { $ext = '.lock' }
        if ($ext -eq '' -and $file.Name -match 'ignore$') { $ext = '.ignore' }

        if (-not $stats.ContainsKey($ext)) { continue }

        try {
            $lines = @(Get-Content -LiteralPath $file.FullName -ErrorAction Stop).Count
        } catch { continue }

        $relDir = (Get-Item (Split-Path $file.FullName -Parent)).Name
        Write-Host ("{0,-8} {1,-8} {2,-10} {3}" -f $lines, $ext, $relDir, $file.Name)
    }
    Write-Host ""
}

Write-Host ("{0,-10} {1,-8} {2,-10}" -f 'Lines', 'Files', 'Type') -ForegroundColor Yellow
Write-Host ('-' * 30)

$sorted = $stats.GetEnumerator() | Where-Object { $_.Value.Files -gt 0 } | Sort-Object { $_.Value.Lines } -Descending

foreach ($entry in $sorted) {
    Write-Host ("{0,-10} {1,-8} {2,-10}" -f $entry.Value.Lines, $entry.Value.Files, $entry.Key)
}

Write-Host ('-' * 30)
Write-Host ("{0,-10} {1,-8} {2}" -f $totalLines, $totalFiles, 'Total') -ForegroundColor Green

if ($ByDir) {
    Write-Host ""
    Write-Host "=== Lines by Directory ===" -ForegroundColor Cyan
    Write-Host ""
    $dirStats = @{}
    foreach ($file in $allFiles) {
        $skip = $false
        foreach ($dir in $excludeDirs) {
            if ($file.FullName -match "[\\/]$dir[\\/]") { $skip = $true; break }
        }
        if ($skip) { continue }

        $ext = $file.Extension.ToLower()
        if ($ext -eq '.prettierignore' -or $file.Name -eq '.prettierrc.json') { $ext = '.prettierignore' }
        if ($ext -eq '' -and $file.Name -match '\.lock$') { $ext = '.lock' }
        if ($ext -eq '' -and $file.Name -match 'ignore$') { $ext = '.ignore' }
        if (-not $stats.ContainsKey($ext)) { continue }

        $relPath = $file.FullName.Substring($root.Length + 1)
        $topDir = if ($relPath -match '^[^\\/]+') { $matches[0] } else { '.' }

        if (-not $dirStats.ContainsKey($topDir)) {
            $dirStats[$topDir] = @{ Files = 0; Lines = 0 }
        }
        try {
            $lines = @(Get-Content -LiteralPath $file.FullName -ErrorAction Stop).Count
        } catch { continue }
        $dirStats[$topDir].Files++
        $dirStats[$topDir].Lines += $lines
    }

    Write-Host ("{0,-20} {1,-10} {2,-10}" -f 'Directory', 'Lines', 'Files') -ForegroundColor Yellow
    Write-Host ('-' * 42)
    foreach ($entry in ($dirStats.GetEnumerator() | Sort-Object { $_.Value.Lines } -Descending)) {
        Write-Host ("{0,-20} {1,-10} {2,-10}" -f $entry.Key, $entry.Value.Lines, $entry.Value.Files)
    }
    Write-Host ('-' * 42)
    $dirTotalLines = ($dirStats.Values | Measure-Object -Property Lines -Sum).Sum
    $dirTotalFiles = ($dirStats.Values | Measure-Object -Property Files -Sum).Sum
    Write-Host ("{0,-20} {1,-10} {2,-10}" -f 'Total', $dirTotalLines, $dirTotalFiles) -ForegroundColor Green
}

Write-Host ""
if (-not [Console]::IsInputRedirected) {
    Write-Host "Press Enter to exit..." -ForegroundColor DarkGray
    $null = [Console]::ReadKey($true)
}
