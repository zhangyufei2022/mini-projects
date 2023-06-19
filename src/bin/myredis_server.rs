use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use mini_redis::Result;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info};
type Db = Arc<Mutex<HashMap<String, Bytes>>>;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    tokio::select! {
        res = run(&listener, &db) => {
            if let Err(err) = res {
                error!(cause = %err, "failed to accept");
            }
        }
        _ = tokio::signal::ctrl_c() => {
            info!("shutting down");
        }
    }
}

async fn run(listener: &TcpListener, db: &Db) -> Result<()> {
    loop {
        let (socket, addr) = listener.accept().await?;
        debug!("socket addr: {:?}", addr);
        let db = db.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket, db).await {
                error!(cause = ?err, "connection error");
            }
        });
    }
}

async fn handle_connection(socket: TcpStream, db: Db) -> Result<()> {
    use mini_projects::Connection;
    use mini_redis::{Command, Frame};

    // `Connection` 对于 redis 的读写进行了抽象封装，因此我们读到的是一个一个数据帧frame(数据帧 = redis命令 + 数据)，而不是字节流
    // `Connection` 是在 mini-redis 中定义
    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await? {
        println!("Got frame: {:?}", frame);

        let cmd = Command::from_frame(frame)?;
        println!("cmd: {:?}", cmd);

        let response = match cmd {
            Command::Get(cmd) => {
                let db = db.lock().unwrap();
                if let Some(value) = db.get(cmd.key()) {
                    Frame::Bulk(value.clone())
                } else {
                    Frame::Null
                }
            }
            Command::Set(cmd) => {
                let mut db = db.lock().unwrap();
                db.insert(cmd.key().to_string(), cmd.value().clone());
                Frame::Simple("OK".to_string())
            }
            cmd => panic!("unimplemented {:?}", cmd),
        };
        connection.write_frame(&response).await?;
    }

    Ok(())
}
