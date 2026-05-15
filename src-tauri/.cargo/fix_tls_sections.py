"""
Post-process crt_stubs.obj to fix .tls section characteristics.
LLVM's assembler for COFF doesn't honor the 'w' (WRITE) flag.
This script ORs in IMAGE_SCN_MEM_WRITE (0x80000000).
"""
import struct, os

OBJ = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs.obj"

# Read entire file
with open(OBJ, 'rb') as f:
    data = bytearray(f.read())

num_sec = struct.unpack_from('<H', data, 2)[0]
sec_offset = 20  # after COFF header

fixed = 0
for i in range(num_sec):
    soff = sec_offset + i * 40
    name_raw = bytes(data[soff:soff + 8])
    null_pos = name_raw.find(b'\x00')
    if null_pos < 0:
        null_pos = 8
    name = name_raw[:null_pos].decode('ascii', errors='replace')

    if name.startswith('.tls$'):
        ch = struct.unpack_from('<I', data, soff + 36)[0]
        # Set to exactly 0xC0000040 (WRITE|READ|INITIALIZED_DATA) to match Rust's .tls sections
        new_ch = 0xC0000040
        if ch != new_ch:
            struct.pack_into('<I', data, soff + 36, new_ch)
            print(f"  Fixed {name}: {ch:#010x} -> {new_ch:#010x}")
            fixed += 1
        else:
            print(f"  {name}: already correct ({ch:#010x})")

if fixed > 0:
    with open(OBJ, 'wb') as f:
        f.write(data)
    print(f"\nFixed {fixed} section characteristics")
else:
    print("No fixes needed (or sections not found)")
