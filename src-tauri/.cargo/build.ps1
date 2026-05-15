# Build script for XXGCXY-CampusNet-AutoLogin v1.7.10
# Uses LLVM-MinGW with x86_64-pc-windows-gnu target
#
# Prerequisites:
#   - LLVM-MinGW at C:\llvm-mingw\llvm-mingw-20260505-msvcrt-x86_64
#   - Rust GNU toolchain (default: stable-x86_64-pc-windows-gnu)
#   - Node.js at C:\Program Files\nodejs
#
# Usage:
#   .\build.ps1              - compile only (cargo build --release)
#   .\build.ps1 -Bundle      - compile + package (npx tauri build)
#   .\build.ps1 -Clean       - clean before build

param([switch]$Bundle, [switch]$Clean)

$ErrorActionPreference = "Stop"
$LLVM_MINGW = "C:\llvm-mingw\llvm-mingw-20260505-msvcrt-x86_64"
$env:PATH = "$LLVM_MINGW\bin;C:\Program Files\nodejs;C:\Users\thatg\.cargo\bin;$env:PATH"
$env:MSYS2_ARG_CONV_EXCL = "*"

Set-Location $PSScriptRoot\..

if ($Clean) {
    Write-Host "Cleaning..."
    cargo clean
}

if ($Bundle) {
    Write-Host "=== Full Tauri Build & Package ==="
    npm run tauri build @args
} else {
    Write-Host "=== Cargo Build (release) ==="
    cargo build --release @args
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "BUILD SUCCESSFUL"
    $exe = ".\target\release\app.exe"
    if (Test-Path $exe) {
        $info = Get-Item $exe
        Write-Host "Output: $($info.FullName) ($([math]::Round($info.Length/1MB, 1)) MB)"
    }
} else {
    Write-Host "BUILD FAILED (exit code $LASTEXITCODE)"
}
