import subprocess, os

LLD = "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
CRT = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs.obj"
SDK = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\um\x64"
UCRT = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\ucrt\x64"
WORK = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\tls_test"

# First compile the Rust source to obj
test_rs = os.path.join(WORK, "test_sizeofzero.rs")
test_obj = os.path.join(WORK, "test_sizeofzero.obj")
test_exe = os.path.join(WORK, "test_sizeofzero.exe")

print("=== Compile Rust source ===")
r = subprocess.run([
    "rustc", "--edition", "2021", "--emit", "obj",
    test_rs, "-o", test_obj,
], capture_output=True, text=True)
print(f"stdout: {r.stdout}")
print(f"stderr: {r.stderr}")
if r.returncode != 0:
    print("COMPILE FAILED")
    exit(1)

print("\n=== Link ===")
r = subprocess.run([
    LLD, "-flavor", "link",
    "/nologo",
    "/entry:mainCRTStartup",
    "/subsystem:console",
    "/machine:x64",
    f"/out:{test_exe}",
    f"/libpath:{SDK}",
    f"/libpath:{UCRT}",
    "/defaultlib:kernel32",
    "/defaultlib:advapi32",
    CRT,
    test_obj,
], capture_output=True, text=True)
print(f"stdout: {r.stdout}")
print(f"stderr: {r.stderr}")

if r.returncode != 0:
    print("LINK FAILED")
    exit(1)

print(f"\n=== Run {test_exe} ===")
r = subprocess.run([test_exe], capture_output=True)
print(f"Exit code: {r.returncode}")

