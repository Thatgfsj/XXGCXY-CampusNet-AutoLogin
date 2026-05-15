import struct

def inspect(path):
    print(f"\n=== {path} ===")
    with open(path, 'rb') as f:
        data = f.read()
    
    if len(data) < 64:
        print("Too small for PE")
        return
    
    # DOS header
    dos_sig = struct.unpack_from('<H', data, 0)[0]
    if dos_sig != 0x5A4D:
        print("Not a PE")
        return
    
    pe_offset = struct.unpack_from('<I', data, 0x3C)[0]
    print(f"PE offset: {pe_offset:#x}")
    
    # PE signature
    fh = pe_offset + 4
    num_sec = struct.unpack_from('<H', data, fh + 2)[0]
    opt_size = struct.unpack_from('<H', data, fh + 16)[0]
    oh = fh + 20
    
    magic = struct.unpack_from('<H', data, oh)[0]
    is_pe32plus = (magic == 0x20B)
    print(f"PE32+: {is_pe32plus}")
    
    # ImageBase
    if is_pe32plus:
        image_base = struct.unpack_from('<Q', data, oh + 24)[0]
    else:
        image_base = struct.unpack_from('<I', data, oh + 28)[0]
    print(f"ImageBase: {image_base:#x}")
    
    # DataDirectory[9] = TLS
    tls_dd_off = oh + (112 if is_pe32plus else 96) + 9 * 8
    tls_dd_rva, tls_dd_size = struct.unpack_from('<II', data, tls_dd_off)
    print(f"TLS DataDirectory: RVA={tls_dd_rva:#x} Size={tls_dd_size}")
    
    # Section headers
    sec_start = fh + 20 + opt_size
    sections = []
    for i in range(num_sec):
        soff = sec_start + i * 40
        raw = data[soff:soff + 40]
        name_bytes = raw[0:8]
        null_pos = name_bytes.find(b'\x00')
        name = name_bytes[:null_pos].decode('ascii') if null_pos >= 0 else name_bytes.decode('ascii', errors='replace')
        vs = struct.unpack_from('<I', raw, 8)[0]
        va = struct.unpack_from('<I', raw, 12)[0]
        sr = struct.unpack_from('<I', raw, 16)[0]
        pr = struct.unpack_from('<I', raw, 20)[0]
        ch = struct.unpack_from('<I', raw, 36)[0]
        sections.append((name, va, vs, pr, sr, ch))
        print(f"  Section '{name}': VA={va:#x} VSize={vs:#x} RawOff={pr:#x} RawSize={sr:#x} Ch={ch:#08x}")
    
    # If TLS directory is set, parse IMAGE_TLS_DIRECTORY64
    if tls_dd_rva > 0 and tls_dd_size >= 40:
        # Convert RVA to file offset
        tls_file_off = None
        for name, va, vs, pr, sr, ch in sections:
            if va <= tls_dd_rva < va + max(vs, sr):
                tls_file_off = pr + (tls_dd_rva - va)
                break
        
        if tls_file_off:
            start_va = struct.unpack_from('<Q', data, tls_file_off)[0]
            end_va = struct.unpack_from('<Q', data, tls_file_off + 8)[0]
            idx_va = struct.unpack_from('<Q', data, tls_file_off + 16)[0]
            cb_va = struct.unpack_from('<Q', data, tls_file_off + 24)[0]
            zf = struct.unpack_from('<I', data, tls_file_off + 32)[0]
            ch = struct.unpack_from('<I', data, tls_file_off + 36)[0]
            
            start_rva = start_va - image_base if start_va >= image_base else start_va
            end_rva = end_va - image_base if end_va >= image_base else end_va
            idx_rva = idx_va - image_base if idx_va >= image_base else idx_va
            cb_rva = cb_va - image_base if cb_va >= image_base else cb_va
            
            print(f"\n  TLS Directory (at file offset {tls_file_off:#x}):")
            print(f"  StartAddressOfRawData: {start_va:#016x} (RVA={start_rva:#x})")
            print(f"  EndAddressOfRawData:   {end_va:#016x} (RVA={end_rva:#x})")
            print(f"  Template size:         {end_rva - start_rva:#x}")
            print(f"  AddressOfIndex:        {idx_va:#016x} (RVA={idx_rva:#x})")
            print(f"  AddressOfCallBacks:    {cb_va:#016x} (RVA={cb_rva:#x})")
            print(f"  SizeOfZeroFill:        {zf}")
            print(f"  Characteristics:       {ch:#010x}")
            
            # Check if addresses are valid
            for label, va in [("Start", start_va), ("End", end_va), ("Index", idx_va), ("CallBacks", cb_va)]:
                if va == 0:
                    print(f"  ** {label} is NULL **")
                elif va < image_base:
                    print(f"  ** {label} ({va:#x}) < ImageBase — may be pure RVA **")
                else:
                    rva = va - image_base
                    found = False
                    for name, sva, svs, spr, ssr, sch in sections:
                        if sva <= rva < sva + max(svs, ssr):
                            print(f"  {label}: in section '{name}' at offset {rva - sva:#x}")
                            found = True
                            break
                    if not found:
                        print(f"  ** {label}: RVA={rva:#x} NOT in any section **")

inspect("test_linker_tls.exe")
inspect("test_tls_exit.exe")
inspect("test_tls_thread.exe")
inspect("test_simple_exit.exe")
