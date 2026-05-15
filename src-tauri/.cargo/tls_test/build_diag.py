import subprocess

LLD = "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
CRT_STUBS = "C:/Users/thatg/AppData/Local/Temp/XXGCXY-CampusNet-AutoLogin/src-tauri/.cargo/crt_stubs.obj"
OUT = "test_diag.exe"
SDK_LIB = "C:/Program Files (x86)/Windows Kits/10/Lib/10.0.18362.0/um/x64"
UCRT_LIB = "C:/Program Files (x86)/Windows Kits/10/Lib/10.0.18362.0/ucrt/x64"

args = [LLD, "-flavor", "link",
    "/nologo",
    "/entry:mainCRTStartup",
    "/subsystem:console",
    "/machine:x64",
    f"/out:{OUT}",
    f"/libpath:{SDK_LIB}",
    f"/libpath:{UCRT_LIB}",
    "/defaultlib:advapi32",
    CRT_STUBS,
]
result = subprocess.run(args, capture_output=True, text=True)
print("STDOUT:", result.stdout)
print("STDERR:", result.stderr)
print("RC:", result.returncode)
