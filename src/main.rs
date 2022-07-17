#[macro_use]
extern crate rocket;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let _ = rocket::build().mount("/", routes![index]).launch().await?;
    Ok(())
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}
