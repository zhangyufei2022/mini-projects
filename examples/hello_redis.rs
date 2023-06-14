use mini_redis::client;
use mini_redis::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;
    client.set("key", "111".into()).await?;

    let res = client.get("key").await?;

    println!("result is: {:?}", res);
    assert_eq!(res, Some("111".into()));

    Ok(())
}
