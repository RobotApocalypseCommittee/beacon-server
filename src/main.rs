use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{Read, Write};

fn handle_client(mut stream: TcpStream) {
    // 10 byte buffer
    let mut data = [0 as u8; 10];
    loop {
        match stream.read(&mut data) {
            Ok(len) => {
                if len == 0 {
                    println!("Client {} disconnected.", stream.peer_addr().unwrap());
                    break;
                }
                stream.write(&data[..len]).unwrap();
            }
            Err(e) => {
                println!("Error: {}", e);
                println!("An error occurred, terminating connection with {}", stream.peer_addr().unwrap());
                stream.shutdown(Shutdown::Both).unwrap();
                break;
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:80")?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        handle_client(stream?);
    }
    Ok(())
}