use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

use async_std::task;

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    // listener.incoming 会在当前阻塞式监听
    // take 方法限制迭代的最大次数
    for stream in listener.incoming().take(5) {
        match stream {
            Ok(stream) => {
                handle_connection(stream).await;
            }
            Err(err) => {
                eprintln!("Failed to get tcpstream, reason:{}", err);
            }
        }
    }
}

async fn handle_connection(mut stream: TcpStream) {
    println!("start...");
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";

    // 访问根或者/sleep路径，显示hello.html页面；访问其他路径，显示err.html
    let (code, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK", "hello.html")
    } else if buffer.starts_with(sleep) {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK", "hello.html")
    } else {
        ("HTTP/1.1 404 Not Found", "err.html")
    };

    let contents = if let Ok(contents) = fs::read_to_string(filename) {
        contents
    } else {
        return;
    };
    let len = contents.len();
    let response = format!("{code}\r\nContent-Length: {len}\r\n\r\n{contents}");
    let result = stream.write_all(response.as_bytes());
    match result {
        Ok(_) => println!("http response: {:#?}", response),
        Err(err) => {
            eprintln!("Failed to respond, reason:{}", err);
        }
    }
    println!("finish...");
}
