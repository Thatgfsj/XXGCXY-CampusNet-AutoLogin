#![no_std]
use core::arch::global_asm;
global_asm!(
    r#"
    .global main
    .text
main:
    mov     eax, 88
    ret
    "#
);
