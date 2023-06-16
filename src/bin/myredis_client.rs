use bytes::Bytes;
use mini_redis::{client, Result};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Set {
        key: String,
        value: Bytes,
        responsor: Responsor<()>,
    },
    Get {
        key: String,
        responsor: Responsor<Option<Bytes>>,
    },
}

// 这是一个发送器，用于将管理任务中从服务端获取到的命令执行结果返回给发送命令的任务
type Responsor<T> = oneshot::Sender<Result<T>>;

#[tokio::main]
async fn main() {
    // 使用消息通道 mpsc 发送命令给管理连接的任务，管理任务收到命令后，再调用 mini-redis 客户端发送命令给服务端，并返回结果
    // 创建消息通道
    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    // 发送命令的任务: mpsc 支持多发送方，这里一个用于发送 set 命令，一个用于发送 get 命令
    let t1 = tokio::spawn(async move {
        let (res_tx, res_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "key2".to_string(),
            value: "222".into(),
            responsor: res_tx,
        };
        // 通过mpsc通道发送命令给管理任务
        tx.send(cmd).await.unwrap();

        // 通过oneshot通道接收命令执行结果
        let res = res_rx.await.unwrap();
        println!("Set cmd result: {:?}", res);
    });

    let t2 = tokio::spawn(async move {
        let (res_tx, res_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "key".to_string(),
            responsor: res_tx,
        };
        tx2.send(cmd).await.unwrap();

        let res = res_rx.await.unwrap();
        println!("Get cmd result: {:?}", res);
    });

    // 管理连接的任务
    let manager = tokio::spawn(async move {
        let mut client = client::connect("127.0.0.1:6379").await.unwrap();

        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Set {
                    key,
                    value,
                    responsor,
                } => {
                    let res = client.set(&key, value).await;
                    // 返回命令执行结果给发送命令的任务；忽略错误
                    let _ = responsor.send(res);
                }
                Command::Get { key, responsor } => {
                    let res = client.get(&key).await;
                    let _ = responsor.send(res);
                }
            }
        }
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
}
