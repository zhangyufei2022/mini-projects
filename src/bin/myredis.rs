use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("socket addr: {:?}", addr);
        tokio::spawn(async move { handle_connection(socket).await });
    }
}

async fn handle_connection(socket: TcpStream) {
    use mini_redis::{Command, Connection, Frame};
    use std::collections::HashMap;

    // `Connection` 对于 redis 的读写进行了抽象封装，因此我们读到的是一个一个数据帧frame(数据帧 = redis命令 + 数据)，而不是字节流
    // `Connection` 是在 mini-redis 中定义
    let mut connection = Connection::new(socket);

    let mut db: HashMap<String, Vec<u8>> = HashMap::new();

    while let Some(frame) = connection.read_frame().await.unwrap() {
        println!("Got frame: {:?}", frame);

        let cmd = Command::from_frame(frame).unwrap();
        println!("cmd: {:?}", cmd);

        let response = match cmd {
            Command::Get(cmd) => {
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone().into())
                } else {
                    Frame::Null
                }
            }
            Command::Set(cmd) => {
                db.insert(cmd.key().to_string(), cmd.value().to_vec());
                Frame::Simple("OK".to_string())
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };
        connection.write_frame(&response).await.unwrap();
    }
}
