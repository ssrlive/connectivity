#[macro_use]
extern crate rocket;
use port_check::is_port_reachable_with_timeout;
use rocket::{serde::json::Json, Request};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = rocket::build()
        .register("/", catchers![not_found])
        .mount("/", routes![index, ping, ping_from_china])
        .launch()
        .await?;
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct TargetAddr {
    host: String,
    port: u16,
}

impl TargetAddr {
    fn to_host_port(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    fn is_reachable(&self) -> bool {
        is_port_reachable_with_timeout(self.to_host_port(), Duration::from_millis(10_000))
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct PingResult {
    target: TargetAddr,
    result: bool,
    duration_secs: u64,
}

#[get("/ping?<host>&<port>")]
async fn ping(host: &str, port: u16) -> Json<PingResult> {
    let target = TargetAddr {
        host: host.to_string(),
        port,
    };
    let start = Instant::now();
    let result = target.is_reachable();
    let duration_secs = start.elapsed().as_secs();
    Json(PingResult {
        target,
        result,
        duration_secs,
    })
}

#[get("/pingfromchina?<host>&<port>")]
async fn ping_from_china(host: &str, port: u16) -> Json<PingResult> {
    let target = TargetAddr {
        host: host.to_string(),
        port,
    };
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
            if let Some(val) = value.attr("id") {
                if val == "encode" {
                    if let Some(val) = value.attr("value") {
                        encode = Some(val.to_string());
                        break;
                    }
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
    let duration_secs = start.elapsed().as_secs();
    Json(PingResult {
        target,
        result,
        duration_secs,
    })
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
