import struct

def inspect_obj(path):
    print(f"\n=== {path} ===")
    with open(path, 'rb') as f:
        data = f.read()
    
    # COFF object file
    if data[:2] == b'MZ':
        print("This is a PE, not COFF object")
        return
    
    # COFF header
    machine = struct.unpack_from('<H', data, 0)[0]
    num_sections = struct.unpack_from('<H', data, 2)[0]
    print(f"Machine: {machine:#06x}, Sections: {num_sections}")
    
    # Symbol table is at the end; we care about sections first
    # Parse sections (each section header is 40 bytes)
    sec_offset = 20  # after COFF header
    
    for i in range(num_sections):
        soff = sec_offset + i * 40
        name_raw = data[soff:soff+8]
        null_pos = name_raw.find(b'\x00')
        if null_pos >= 0:
            name = name_raw[:null_pos].decode('ascii')
        else:
            name = name_raw.decode('ascii', errors='replace')
        
        vs = struct.unpack_from('<I', data, soff + 8)[0]
        va = struct.unpack_from('<I', data, soff + 12)[0]
        sr = struct.unpack_from('<I', data, soff + 16)[0]
        pr = struct.unpack_from('<I', data, soff + 20)[0]
        ch = struct.unpack_from('<I', data, soff + 36)[0]
        
        print(f"  Section '{name}': VSize={vs:#x} RawOff={pr:#x} RawSize={sr:#x} Ch={ch:#010x}")

inspect_obj("C:/Users/thatg/AppData/Local/Temp/XXGCXY-CampusNet-AutoLogin/src-tauri/.cargo/crt_stubs.obj")
