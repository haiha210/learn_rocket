use rocket::get;

#[get("/users")]
pub fn index() -> String {
  "Ok".to_string()
}
