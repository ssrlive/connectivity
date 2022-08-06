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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[serde(rename_all = "camelCase")]
pub struct ChinazResultData {
    pub is_open: bool,
    pub rcode: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ChinazResult {
    pub code: i32,
    pub msg: String,
    pub data: ChinazResultData,
}

#[test]
fn test_china_result() {
    let text = r#"{"code":1,"data":{"isOpen":true,"rcode":1},"msg":"成功"}"#;
    let r = serde_json::from_str::<ChinazResult>(&text).unwrap();
    assert_ne!(r.code, 0_i32);
    let s = serde_json::to_string(&r).unwrap();
    println!("{:?}", s);
}
