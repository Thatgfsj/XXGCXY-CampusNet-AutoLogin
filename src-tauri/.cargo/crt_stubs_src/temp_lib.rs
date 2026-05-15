#![no_std]

global_asm!(
    r#"
    // Required symbols — must be strong (linker error without them)
    .global _fltused
    .global mainCRTStartup
    .global WinMainCRTStartup
    .global _DllMainCRTStartup
    .global __chkstk
    .global "??_7type_info@@6B@"

    // Symbols that the linker or Rust std might provide — make weak
    .weak _tls_index
    .weak _tls_used
    .weak __security_check_cookie
    .weak __security_cookie

    .text
mainCRTStartup:
WinMainCRTStartup:
    sub     rsp, 40
    xor     ecx, ecx
    xor     edx, edx
    xor     r8d, r8d
    call    main
    mov     ecx, eax
    call    ExitProcess
    ud2

_DllMainCRTStartup:
    mov     eax, 1
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

    .weak main
main:
    mov     eax, 0
    ret

    .data
_fltused:
    .long 1

__security_cookie:
    .quad 0x00002B992DDFA232

    .bss
_tls_index:
    .long 0

    .data
_tls_used:
    .zero 16
    .quad _tls_index
    .zero 16

    .data
"??_7type_info@@6B@":
    .zero 48
"#
);
