BITS 64
global _start
section .text
_start:
    jmp short init
main:
    ; write(1, str, len);
    xor rax, rax
    xor rdx, rdx
    inc al
    mov rdi, rax
    pop rsi
    mov dl, 25
    syscall
    xor rdx, rdx
    jmp short finish
init:
    call main
    db "dont tell anyone im here", 0xa, 0x0
finish:
    ; mov rax, 0x0123456789abcdef
    ; jmp rax
