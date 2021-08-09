# hot-reload-example
Example of open socket handover between process by sending FDs over IPC in Rust. Inspired by Envoy proxy.

## Running
This example is rather fragile, but demonstrates the basics. If you follow the steps in order, you'll see how it all works.

First build the project via `cargo build`

Then follow the steps below:
1) Start in "server" mode -> `./target/debug/hot-reload`
2) Connect to the "server" via netcat or telnet -> `nc 127.0.0.1 8080`
3) Start the example again in client mode (while the server mode is running), you will see "waiting fro message" printed every 1 second while it's waiting for FD handover -> `./target/debug/hot-reload -c`
4) Send any lines / messages you want. You will see them printed in the stdout from the "server", they will also be echoed back to you in netcat.
5) To trigger the socket handover between processes send the message "STEAL". The next line you send from `nc` will appear in the stdout for the "client" process.

If you want to rerun this process and something broke while doing the above steps. You might need to delete `/tmp/hot-socket2` if it didnt get cleaned up. As mentioned... this example is fragile :)

## How this works
More or less this is an example implementation of the process as described by https://copyconstruct.medium.com/file-descriptor-transfer-over-unix-domain-sockets-dcbbf5b3b6ec
and implemented in Envoy proxy (https://blog.envoyproxy.io/envoy-hot-restart-1d16b14555b5). 

tl;dr -> We send a control message to the new process over a unix socket using a SCMrights message which gives the receiving process access to the socket File Description (not file descriptor) of the original socket.
We can then construct a socket from the raw FD and start reading from it. 

You can apparently use some newer system calls available in the linux kernel (5.6+) which is more ergonomic. 

It does require some different permissions related to ptrace though. The method here only requires access to the Unix socket (iirc). 

See https://copyconstruct.medium.com/seamless-file-descriptor-transfer-between-processes-with-pidfd-and-pidfd-getfd-816afcd19ed4
