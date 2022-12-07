#![feature(proc_macro_hygiene, decl_macro)]

use routes::route_api_v1;

#[macro_use]
extern crate rocket;

mod controllers;
pub mod routes;

#[launch]
pub fn rocket() -> _ {
    rocket::build()
    .mount("/", routes![controllers::health::healthz])
    .mount("/api/v1", route_api_v1())
}
