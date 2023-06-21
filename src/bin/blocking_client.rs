use bytes::Bytes;
use mini_redis::client;
use mini_redis::Result;
use tokio::{net::ToSocketAddrs, runtime::Runtime};

pub struct BlockingClient {
    // 之前实现的异步客户端
    inner: client::Client,

    // 一个 tokio 运行时
    rt: Runtime,
}

pub fn connect<T: ToSocketAddrs>(addr: T) -> Result<BlockingClient> {
    // 构建一个 tokio 运行时： Runtime
    // 由于 current_thread 运行时并不生成新的线程，只是运行在已有的主线程上，
    // 因此只有当 block_on 被调用后，该运行时才能执行相应的操作。
    // 一旦 block_on 返回，那运行时上所有生成的任务将再次冻结，直到 block_on 的再次调用。
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    // 使用运行时来调用异步的连接方法
    let inner = rt.block_on(client::connect(addr))?;

    Ok(BlockingClient { inner, rt })
}

impl BlockingClient {
    pub fn get(&mut self, key: &str) -> Result<Option<Bytes>> {
        self.rt.block_on(self.inner.get(key))
    }

    pub fn set(&mut self, key: &str, value: Bytes) -> Result<()> {
        self.rt.block_on(self.inner.set(key, value))
    }
}

fn main() {
    let mut client = connect("127.0.0.1:6379").unwrap();

    client.set("key1", "value1".into()).unwrap();
    let res = client.get("key1").unwrap().unwrap();
    println!("Got: {:?}", res);
    assert_eq!(res, "value1");
}
