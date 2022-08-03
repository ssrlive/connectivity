#[macro_use]
extern crate rocket;
use connectivity::{pingfromchina, redis, PingResult, PingTask, TargetAddr};
use rocket::figment::value::{Num, Value};
use rocket::{serde::json::Json, Request, State};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::{
    sync::mpsc::{self, Sender},
    time,
};

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
    let survival_time: Duration;
    {
        let config = rt_env.figment().clone();
        let config = config.select("my_settings");
        let v = config
            .find_value("ping_timeout_secs")
            .unwrap_or_else(|_| Value::from(5));

        let mut n0: u64 = 5;
        if let Value::Num(_, Num::I64(n)) = v {
            n0 = n as u64;
        }
        ping_timeout = Duration::from_secs(n0);

        let v2 = config
            .find_value("survival_time_secs")
            .unwrap_or_else(|_| Value::from(3600));
        let mut n2: u64 = 3600;
        if let Value::Num(_, Num::I64(n)) = v2 {
            n2 = n as u64;
        }
        survival_time = Duration::from_secs(n2);
    }

    let (tx, mut rx) = mpsc::channel::<PingTask>(1000);
    let tx = Arc::new(tx);
    let settings = CustomerSettings::new(ping_timeout, tx.clone());
    let handle = tokio::spawn(async move {
        while let Some(task) = rx.recv().await {
            if let PingTask::Terminate = task {
                break;
            }
            if let PingTask::PingFromChina(addr) = task {
                time::sleep(Duration::from_secs(2)).await; // wait for 2 seconds
                let start = Instant::now();
                let b = pingfromchina::ping_from_china(&addr.host, addr.port).await;
                let result = PingResult::new(&addr, b, &start.elapsed());
                if let Err(r) = redis::put_to_redis(&addr, &result, &survival_time) {
                    print!("{}", r);
                }
            }
        }
        println!("Worker thread is shutting down");
    });

    let _ = rt_env
        .manage(settings)
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

    Json(PingResult::new(&target, false, &Duration::from_secs(1)))
}
