use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use mini_projects::ThreadPool;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = ThreadPool::new(3);

    // listener.incoming 会在当前阻塞式监听
    // take 方法限制迭代的最大次数
    for stream in listener.incoming().take(5) {
        match stream {
            Ok(stream) => {
                pool.submit(|| {
                    handle_connection(stream);
                });
            }
            Err(err) => {
                eprintln!("Failed to get tcpstream, reason:{}", err);
            }
        }
    }
    println!("Shutting down...");
}

fn handle_connection(mut stream: TcpStream) {
    let buffer_reader = BufReader::new(&mut stream);
    let request: Vec<_> = buffer_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();
    // println!("http request: {:#?}", request);

    let first_line = if let Some(first_line) = request.get(0) {
        first_line
    } else {
        return;
    };

    // 访问根或者/sleep路径，显示hello.html页面；访问其他路径，显示err.html
    let (code, filename) = match &first_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
        _ => ("HTTP/1.1 404 Not Found", "err.html"),
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
}
