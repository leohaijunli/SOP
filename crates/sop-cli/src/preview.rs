//! A local, read-only viewer for `dist/manifest.json`.
//!
//! This exists so the layout and the help panel can be looked at, and the manifest
//! checked, before the desktop shell is built. It serves one page and one JSON document
//! on the loopback interface and nothing else: no static directory walking, no directory
//! listing, and no binding to anything but `127.0.0.1`.
//!
//! The manifest is rebuilt on every request, so editing a Markdown file and refreshing
//! the browser is the whole edit loop.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::Path;

use sop_repo::manifest;

/// The page is embedded so `sop preview` works from a downloaded binary in any directory.
const PAGE: &str = include_str!("../assets/preview.html");

pub fn serve(root: &Path, port: u16) -> std::io::Result<()> {
    let repo = sop_repo::Repo::open(root);
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))?;
    let address = listener.local_addr()?;

    println!("serving {} at http://{address}/", repo.root().display());
    println!("note: this is a read-only preview of the layout, not the field app");
    println!("press ctrl-c to stop");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = respond(&repo, stream) {
                    eprintln!("request failed: {error}");
                }
            }
            Err(error) => eprintln!("connection failed: {error}"),
        }
    }
    Ok(())
}

fn respond(repo: &sop_repo::Repo, mut stream: TcpStream) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    if reader.read_line(&mut request)? == 0 {
        return Ok(());
    }
    let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();

    // Drain the remaining headers so the client is not left mid-request.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    match path.as_str() {
        "/" | "/index.html" => send(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            PAGE.as_bytes(),
        ),
        "/manifest.json" => {
            let body = manifest::build(repo).to_json();
            send(
                &mut stream,
                "200 OK",
                "application/json; charset=utf-8",
                body.as_bytes(),
            )
        }
        _ => send(
            &mut stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found\n",
        ),
    }
}

fn send(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}
