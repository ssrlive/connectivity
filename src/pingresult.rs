use crate::targetaddr::TargetAddr;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
pub struct PingResult {
    pub target: TargetAddr,
    pub result: bool,
    pub duration_secs: u64,
}

impl PingResult {
    pub fn new(target: &TargetAddr, result: bool, duration: &Duration) -> Self {
        Self {
            target: target.clone(),
            result,
            duration_secs: duration.as_secs(),
        }
    }
}

#[test]
fn test_china_result() {
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(crate = "rocket::serde")]
    struct ChinazResult {
        status: u32,
        msg: String,
    }

    let text = "{\"status\":1,\"msg\":\"开启\"}";

    let r = serde_json::from_str::<ChinazResult>(&text).unwrap();
    assert_ne!(r.status, 0);
    println!("{:?}", r.msg);
}
