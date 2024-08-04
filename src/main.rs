use tokio::net::{TcpStream,TcpListener};
use tokio::io::AsyncWriteExt;
use tokio::fs;
use std::error::Error;
use std::io;
use std::str::from_utf8;

// Returns either a path that was requested or the appropriate error
// to send back to the client.
fn parse_request_line(request_line: &str) -> Result<&str, &str>{
    let mut terms = request_line.split(' ');
    let req_type = terms.next().ok_or("HTTP/1.1 400 No Request Type\r\n\r\n")?;
    let path = terms.next().ok_or("HTTP/1.1 400 No Path\r\n\r\n")?;
    let _version = terms.next().ok_or("HTTP/1.1 400 No Version\r\n\r\n")?;
    if req_type != "get" && req_type != "GET" {
        return Err("HTTP/1.1 403 Only GET requests are supported\r\n\r\n")
    } else {
        return Ok(path);
    }
}

async fn process_stream(mut stream:TcpStream) -> Result<(), Box<dyn Error>>{
    // try to parse a request and answer accordingly

    loop {
        stream.readable().await?;
        let mut line_buf = [0; 4096];
        match stream.try_read(&mut line_buf) {
            Ok(0) => break,
            Ok(n) => {
                let mut request_contents = from_utf8(&line_buf[0..n])?.lines();
                let request_line = match request_contents.next() {
                    Some(a) => a,
                    None => {
                        return Err(
                            Box::<dyn Error>::from(
                                format!("No request line in the initial contents. Read {} bytes successfully", n)));
                    },
                };

                match parse_request_line(request_line) {
                    Ok(path) => {
                        // we have the path now, and we know it's a GET request
                        match fs::read(path).await {
                            Err(_) => {
                                // TODO better error responses
                                return stream.write_all(b"HTTP/1.1 500 Couldn't get file contents\r\n\r\n")
                                    .await
                                    .map_err(|err| Box::new(err) as Box<dyn Error>);
                            },
                            Ok(file_contents) => {
                                stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n")
                                    .await
                                    .map_err(|err| Box::new(err) as Box<dyn Error>)?;
                                return stream.write_all(&file_contents[..])
                                    .await
                                    .map_err(|err| Box::new(err) as Box<dyn Error>);
                            },
                        }
                    },
                    Err(response) => {
                        return stream.write_all(response.as_bytes())
                            .await
                            .map_err(|err| Box::new(err) as Box<dyn Error>);
                    },
                };

            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            },
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    panic!();
}

const HOST_ADDR: [u8;4] = [127, 0, 0, 1];
const HOST_PORT: u32 = 3030;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind(format!("{}.{}.{}.{}:{}", HOST_ADDR[0],
                                             HOST_ADDR[0],
                                             HOST_ADDR[0],
                                             HOST_ADDR[0],
                                             HOST_PORT)).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        match process_stream(stream).await {
            Ok(()) => {},
            Err(e) => panic!("Response error {}", e),
        }
    }
}
