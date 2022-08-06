use std::sync::{Arc, Mutex};
use tokio::sync::Notify;

use crate::pingresult::PingResult;
use crate::targetaddr::TargetAddr;

#[derive(Debug, Clone)]
pub enum PingTask {
    #[allow(dead_code)]
    Ping(TargetAddr),
    PingFromChina((Arc<Mutex<PingResult>>, Arc<Notify>)),
    Terminate,
}
