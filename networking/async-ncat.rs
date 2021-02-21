/*
 * Opens and interacts with a TCP connection to 127.0.0.1:4444
 * This tool *feels* like ncat, and could be extended to send a payload 
 * before listening to user input.
 * This version uses the Tokio runtime (tokio crate).
 * There is also a threaded version that doesn't require any external crates. 
 */

use std::io::{self, prelude::*};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let stream = TcpStream::connect("127.0.0.1:4444").await
        .expect("could not connect to remote host");

    let (mut rd, mut wr) = stream.into_split();

    let t1 = tokio::spawn(async move {
        let mut buf = [0];
        loop {
            match rd.read(&mut buf).await {
                Err(_) => break,
                Ok(0) => break,
                Ok(_) => {
                    io::stdout().write(&buf).unwrap();
                    io::stdout().flush().unwrap();
                }
            }
        }
    });

    let t2 = tokio::spawn(async move {
        loop {
            let mut buf = String::new();
            io::stdin().read_line(&mut buf).unwrap();
            if wr.write(buf.as_bytes()).await.is_err() {
                break;
            }
            let _ = wr.flush();
        }
    });

    let _ = t1.await;
    let _ = t2.await;
}