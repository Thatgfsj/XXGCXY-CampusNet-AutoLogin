import subprocess, struct, os

LLD = "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
CRT_STUBS = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs.obj"
SDK_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\um\x64"
UCRT_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\ucrt\x64"
WORK = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\tls_test"

os.chdir(WORK)

# Create minimal main.obj that just provides main() -> ExitProcess(88)
# We'll use NASM-like assembly. Actually, let's use rustc to create a tiny obj.
main_rs = os.path.join(WORK, "just_main.rs")
with open(main_rs, 'w') as f:
    f.write(r"""
#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn main() -> i32 {
    88
}
""")

# Also need to provide panic handler
# Actually it's easier to just create assembly with rustc
print("=== Compile minimal main.obj ===")
main_obj = os.path.join(WORK, "just_main.obj")
r = subprocess.run([
    "rustc", "--edition", "2021", "--emit", "obj",
    main_rs, "-o", main_obj,
], capture_output=True, text=True)
print(f"stdout: {r.stdout}, stderr: {r.stderr}")
if r.returncode != 0:
    # Missing panic handler, try with #![no_std] using global asm
    print("Retrying with assembly approach...")
    
    # Write assembly manually using a raw object
    # Actually, let's just use the crt_stubs exit_early approach
    # but compile it separately without the full CRT
    
    exit_src = os.path.join(WORK, "exit_early_simple.rs")
    with open(exit_src, 'w') as f:
        f.write(r"""#![no_std]
use core::arch::global_asm;
global_asm!(
    r#"
    .global main
    .text
main:
    mov     eax, 88
    ret
    "#
);
""")
    main_obj = os.path.join(WORK, "exit_minimal.obj")
    r = subprocess.run([
        "rustc", "--edition", "2021", "--crate-type", "cdylib", "--emit", "obj",
        exit_src, "-o", main_obj,
    ], capture_output=True, text=True)
    print(f"stdout: {r.stdout}, stderr: {r.stderr}")
    if r.returncode != 0:
        print("FAIL:", r.stderr)
        exit(1)

print("\n=== Link test binary ===")
out = os.path.join(WORK, "test_start_end_trick.exe")
r = subprocess.run([
    LLD, "-flavor", "link",
    "/nologo",
    "/entry:mainCRTStartup",
    "/subsystem:console",
    "/machine:x64",
    f"/out:{out}",
    f"/libpath:{SDK_LIB}",
    f"/libpath:{UCRT_LIB}",
    "/defaultlib:kernel32",
    "/defaultlib:advapi32",
    CRT_STUBS,
    main_obj,
], capture_output=True, text=True)
if r.returncode != 0:
    print(f"LINK FAIL: {r.stderr}")
    exit(1)
print(f"OK: {os.path.getsize(out)} bytes")

print("\n=== Inspect PE TLS ===")
with open(out, 'rb') as f:
    pe_data = f.read()

pe_offset = struct.unpack_from('<I', pe_data, 0x3C)[0]
fh = pe_offset + 4
num_sec = struct.unpack_from('<H', pe_data, fh + 2)[0]
opt_size = struct.unpack_from('<H', pe_data, fh + 16)[0]
oh = fh + 20
magic = struct.unpack_from('<H', pe_data, oh)[0]
is_pe32plus = (magic == 0x20B)

image_base = struct.unpack_from('<Q', pe_data, oh + 24)[0]

tls_dd_off = oh + (112 if is_pe32plus else 96) + 9 * 8
tls_rva, tls_size = struct.unpack_from('<II', pe_data, tls_dd_off)

sec_start = fh + 20 + opt_size
sections = []
for i in range(num_sec):
    soff = sec_start + i * 40
    raw = pe_data[soff:soff + 40]
    name_bytes = raw[0:8]
    null_pos = name_bytes.find(b'\x00')
    name = name_bytes[:null_pos].decode('ascii') if null_pos >= 0 else name_bytes.decode('ascii', errors='replace')
    vs = struct.unpack_from('<I', raw, 8)[0]
    va = struct.unpack_from('<I', raw, 12)[0]
    sr = struct.unpack_from('<I', raw, 16)[0]
    pr = struct.unpack_from('<I', raw, 20)[0]
    ch = struct.unpack_from('<I', raw, 36)[0]
    sections.append((name, va, vs, pr, sr, ch))

# Find TLS data
if tls_rva > 0:
    tls_fo = None
    for name, va, vs, pr, sr, ch in sections:
        if va <= tls_rva < va + max(vs, sr):
            tls_fo = pr + (tls_rva - va)
            break
    
    if tls_fo:
        start = struct.unpack_from('<Q', pe_data, tls_fo)[0]
        end = struct.unpack_from('<Q', pe_data, tls_fo + 8)[0]
        idx = struct.unpack_from('<Q', pe_data, tls_fo + 16)[0]
        cb = struct.unpack_from('<Q', pe_data, tls_fo + 24)[0]
        zf = struct.unpack_from('<I', pe_data, tls_fo + 32)[0]
        ch = struct.unpack_from('<I', pe_data, tls_fo + 36)[0]

        print(f"  ImageBase: {image_base:#x}")
        print(f"  Start:   {start:#016x} (RVA={start-image_base:#x})")
        print(f"  End:     {end:#016x} (RVA={end-image_base:#x})")
        print(f"  Index:   {idx:#016x}")
        print(f"  CBs:     {cb:#016x}")
        print(f"  ZFill:   {zf:#x}")
        print(f"  Ch:      {ch:#x}")
        print(f"  Start==End==NonZero: {start == end and start != 0}")

print("\n=== Run ===")
r = subprocess.run([out], capture_output=True)
print(f"Exit code: {r.returncode}" + (" SUCCESS" if r.returncode == 88 else f" CRASH ({r.returncode:#010x})"))
