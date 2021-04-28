BITS 64
%define VXSIZE 0xaaaaaaaaaaaaaaaa
%define ENTRY0 0xbbbbbbbbbbbbbbbb
%define _START 0xcccccccccccccccc

    ; calculate and save PIE delta into rax
    ; add pre-randomization sym._start to find the post-randomization addr
    ; finally, jump to post-randomization addr of _start
    ; https://tmpout.sh/1/11.html
    call get_rip
    mov r9, VXSIZE
    mov r10, ENTRY0
    mov r11, _START
    sub rax, r9
    sub rax, 5
    sub rax, r10
    add rax, r11
    jmp rax
get_rip:
    mov rax, [rsp]
    ret
