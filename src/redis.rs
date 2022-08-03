extern crate redis;
use crate::pingresult::PingResult;
use crate::targetaddr::TargetAddr;
use redis::Commands;
use std::error::Error;

pub fn put_to_redis(
    addr: &TargetAddr,
    result: &PingResult,
    expire_secs: usize,
) -> Result<(), Box<dyn Error>> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_connection()?;

    let key = addr.to_host_port();

    let value = serde_json::to_string(&result)?;

    conn.set/*::<_, String, _>*/(&key, &value)?;
    conn.expire(&key, expire_secs)?;

    Ok(())
}

pub fn get_from_redis(addr: &TargetAddr) -> Result<PingResult, Box<dyn Error>> {
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let mut conn = client.get_connection()?;

    let key = addr.to_host_port();

    // let v : String = conn.get(key)?;
    let v = conn.get::<_, String>(&key)?;
    let v = serde_json::from_str::<PingResult>(&v)?;
    Ok(v)
}

#[test]
fn test_redis_addr() {
    use std::time::Duration;

    let addr = TargetAddr::new("www.baidu.com", 80);

    let result = PingResult::new(&addr, true, &Duration::from_secs(1));

    let mut result2 = get_from_redis(&addr);
    if let Err(_) = result2 {
        let r = put_to_redis(&addr, &result, 20);
        assert!(r.is_ok());
        result2 = get_from_redis(&addr);
    }
    assert_eq!(result2.unwrap(), result);
}
