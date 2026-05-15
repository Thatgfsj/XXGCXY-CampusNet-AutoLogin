#!/usr/bin/env python3
"""Generate an import library for vcruntime140.dll using only the symbols we need."""

import struct
import os

def parse_dll_exports(dll_path):
    """Parse a PE DLL to extract exported symbol names."""
    with open(dll_path, 'rb') as f:
        # Read DOS header
        dos_header = f.read(64)
        pe_offset = struct.unpack_from('<I', dos_header, 0x3C)[0]

        # Read PE signature and COFF header
        f.seek(pe_offset)
        pe_sig = f.read(4)
        coff_header = f.read(20)
        optional_header_size = struct.unpack_from('<H', coff_header, 16)[0]

        # Read the data directory entry for exports (entry 0)
        # Optional header starts at pe_offset + 24
        optional_start = pe_offset + 24
        export_rva_offset = optional_start + 96  # entry 0 is at 0x70 + 0x8*0 = 0x70 -> bytes 112-119
        export_entry = f.read(8)
        f.seek(optional_start + 112)
        export_entry = f.read(8)
        export_rva, export_size = struct.unpack('<II', export_entry)

        if export_rva == 0:
            return []

        # We need to convert RVA to file offset. Find the section containing the RVA.
        section_table_offset = optional_start + optional_header_size
        f.seek(section_table_offset)

        def rva_to_offset(rva):
            f.seek(section_table_offset)
            for i in range(struct.unpack_from('<H', coff_header, 2)[0]):  # NumberOfSections
                section = f.read(40)
                sec_name = section[0:8].rstrip(b'\x00').decode('ascii', errors='replace')
                sec_virtual_address = struct.unpack_from('<I', section, 12)[0]
                sec_virtual_size = struct.unpack_from('<I', section, 8)[0]
                sec_ptr_raw_data = struct.unpack_from('<I', section, 20)[0]
                if sec_virtual_address <= rva < sec_virtual_address + sec_virtual_size:
                    return sec_ptr_raw_data + (rva - sec_virtual_address)
            return None

        # Read export directory
        export_offset = rva_to_offset(export_rva)
        if export_offset is None:
            return []

        f.seek(export_offset)
        export_data = f.read(40)
        num_names = struct.unpack_from('<I', export_data, 24)[0]
        address_of_names = struct.unpack_from('<I', export_data, 32)[0]

        names_offset = rva_to_offset(address_of_names)
        if names_offset is None:
            return []

        exports = []
        f.seek(names_offset)
        for i in range(num_names):
            name_rva = struct.unpack('<I', f.read(4))[0]
            name_offset = rva_to_offset(name_rva)
            if name_offset:
                pos = f.tell()
                f.seek(name_offset)
                name_bytes = []
                while True:
                    b = f.read(1)
                    if b == b'\x00':
                        break
                    name_bytes.append(b)
                name = b''.join(name_bytes).decode('ascii', errors='replace')
                exports.append(name)
                f.seek(pos)

        return exports

def generate_llvm_def(exports, output_path):
    """Generate a .def file for the given exports."""
    with open(output_path, 'w') as f:
        f.write("LIBRARY vcruntime140.dll\n")
        f.write("EXPORTS\n")
        for exp in sorted(exports):
            f.write(f"    {exp}\n")

# Find vcruntime140.dll
dll_path = "C:/Windows/System32/vcruntime140.dll"
if not os.path.exists(dll_path):
    dll_path = "C:/Windows/SysWOW64/vcruntime140.dll"

if not os.path.exists(dll_path):
    print(f"ERROR: vcruntime140.dll not found")
    exit(1)

print(f"Parsing exports from: {dll_path}")
exports = parse_dll_exports(dll_path)

# Filter for the symbols we care about
interesting = [e for e in exports if any(pattern in e.lower() for pattern in [
    'security', 'chkstk', 'tls', 'fltused', 'maincrt', 'winmaincrt', 'dllmaincrt',
    '_crt_', '_initterm'
])]
print(f"Total exports: {len(exports)}")
print(f"Interesting exports: {interesting}")

# Generate the .def file
def_path = "C:/Users/thatg/AppData/Local/Temp/XXGCXY-CampusNet-AutoLogin/src-tauri/.cargo/vcruntime140.def"
generate_llvm_def(exports, def_path)
print(f"Generated: {def_path}")
