BITS 64
global _start
section .text
_start:
    call main
    db "dont tell anyone im here", 0xa, 0x0
main:
    xor rax, rax
    xor rdx, rdx
    inc al
    mov rdi, rax
    pop rsi
    mov dl, 25
    syscall
    xor rdx, rdx
    ; mov rax, 0x0123456789abcdef
    ; jmp rax
