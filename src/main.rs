use learn_rocket;
use rocket;

#[rocket::main]
async fn main() {
    let launch_result = learn_rocket::rocket().launch().await;

    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {}", err),
    };
}
