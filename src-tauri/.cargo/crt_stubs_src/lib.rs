extern crate core;
use core::arch::global_asm;

global_asm!(
    r#"
    .global _fltused
    .global mainCRTStartup
    .global WinMainCRTStartup
    .global _DllMainCRTStartup
    .global __chkstk
    .global "??_7type_info@@6B@"
    .global __security_cookie
    .global __security_check_cookie
    .global _tls_index
    .global _tls_used
    .global __tls_anchor
    .global __tls_callbacks
    .global __tls_start
    .global __tls_end

    .weak main
    .text
mainCRTStartup:
WinMainCRTStartup:
    sub     rsp, 40
    call    __security_init_cookie
    xor     ecx, ecx
    xor     edx, edx
    xor     r8d, r8d
    call    main
    mov     ecx, eax
    call    ExitProcess
    ud2

_DllMainCRTStartup:
    cmp     edx, 1
    jne     1f
    call    __security_init_cookie
1:
    mov     eax, 1
    ret

__security_init_cookie:
    push    rcx
    push    rdx
    push    rax
    rdtsc
    shl     rdx, 32
    or      rax, rdx
    mov     rcx, 0x2B992DDFA232
    xor     rax, rcx
    lea     rcx, [rip + __security_cookie]
    mov     [rcx], rax
    pop     rax
    pop     rdx
    pop     rcx
    ret

__chkstk:
    push    rcx
    push    r11
    cmp     rax, 0x1000
    jb      2f
    lea     rcx, [rsp + 24]
    sub     rcx, rax
    and     rcx, ~0xFFF
    lea     r11, [rsp + 24]
    and     r11, ~0xFFF
1:
    sub     r11, 0x1000
    test    qword ptr [r11], r11
    cmp     r11, rcx
    jg      1b
2:
    pop     r11
    pop     rcx
    ret

__security_check_cookie:
    ret

    .data
_fltused:
    .long 1

__security_cookie:
    .quad 0x00002B992DDFA232

    // TLS section start/end markers — placed at both extremes so the
    // linker's section-merging puts __tls_start at the beginning of the
    // merged .tls section and __tls_end at the end.
    .section .tls$AAA,"wdr"
    .globl __tls_start
__tls_start:
    .zero 4                     // minimal contribution to keep the section

    .section .tls$ZZZ,"wdr"
    .globl __tls_end
__tls_end:
    .zero 4                     // minimal contribution to keep the section

    .bss
_tls_index:
    .long 0

    // TLS callback array: empty, just a null terminator
    .section .rdata,"dr"
    .balign 8
    .globl __tls_callbacks
__tls_callbacks:
    .quad 0                     // null terminator — no callbacks

    // TLS directory: put a magic anchor right before _tls_used so
    // the post-link wrapper can find it regardless of RVA changes.
    .section .rdata$T,"dr"
    .balign 8
    .globl __tls_anchor
__tls_anchor:
    .quad 0x524F48434E415F5F   // "__ANCHOR" backwards in little-endian
    .globl _tls_used
_tls_used:
    .quad __tls_start           // StartAddressOfRawData = VA of __tls_start (in .tls section)
    .quad __tls_start           // EndAddressOfRawData = same VA → zero-size template, OS skips copy
    .quad _tls_index            // AddressOfIndex → writable .bss DWORD
    .quad __tls_callbacks       // AddressOfCallBacks → null array in .rdata
    .long 0x4000                // SizeOfZeroFill = 16KB zeroed TLS allocation per thread
    .long 0                     // Characteristics = 0

    .data
"??_7type_info@@6B@":
    .zero 48
"#
);
