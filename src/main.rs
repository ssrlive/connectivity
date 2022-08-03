#[macro_use]
extern crate rocket;
use rocket::figment::value::{Num, Value};
use rocket::{serde::json::Json, Request, State};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Sender};

struct CustomerSettings {
    ping_timeout: Duration,
    sender: Arc<Sender<PingTask>>,
}

impl CustomerSettings {
    fn new(ping_timeout: Duration, sender: Arc<Sender<PingTask>>) -> Self {
        CustomerSettings {
            ping_timeout,
            sender,
        }
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let rt_env = rocket::build();

    let ping_timeout: Duration;
    {
        let config = rt_env.figment().clone();
        let config = config.select("my_settings");
        let v = config
            .find_value("ping_timeout_second")
            .unwrap_or_else(|_| Value::from(5));

        let mut n0: u64 = 5;
        if let Value::Num(_, Num::I64(n)) = v {
            n0 = n as u64;
        }
        ping_timeout = Duration::from_secs(n0);
    }

    let (tx, mut rx) = mpsc::channel::<PingTask>(1000);
    let tx = Arc::new(tx);
    let handle = tokio::spawn(async move {
        while let Some(obj) = rx.recv().await {
            if let PingTask::Terminate = obj {
                break;
            }
            if let PingTask::PingFromChina(addr) = obj {
                println!("{:#?}", addr);
            }
        }
        println!("Worker thread is shutting down");
    });

    let _ = rt_env
        .manage(CustomerSettings::new(ping_timeout, tx.clone()))
        .register("/", catchers![not_found])
        .mount("/", routes![index, ping, ping_from_china])
        .launch()
        .await?;

    tx.clone().send(PingTask::Terminate).await.unwrap();
    handle.await.unwrap();

    Ok(())
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Sorry, '{}' is not a valid path.", req.uri())
}

pub fn is_port_reachable_with_timeout<A: ToSocketAddrs>(address: A, timeout: Duration) -> bool {
    match address.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(address) = addrs.next() {
                if TcpStream::connect_timeout(&address, timeout).is_ok() {
                    return true;
                }
            }
            false
        }
        Err(_err) => false,
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct TargetAddr {
    host: String,
    port: u16,
}

impl TargetAddr {
    fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
        }
    }

    fn to_host_port(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn is_reachable(&self, timeout: Duration) -> bool {
        is_port_reachable_with_timeout(self.to_host_port(), timeout)
    }
}

#[derive(Debug, Clone)]
enum PingTask {
    #[allow(dead_code)]
    Ping(TargetAddr),
    PingFromChina(TargetAddr),
    Terminate,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct PingResult {
    target: TargetAddr,
    result: bool,
    duration_secs: u64,
}

impl PingResult {
    fn new(target: &TargetAddr, result: bool, duration: &Duration) -> Self {
        Self {
            target: target.clone(),
            result,
            duration_secs: duration.as_secs(),
        }
    }
}

#[get("/ping?<host>&<port>")]
async fn ping(host: &str, port: u16, state: &State<CustomerSettings>) -> Json<PingResult> {
    let target = TargetAddr::new(host, port);
    let start = Instant::now();
    let timeout = state.ping_timeout;
    let result = target.is_reachable(timeout);
    Json(PingResult::new(&target, result, &start.elapsed()))
}

#[get("/pingfromchina?<host>&<port>")]
async fn ping_from_china(
    host: &str,
    port: u16,
    state: &State<CustomerSettings>,
) -> Json<PingResult> {
    let target = TargetAddr::new(host, port);

    state
        .sender
        .clone()
        .send(PingTask::PingFromChina(target.clone()))
        .await
        .unwrap();

    let start = Instant::now();

    let url = format!("https://tool.chinaz.com/port?host={}&port={}", host, port);

    let resp = reqwest::get(url).await.unwrap();
    let text = resp.text().await.unwrap();

    let mut encode = None;
    {
        let document = Html::parse_document(&text);
        let selector = Selector::parse(r#"input"#).unwrap();
        for item in document.select(&selector) {
            let value = item.value();
            if let Some(val) = value.attr("id") && val == "encode" {
                    if let Some(val) = value.attr("value") {
                        encode = Some(val.to_string());
                        break;
                    }
            }
        }
    }
    let mut result = false;
    if let Some(encode) = encode {
        let url = "https://tool.chinaz.com/iframe.ashx?t=port";
        let map = HashMap::from([
            ("encode", encode),
            ("host", host.to_string()),
            ("port", port.to_string()),
        ]);

        let client = reqwest::Client::new();
        let resp = client.post(url).form(&map).send().await.unwrap();

        let text = resp.text().await.unwrap();
        result = text.contains("status:1");
    }
    Json(PingResult::new(&target, result, &start.elapsed()))
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
