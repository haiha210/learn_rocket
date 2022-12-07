use rocket::get;

#[get("/health")]
pub fn healthz() -> String {
  "Ok".to_string()
}
