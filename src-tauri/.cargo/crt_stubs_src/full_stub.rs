use core::arch::global_asm;

global_asm!(
    r#"
    .global _fltused
    .global _tls_index
    .global _tls_used
    .global mainCRTStartup
    .global WinMainCRTStartup
    .global __chkstk
    .global main

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

main:
    mov     eax, 42
    ret

__chkstk:
    ret

    .data
_fltused:
    .long 1

    .bss
_tls_index:
    .long 0

    .data
_tls_used:
    .zero 16
    .quad _tls_index
    .zero 16
"#
);
