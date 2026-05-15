"""
Build a proper COFF object file with correct section characteristics.
This bypasses LLVM's assembler which doesn't honor 'w' flag for COFF sections.
"""
import struct, os

OUT = r"C:\Users\thatg\AppData\Local\Temp\XXGCXY-CampusNet-AutoLogin\src-tauri\.cargo\crt_stubs.obj"

SEC_TEXT = 0x60000020
SEC_DATA = 0xC0000040
SEC_RDATA = 0x40000040
SEC_BSS = 0xC0000080
SEC_TLS = 0xC0000040  # read+write+data - MUST match between AAA and ZZZ for merging

IMPORTANT: Due to the complexity and error-proneness of manually building a COFF file,
let me take a simpler approach.

Actually, let me use a much simpler approach: just use NASM which should be available,
or use the fact that LLVM's assembler actually DOES support numeric section characteristics.

Let me try using numeric flags in the section directive instead of "wdr".
