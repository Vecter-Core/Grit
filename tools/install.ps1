# Cài Grit trên Windows (x86_64-pc-windows-msvc). **CHƯA KIỂM** sau các thay đổi của nhánh Seed 1.0 (docs/ENVIRONMENT.md mô tả nền chính của dự án).
# Yêu cầu: Rust (rustup, toolchain msvc), Visual Studio Build Tools (link.exe), LLVM 20.1.8 bản dựng sẵn giải nén ở $env:GRIT_LLVM_DIR (mặc định D:\Tools\llvm-20.1.8).
# Dùng (PowerShell 7): pwsh tools/install.ps1
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")
if (-not $env:GRIT_LLVM_DIR) { $env:GRIT_LLVM_DIR = "D:\Tools\llvm-20.1.8" }
if (-not (Test-Path (Join-Path $env:GRIT_LLVM_DIR "lib\LLVM-C.lib"))) {
  throw "Không thấy LLVM-C.lib trong $env:GRIT_LLVM_DIR. Tải clang+llvm-20.1.8-x86_64-pc-windows-msvc.tar.xz (GitHub llvm-project, llvmorg-20.1.8), giải nén, đặt GRIT_LLVM_DIR."
}
$env:PATH = "$(Join-Path $env:GRIT_LLVM_DIR 'bin');$env:PATH"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw "Thiếu cargo: cài rustup (https://rustup.rs) rồi chạy lại." }
cargo build --release --manifest-path runtime/gt-rt/Cargo.toml
cargo build --release -p gt-driver -p rig --bin gritc --bin rig
& .\target\release\gritc.exe doctor
if ($LASTEXITCODE -ne 0) { throw "gritc doctor ĐỎ: xem các dòng LỖI ở trên." }
Write-Host "Xong. Thêm vào PATH: `$env:PATH = `"$PWD\target\release;`$env:PATH`""
