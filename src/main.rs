#[macro_use]
extern crate rocket;

pub mod models;
mod post_listener;

use models::Meta;
use rocket::fairing::{self, Fairing};
use rocket::serde::json::Json;

use rocket::{Build, Rocket};
use rocket_db_pools::sqlx::Row;
use rocket_db_pools::{sqlx, Connection, Database};

#[derive(Database)]
#[database("bluesky_comments")]
struct Comments(sqlx::PgPool);

struct ListenerFairing;

#[rocket::async_trait]
impl Fairing for ListenerFairing {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Jetstream listener",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        let pool = match Comments::fetch(&rocket) {
            Some(pool) => pool.0.clone(),
            None => return Err(rocket),
        };
        rocket::tokio::task::spawn(post_listener::websocket_listener(pool));
        Ok(rocket)
    }
}

#[get("/")]
fn index() -> &'static str {
    "at-comments database API server"
}

#[get("/<slug>")]
async fn post_meta(mut db: Connection<Comments>, slug: &str) -> Option<Json<Meta>> {
    sqlx::query("SELECT * FROM posts WHERE slug = $1")
        .bind(slug)
        .fetch_one(&mut **db)
        .await
        .map(|row| {
            let meta = Meta {
                id: row.get(0),
                slug: row.get(1),
                rkey: row.get(2),
                time_us: row.get(3),
            };
            Json(meta)
        })
        .ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Comments::init()) // init the database
        .attach(ListenerFairing)
        .mount("/", routes![index, post_meta])
}
