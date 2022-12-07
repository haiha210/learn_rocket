use rocket::Route;

use crate::controllers;

pub fn route_api_v1() -> Vec<Route> {
  routes![
    controllers::v1::users::index,
  ]
}