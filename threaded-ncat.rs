/*
 * Opens and interacts with a TCP connection to 127.0.0.1:4444
 * This tool *feels* like ncat, and could be extended to send a payload 
 * before listening to user input.
 * This version only uses std features, and does not require cargo.
 *
 * Compile with `rustc -C opt-level=3 -C debuginfo=0 threaded-nc.rs`
 * There is an infinite loop at the end; Use Ctrl-C to exit. 
 */
 
use std::net::TcpStream;
use std::io::{self, prelude::*, BufReader, BufWriter};
use std::thread;

fn handle_in(rd: BufReader<TcpStream>) {
    let mut rd = rd;
    let mut buf = [0];
    loop {
        if rd.read(&mut buf).unwrap() == 0 {
            break;
        };
        io::stdout().write(&buf).unwrap();
        io::stdout().flush().unwrap();
    }
}

fn handle_out(wr: BufWriter<TcpStream>) {
    let mut wr = wr;
    let mut buf = [0];
    loop {
        io::stdin().read(&mut buf).unwrap();
        wr.write(&buf).unwrap();
        wr.flush().unwrap();
    }
}

fn main() {
    let stream = TcpStream::connect("127.0.0.1:4444").unwrap();
    let rd = BufReader::new(stream.try_clone().unwrap());
    let wr = BufWriter::new(stream);

    thread::spawn(move || {
        handle_in(rd);
    });
    
    thread::spawn(move || {
        handle_out(wr);
    });

    loop {}
}