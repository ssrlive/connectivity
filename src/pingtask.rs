use crate::targetaddr::TargetAddr;

#[derive(Debug, Clone)]
pub enum PingTask {
    #[allow(dead_code)]
    Ping(TargetAddr),
    PingFromChina(TargetAddr),
    Terminate,
}
