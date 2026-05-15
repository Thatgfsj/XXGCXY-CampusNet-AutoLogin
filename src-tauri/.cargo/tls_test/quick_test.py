"""
Quick test: does Start=End=non-zero with SizeOfZeroFill work?
Builds a minimal TLS test using new crt_stubs.obj and examines PE + runtime behavior.
"""
import subprocess, struct, os, shutil

LLD = "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
CRT_STUBS_SRC = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs_src"
CRT_STUBS = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs.obj"
SDK_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\um\x64"
UCRT_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\ucrt\x64"
WORK_DIR = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\tls_test"

os.chdir(WORK_DIR)

# Step 1: Check crt_stubs.obj TLS directory
print("=== Step 1: Inspect crt_stubs.obj TLS structure ===")
with open(CRT_STUBS, 'rb') as f:
    obj_data = f.read()

# Find rdata$T section
num_sec = struct.unpack_from('<H', obj_data, 2)[0]
sec_offset = 20  # after COFF header

rdataT_off = None
rdataT_size = None
for i in range(num_sec):
    soff = sec_offset + i * 40
    name_raw = obj_data[soff:soff+8]
    null_pos = name_raw.find(b'\x00')
    name = name_raw[:null_pos].decode('ascii') if null_pos >= 0 else name_raw.decode('ascii', errors='replace')
    sr = struct.unpack_from('<I', obj_data, soff + 16)[0]
    pr = struct.unpack_from('<I', obj_data, soff + 20)[0]
    ch = struct.unpack_from('<I', obj_data, soff + 36)[0]
    if name == '.rdata$T':
        rdataT_off = pr
        rdataT_size = sr
        print(f"  .rdata$T: RawOff={pr:#x}, RawSize={sr:#x}, Ch={ch:#010x}")

if rdataT_off:
    # Parse _tls_used: after the anchor (8 bytes)
    sec_data = obj_data[rdataT_off:rdataT_off + rdataT_size]
    # Find anchor magic
    anchor_magic = struct.pack('<Q', 0x524F48434E415F5F)
    anchor_idx = sec_data.find(anchor_magic)
    if anchor_idx >= 0:
        tls_used = sec_data[anchor_idx + 8:anchor_idx + 8 + 40]
        start = struct.unpack_from('<Q', tls_used, 0)[0]
        end = struct.unpack_from('<Q', tls_used, 8)[0]
        idx = struct.unpack_from('<Q', tls_used, 16)[0]
        cb = struct.unpack_from('<Q', tls_used, 24)[0]
        zf = struct.unpack_from('<I', tls_used, 32)[0]
        ch = struct.unpack_from('<I', tls_used, 36)[0]
        print(f"  _tls_used (raw values before relocation):")
        print(f"    Start={start:#x}, End={end:#x}")
        print(f"    Index={idx:#x}, CallBacks={cb:#x}")
        print(f"    SizeOfZeroFill={zf:#x}, Characteristics={ch:#x}")
        if start == end and start != 0:
            print(f"    -> Start==End (non-zero): Zero-size template, good!")
        elif start == 0 and end == 0:
            print(f"    -> Start==End==0: May cause issues with TLS allocation")
        else:
            print(f"    -> Non-matching start/end: Template copy will happen")

# Step 2: Build exit_early test with new CRT
print("\n=== Step 2: Build exit_early CRT stub ===")
exit_early_obj = os.path.join(WORK_DIR, "exit_early_v2.obj")
r = subprocess.run([
    "rustc", "--edition", "2021", "--crate-type", "cdylib", "--emit", "obj",
    os.path.join(CRT_STUBS_SRC, "exit_early.rs"),
    "-o", exit_early_obj,
], capture_output=True, text=True)
if r.returncode != 0:
    print(f"FAIL: {r.stderr}")
    exit(1)

out = os.path.join(WORK_DIR, "test_start_end_trick.exe")
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
    exit_early_obj,
], capture_output=True, text=True)
if r.returncode != 0:
    print(f"LINK FAIL: {r.stderr}")
    exit(1)
print(f"Linked OK: {os.path.getsize(out)} bytes")

# Step 3: Inspect the PE's TLS directory
print("\n=== Step 3: Inspect PE TLS directory ===")
with open(out, 'rb') as f:
    pe_data = f.read()

dos_sig = struct.unpack_from('<H', pe_data, 0)[0]
pe_offset = struct.unpack_from('<I', pe_data, 0x3C)[0]
fh = pe_offset + 4
num_sec = struct.unpack_from('<H', pe_data, fh + 2)[0]
opt_size = struct.unpack_from('<H', pe_data, fh + 16)[0]
oh = fh + 20
magic = struct.unpack_from('<H', pe_data, oh)[0]
is_pe32plus = (magic == 0x20B)

image_base = struct.unpack_from('<Q', pe_data, oh + 24)[0]
print(f"  ImageBase: {image_base:#x}")

tls_dd_off = oh + (112 if is_pe32plus else 96) + 9 * 8
tls_rva, tls_size = struct.unpack_from('<II', pe_data, tls_dd_off)
print(f"  TLS DataDirectory: RVA={tls_rva:#x}, Size={tls_size}")

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
    print(f"  Section '{name}': VA={va:#x} VSize={vs:#x} RawOff={pr:#x} Ch={ch:#010x}")

# Find _tls_used via TLS DataDirectory
if tls_rva > 0:
    # Convert RVA to file offset
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

        print(f"\n  IMAGE_TLS_DIRECTORY64 (at file offset {tls_fo:#x}):")
        print(f"  StartAddressOfRawData: {start:#016x}")
        print(f"  EndAddressOfRawData:   {end:#016x}")
        print(f"  AddressOfIndex:        {idx:#016x}")
        print(f"  AddressOfCallBacks:    {cb:#016x}")
        print(f"  SizeOfZeroFill:        {zf:#x} ({zf} bytes)")
        print(f"  Characteristics:       {ch:#010x}")

        if start == end and start != 0:
            print(f"\n  *** Start==End (non-zero): Zero-size template, SizeOfZeroFill={zf:#x} ***")
            if zf > 0:
                print(f"  *** OS should allocate {zf} zeroed bytes per thread ***")
        elif start == 0 and end == 0:
            print(f"\n  *** Start==End==0: No template, SizeOfZeroFill={zf:#x} ***")
        elif start < end:
            print(f"\n  *** Template copy: {end-start} bytes from {start:#x} ***")

# Step 4: Run it
print("\n=== Step 4: Run test binary ===")
r = subprocess.run([out], capture_output=True)
print(f"Exit code: {r.returncode}" + (" (EXPECTED 88)" if r.returncode == 88 else f" (NTSTATUS={r.returncode:#010x})"))
