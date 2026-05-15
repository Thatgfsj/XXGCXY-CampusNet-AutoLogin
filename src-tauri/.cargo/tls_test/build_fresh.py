"""
Build a fresh test binary and check if non-zero TLS Start/End crashes.
"""
import subprocess, struct, os

LLD = "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
CRT_STUBS_SRC = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs_src"
SDK_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\um\x64"
UCRT_LIB = r"C:\Program Files (x86)\Windows Kits\10\Lib\10.0.18362.0\ucrt\x64"
WORK_DIR = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\tls_test"

os.chdir(WORK_DIR)

# Step 1: Build exit_early CRT stub
print("=== Step 1: Build exit_early CRT stub ===")
exit_early_obj = os.path.join(WORK_DIR, "exit_early_new.obj")
r = subprocess.run([
    "rustc", "--edition", "2021", "--crate-type", "cdylib", "--emit", "obj",
    os.path.join(CRT_STUBS_SRC, "exit_early.rs"),
    "-o", exit_early_obj,
], capture_output=True, text=True)
if r.returncode != 0:
    print(f"FAIL: {r.stderr}")
    exit(1)
print(f"OK: {exit_early_obj}")

# Step 2: Link with LLD
print("\n=== Step 2: Link exit_early.exe ===")
out = os.path.join(WORK_DIR, "test_fresh_early.exe")
args = [
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
    exit_early_obj,
]
r = subprocess.run(args, capture_output=True, text=True)
if r.returncode != 0:
    print(f"FAIL: {r.stderr}")
    exit(1)
print(f"OK: {out}")

# Verify file exists
print(f"  File exists: {os.path.exists(out)}, size: {os.path.getsize(out)}")

# Step 3: Run the EXE
print("\n=== Step 3: Run exit_early.exe ===")
r = subprocess.run([out], capture_output=True)
print(f"Exit code: {r.returncode} (expected 88)")

# Step 4: Inspect PE structure
print("\n=== Step 4: PE structure ===")

with open(out, 'rb') as f:
    orig_data = f.read()

dos_sig = struct.unpack_from('<H', orig_data, 0)[0]
pe_offset = struct.unpack_from('<I', orig_data, 0x3C)[0]
fh = pe_offset + 4
num_sec = struct.unpack_from('<H', orig_data, fh + 2)[0]
opt_size = struct.unpack_from('<H', orig_data, fh + 16)[0]
oh = fh + 20
magic = struct.unpack_from('<H', orig_data, oh)[0]
is_pe32plus = (magic == 0x20B)

if is_pe32plus:
    image_base = struct.unpack_from('<Q', orig_data, oh + 24)[0]
else:
    image_base = struct.unpack_from('<I', orig_data, oh + 28)[0]

tls_dd_off = oh + (112 if is_pe32plus else 96) + 9 * 8
tls_dd_rva, tls_dd_size = struct.unpack_from('<II', orig_data, tls_dd_off)

print(f"  ImageBase: {image_base:#x}")
print(f"  TLS DataDirectory: RVA={tls_dd_rva:#x} Size={tls_dd_size}")

sec_start = fh + 20 + opt_size
sections = []
for i in range(num_sec):
    soff = sec_start + i * 40
    raw = orig_data[soff:soff + 40]
    name_bytes = raw[0:8]
    null_pos = name_bytes.find(b'\x00')
    name = name_bytes[:null_pos].decode('ascii') if null_pos >= 0 else name_bytes.decode('ascii', errors='replace')
    vs = struct.unpack_from('<I', raw, 8)[0]
    va = struct.unpack_from('<I', raw, 12)[0]
    sr = struct.unpack_from('<I', raw, 16)[0]
    pr = struct.unpack_from('<I', raw, 20)[0]
    ch = struct.unpack_from('<I', raw, 36)[0]
    sections.append((name, va, vs, pr, sr, ch))
    print(f"  Section '{name}': VA={va:#x} VSize={vs:#x} RawOff={pr:#x} RawSize={sr:#x} Ch={ch:#010x}")

# Find .rdata section
rdata_va = None
rdata_pr = None
for name, va, vs, pr, sr, ch in sections:
    if name == '.rdata':
        rdata_va = va
        rdata_pr = pr
        break

if rdata_va is None:
    print("No .rdata section found!")
    exit(1)

# Step 5: Patch TLS directory with non-zero Start/End pointing to .rdata
print("\n=== Step 5: Patch TLS with non-zero Start/End ===")

pe_data = bytearray(orig_data)

# Search for 40 consecutive zero bytes in .rdata region for _tls_used
zero40 = b'\x00' * 40
tls_used_off = None
search_start = rdata_pr
search_end = min(rdata_pr + 0x200, len(pe_data))
zero_idx = pe_data.find(zero40, search_start, search_end)
if zero_idx >= 0:
    tls_used_off = zero_idx
    print(f"  Found 40 zero bytes at file offset {tls_used_off:#x}")
else:
    # Try 24 bytes
    zero_idx = pe_data.find(b'\x00' * 24, search_start, search_end)
    if zero_idx >= 0:
        tls_used_off = zero_idx
        print(f"  Found 24 zero bytes at file offset {tls_used_off:#x}")

if tls_used_off is None:
    print("  No suitable space for _tls_used!")
    exit(1)

# Find .data section for writable AddressOfIndex
data_va = None
for name, va, vs, pr, sr, ch in sections:
    if name == '.data':
        data_va = va
        break

if data_va is None:
    print("No .data section found!")
    exit(1)

# Write IMAGE_TLS_DIRECTORY64
# Start/End can point to .rdata (read-only is fine for template copy - OS reads, not writes)
# AddressOfIndex MUST point to writable memory (.data or .bss) - OS writes the TLS index
va_start = image_base + rdata_va
va_end = image_base + rdata_va + 8
va_index = image_base + data_va  # writable .data section
va_cb = 0

struct.pack_into('<Q', pe_data, tls_used_off + 0, va_start)
struct.pack_into('<Q', pe_data, tls_used_off + 8, va_end)
struct.pack_into('<Q', pe_data, tls_used_off + 16, va_index)
struct.pack_into('<Q', pe_data, tls_used_off + 24, va_cb)
struct.pack_into('<I', pe_data, tls_used_off + 32, 0)
struct.pack_into('<I', pe_data, tls_used_off + 36, 0)

# Set TLS DataDirectory
tls_used_rva = rdata_va + (tls_used_off - rdata_pr)
struct.pack_into('<II', pe_data, tls_dd_off, tls_used_rva, 40)

print(f"  _tls_used at file={tls_used_off:#x}, RVA={tls_used_rva:#x}")
print(f"  Start: {va_start:#016x} (RVA={rdata_va:#x})")
print(f"  End:   {va_end:#016x} (RVA={rdata_va + 8:#x})")
print(f"  Index: {va_index:#016x}")

# Write patched binary
patched_out = os.path.join(WORK_DIR, "test_fresh_early_patched.exe")
with open(patched_out, 'wb') as f:
    f.write(pe_data)
print(f"\n  Written: {patched_out}")

# Step 6: Run patched binary
print("\n=== Step 6: Run patched binary ===")
r = subprocess.run([patched_out], capture_output=True)
print(f"Exit code: {r.returncode}")
if r.returncode >= 0x80000000:
    print(f"  -> NTSTATUS error: {r.returncode:#010x}")
