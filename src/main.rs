use anyhow::Result;
use clap::{App, Arg};
use nix::cmsg_space;
use nix::sys::socket::{ControlMessage, ControlMessageOwned, MsgFlags, RecvMsg};
use nix::sys::uio::IoVec;
use std::net::Ipv4Addr;
use std::os::unix::io::AsRawFd;
use std::os::unix::io::FromRawFd;
use std::os::unix::io::RawFd;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::net::UnixListener;
use tokio::net::{TcpListener, UnixStream};
use tokio::prelude::*;
use tokio::time::Duration;

fn get_stream_from_fd(fd: RawFd) -> std::net::TcpStream {
    unsafe { std::net::TcpStream::from_raw_fd(fd) }
}

async fn handle_client() -> Result<()> {
    let stream = UnixStream::connect("/tmp/hot-socket2").await?;

    let mut buf = [0; 1024];
    let mut cmesg_buf = cmsg_space!([RawFd; 1]);
    let mut flags = MsgFlags::empty();
    flags.insert(MsgFlags::MSG_WAITALL);

    let mut foo = [IoVec::from_mut_slice(&mut buf)];
    loop {
        match nix::sys::socket::recvmsg(stream.as_raw_fd(), &foo, Some(&mut cmesg_buf), flags) {
            Ok(result) => {
                match result.cmsgs().next() {
                    Some(ControlMessageOwned::ScmRights(mut fds)) => {
                        let fd: RawFd = fds.remove(0);
                        let mut socket = TcpStream::from_std(get_stream_from_fd(fd))?;

                        loop {
                            let mut buf = [0; 1024];
                            let n = match socket.read(&mut buf).await {
                                // socket closed
                                Ok(n) if n == 0 => return Ok(()),
                                Ok(n) => n,
                                Err(e) => {
                                    eprintln!("failed to read from socket; err = {:?}", e);
                                    return Ok(());
                                }
                            };

                            // Write the data back
                            let my_string = String::from_utf8_lossy(&buf);
                            println!("Captured -> {}", my_string);
                            if let Err(e) = socket.write_all((&my_string[0..n]).as_ref()).await {
                                eprintln!("failed to write to socket; err = {:?}", e);
                                return Ok(());
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(_e) => {
                println!("waiting for message");
                tokio::time::delay_for(Duration::from_millis(900)).await;
            }
        }
    }
    Ok(())
}

async fn handle_server() -> Result<()> {
    let mut listener = TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 8080)).await?;

    println!(">> LISTENER FD => {}", listener.as_raw_fd());

    loop {
        if let Ok((mut socket, address)) = listener.accept().await {
            tokio::spawn(async move {
                // In a loop, read data from the socket and write the data back.
                println!("Binding to local unix socket");
                let mut ulistener = UnixListener::bind("/tmp/hot-socket2").unwrap();
                println!("Waiting for local unix socket");
                let (mut usocket, uaddr) = ulistener.accept().await.unwrap();

                loop {
                    let mut buf = [0; 1024];
                    let n = match socket.read(&mut buf).await {
                        // socket closed
                        Ok(n) if n == 0 => return,
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("failed to read from socket; err = {:?}", e);
                            return;
                        }
                    };

                    // Write the data back
                    let my_string = String::from_utf8_lossy(&buf);
                    if my_string
                        .split_whitespace()
                        .collect::<String>()
                        .to_ascii_uppercase()
                        .starts_with("STEAL")
                    {
                        println!(">> GOT STEAL\n");
                        println!(">> TCP FD => {}", socket.as_raw_fd());
                        println!(">> UNIX FD => {}", usocket.as_raw_fd());

                        {
                            let fd_arr = [socket.as_raw_fd()];
                            let cmesg_buf = [ControlMessage::ScmRights(&fd_arr)];

                            let foo = [IoVec::from_slice(&mut buf)];
                            let result = nix::sys::socket::sendmsg(
                                usocket.as_raw_fd(),
                                &foo,
                                &cmesg_buf,
                                MsgFlags::empty(),
                                None,
                            )
                            .unwrap();
                            println!(">> SENT SOCKET => {}", result);
                            break;
                        }
                    } else {
                        println!("-> {}", my_string);
                        if let Err(e) = socket.write_all((&my_string[0..n]).as_ref()).await {
                            eprintln!("failed to write to socket; err = {:?}", e);
                            return;
                        }
                    }
                }
            });
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("Hot Reload")
        // All application settings go here...
        // A simple "Flag" argument example (i.e. "-d") using the builder pattern
        .arg(Arg::with_name("client").short('c'.to_string()))
        .get_matches();

    if matches.is_present("client") {
        handle_client().await?
    } else {
        handle_server().await?
    }

    println!("Hello, world!");
    Ok(())
}
