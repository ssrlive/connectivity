#[macro_use]
extern crate rocket;
use connectivity::{pingfromchina, redis, PingResult, PingTask, TargetAddr};
use rocket::figment::value::{Num, Value};
use rocket::{serde::json::Json, Request, State};
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
        while let Some(task) = rx.recv().await {
            if let PingTask::Terminate = task {
                break;
            }
            if let PingTask::PingFromChina(addr) = task {
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

    {
        let v = redis::get_from_redis(&target);
        if let Ok(v) = v {
            return Json(v);
        }
    }

    let sender = state.sender.clone();
    sender
        .send(PingTask::PingFromChina(target.clone()))
        .await
        .unwrap();

    let start = Instant::now();

    let result = pingfromchina::ping_from_china(host, port).await;

    Json(PingResult::new(&target, result, &start.elapsed()))
}
