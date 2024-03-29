extern crate redis;
use crate::pingresult::PingResult;
use crate::targetaddr::TargetAddr;
use redis::AsyncCommands;
use std::error::Error;
use std::time::Duration;

pub async fn put_to_redis(
    addr: &TargetAddr,
    result: &PingResult,
    expire: &Duration,
) -> Result<(), Box<dyn Error>> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_async_connection().await?;

    let key = addr.to_host_port();

    let value = serde_json::to_string(&result)?;

    // conn.set::<_, &String, _>(&key, &value).await?;
    conn.set(&key, &value).await?;
    conn.expire(&key, expire.as_secs() as usize).await?;

    Ok(())
}

pub async fn get_from_redis(addr: &TargetAddr) -> Result<PingResult, Box<dyn Error>> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_async_connection().await?;

    let key = addr.to_host_port();

    // let v : String = conn.get(key)?;
    let v = conn.get::<_, String>(&key).await?;
    let v = serde_json::from_str::<PingResult>(&v)?;
    Ok(v)
}

#[test]
fn test_redis_addr() {
    use std::time::Duration;
    use tokio::runtime::Runtime;

    let addr = TargetAddr::new("www.baidu.com", 80);

    let result = PingResult::new(&addr, true, &Duration::from_secs(1));

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let mut result2 = get_from_redis(&addr).await;
        if let Err(_) = result2 {
            let r = put_to_redis(&addr, &result, &Duration::from_secs(20)).await;
            assert!(r.is_ok());
            result2 = get_from_redis(&addr).await;
        }
        assert_eq!(result2.unwrap(), result);
    });
}
