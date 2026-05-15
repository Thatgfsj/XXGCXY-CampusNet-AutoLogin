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

    .weak main
main:
    mov     eax, 0
    ret

    .data
_fltused:
    .long 1

__security_cookie:
    .quad 0x00002B992DDFA232

    .data
"??_7type_info@@6B@":
    .zero 48
"#
);
