use std::{fs, time::Duration};

use async_std::{
    io::{ReadExt, WriteExt},
    net::{TcpListener, TcpStream},
    task,
};
use futures::StreamExt;

#[async_std::main]
async fn main() {
    /*
    异步版本的 TcpListener 为 listener.incoming() 实现了 Stream 特征，以上修改有两个好处:
    listener.incoming() 不再阻塞
    使用 for_each_concurrent 并发地处理从 Stream 获取的元素
    */
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    listener
        .incoming()
        .for_each_concurrent(/* 并发数限制 */ 5, |stream| async move {
            task::spawn(handle_connection(stream.unwrap()));
        })
        .await;
}

async fn handle_connection(mut stream: TcpStream) {
    println!("start...");
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await.unwrap();

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
    let result = stream.write_all(response.as_bytes()).await;
    match result {
        Ok(_) => println!("http response: {:#?}", response),
        Err(err) => {
            eprintln!("Failed to respond, reason:{}", err);
        }
    }
    stream.flush().await.unwrap();
    println!("finish...");
}
