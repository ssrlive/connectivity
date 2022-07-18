#[macro_use]
extern crate rocket;
use port_check::is_port_reachable_with_timeout;
use rocket::{serde::json::Json, Request};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = rocket::build()
        .register("/", catchers![not_found])
        .mount("/", routes![index, ping])
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
}

#[get("/ping?<host>&<port>")]
async fn ping(host: &str, port: u16) -> Json<PingResult> {
    let target = TargetAddr {
        host: host.to_string(),
        port,
    };
    let result = target.is_reachable();
    Json(PingResult { target, result })
}
