#!/usr/bin/env python3
"""
Wrapper around rust-lld that patches the TLS directory after linking.

LLD does not set the TLS DataDirectory entry in the PE optional header
when _tls_used is provided by an external object (our crt_stubs.obj).
This script finds our _tls_used via a magic anchor symbol, sets the
PE TLS DataDirectory entry, and patches the Start/End addresses from
the actual .tls section boundaries.
"""

import sys
import os
import struct
import subprocess

LLD_PATH = (
    "C:/Users/thatg/.rustup/toolchains/stable-x86_64-pc-windows-msvc"
    "/lib/rustlib/x86_64-pc-windows-msvc/bin/rust-lld.exe"
)

# Magic value placed in __tls_anchor immediately before _tls_used
ANCHOR_MAGIC = 0x524F48434E415F5F  # "__ANCHOR" in little-endian
IMAGE_DIRECTORY_ENTRY_TLS = 9


def patch_pe(path):
    """Fix TLS DataDirectory and patch _tls_used in the PE file."""
    try:
        with open(path, 'r+b') as f:
            data = f.read()
            if len(data) < 64:
                return

            # --- Parse PE headers ---
            dos_sig = struct.unpack_from('<H', data, 0)[0]
            if dos_sig != 0x5A4D:
                return
            pe_offset = struct.unpack_from('<I', data, 0x3C)[0]
            fh = pe_offset + 4  # FileHeader after PE sig
            num_sec = struct.unpack_from('<H', data, fh + 2)[0]
            opt_size = struct.unpack_from('<H', data, fh + 16)[0]
            oh = fh + 20  # OptionalHeader
            magic = struct.unpack_from('<H', data, oh)[0]
            is_pe32plus = (magic == 0x20B)

            # DataDirectory[9] = TLS
            tls_dd_off = oh + (112 if is_pe32plus else 96) + IMAGE_DIRECTORY_ENTRY_TLS * 8
            tls_rva, tls_size = struct.unpack_from('<II', data, tls_dd_off)

            # --- Parse section headers ---
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
                sections.append((name, va, vs, pr, sr))

            # --- Find __tls_anchor in .rdata ---
            anchor_rva = None
            tls_used_rva = None
            tls_used_file_off = None
            for name, va, vs, pr, sr in sections:
                if name == '.rdata' and pr > 0:
                    sec_data = data[pr:pr + sr]
                    magic_bytes = struct.pack('<Q', ANCHOR_MAGIC)
                    idx = sec_data.find(magic_bytes)
                    if idx >= 0:
                        anchor_rva = va + idx
                        tls_used_rva = va + idx + 8  # right after anchor
                        tls_used_file_off = pr + idx + 8
                        break

            if tls_used_rva is None:
                # No anchor found (maybe crt_stubs.obj not linked yet)
                return

            # --- Find .tls section ---
            tls_start_rva = None
            tls_end_rva = None
            for name, va, vs, pr, sr in sections:
                if name == '.tls':
                    tls_start_rva = va
                    tls_end_rva = va + vs  # VirtualSize = actual TLS data size
                    break

            # --- Set TLS DataDirectory if not already set ---
            if tls_rva == 0:
                f.seek(tls_dd_off)
                f.write(struct.pack('<II', tls_used_rva, 40))  # sizeof IMAGE_TLS_DIRECTORY64
                print(f"[lld-wrapper] {os.path.basename(path)}: "
                      f"set TLS DataDirectory -> RVA={tls_used_rva:#x}")

            # --- Get ImageBase from PE optional header ---
            if is_pe32plus:
                image_base = struct.unpack_from('<Q', data, oh + 24)[0]
            else:
                image_base = struct.unpack_from('<I', data, oh + 28)[0]

            # --- Patch _tls_used if .tls section exists ---
            # NOTE: IMAGE_TLS_DIRECTORY64 fields are VAs (ImageBase + RVA), not pure RVAs.
            if tls_start_rva is not None:
                f.seek(tls_used_file_off)
                current_start = struct.unpack('<Q', f.read(8))[0]
                current_end = struct.unpack('<Q', f.read(8))[0]

                va_start = image_base + tls_start_rva
                va_end = image_base + tls_end_rva

                f.seek(tls_used_file_off)
                f.write(struct.pack('<Q', va_start))  # StartAddressOfRawData (VA)
                f.write(struct.pack('<Q', va_end))    # EndAddressOfRawData (VA)

                if current_start != tls_start_rva or current_end != tls_end_rva:
                    print(f"[lld-wrapper] {os.path.basename(path)}: "
                          f"patched TLS Start={tls_start_rva:#x} End={tls_end_rva:#x} "
                          f"(was Start={current_start:#x} End={current_end:#x})")

    except Exception as e:
        print(f"[lld-wrapper] ERROR in {os.path.basename(path)}: {e}")


def main():
    args = sys.argv[1:] if len(sys.argv) > 1 else []

    out_path = None
    for arg in args:
        if arg.upper().startswith('/OUT:'):
            out_path = arg[5:]
            break

    # Run the real linker
    result = subprocess.run([LLD_PATH, "-flavor", "link"] + args)

    if result.returncode == 0 and out_path and os.path.exists(out_path):
        patch_pe(out_path)

    sys.exit(result.returncode)


if __name__ == '__main__':
    main()
