#[macro_use]
extern crate rocket;
use connectivity::{pingfromchina, redis, PingResult, PingTask, TargetAddr};
use rocket::figment::{
    value::{Num, Value},
    Figment,
};
use rocket::{serde::json::Json, Request, State};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::{
    sync::{
        mpsc::{self, Sender},
        Mutex, Notify,
    },
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
    let request_interval: Duration;
    let anti_banned_as_robot: bool;
    {
        let config = rt_env.figment().clone();
        let config = config.select("my_settings");

        let n0 = config_get_value_of_key(&config, "ping_timeout_secs", 5_u64);
        ping_timeout = Duration::from_secs(n0);

        let n2 = config_get_value_of_key(&config, "survival_time_secs", 3600_u64);
        survival_time = Duration::from_secs(n2);

        let n3 = config_get_value_of_key(&config, "request_interval_secs", 1_u64);
        request_interval = Duration::from_secs(n3);

        anti_banned_as_robot = config_get_bool_value_of_key(&config, "anti_banned_as_robot", true);
    }

    let (tx, mut rx) = mpsc::channel::<PingTask>(1000);
    let tx = Arc::new(tx);
    let settings = CustomerSettings::new(ping_timeout, tx.clone());
    let handle = tokio::spawn(async move {
        while let Some(task) = rx.recv().await {
            if let PingTask::Terminate = task {
                break;
            }
            if let PingTask::PingFromChina((v, notify)) = task {
                let start = Instant::now();
                let mut result = v.lock().await;
                let addr = result.target.clone();
                if let Ok(b) = pingfromchina::ping_from_china(&addr.host, addr.port).await {
                    result.result = b;
                    result.duration_secs = start.elapsed().as_secs();
                    if let Err(r) = redis::put_to_redis(&addr, &result, &survival_time).await {
                        println!("{:?}", r);
                    }
                }
                if anti_banned_as_robot {
                    time::sleep(request_interval).await;
                }
                notify.notify_one();
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

fn config_get_value_of_key(config: &Figment, key: &str, default: u64) -> u64 {
    let v = config
        .find_value(key)
        .unwrap_or_else(|_| Value::from(default as i64));

    let mut n0: u64 = default;
    if let Value::Num(_, Num::I64(n)) = v {
        n0 = n as u64;
    }
    n0
}

fn config_get_bool_value_of_key(config: &Figment, key: &str, default: bool) -> bool {
    let v = config
        .find_value(key)
        .unwrap_or_else(|_| Value::from(default));

    let mut n0: bool = default;
    if let Value::Bool(_, n) = v {
        n0 = n;
    }
    n0
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

    if let Ok(v) = redis::get_from_redis(&target).await {
        return Json(v);
    }

    let notify = Arc::new(Notify::new());
    let v = Arc::new(Mutex::new(PingResult::new(
        &target,
        false,
        &Duration::from_secs(1),
    )));

    let sender = &state.sender;
    sender
        .send(PingTask::PingFromChina((v.clone(), notify.clone())))
        .await
        .unwrap();

    notify.notified().await;

    let v = v.lock().await;

    Json(v.clone())
}
