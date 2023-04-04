#[macro_use]
extern crate rocket;
// use lazy_static::lazy_static;
use rocket::{
    fairing::{self, Fairing, Info, Kind},
    http::{ContentType, Header, Status},
    request::FromParam,
    response::{self, Responder},
    Build, Data, Orbit, Request, Response, Rocket, State,
};
use rocket_db_pools::{
    sqlx,
    sqlx::{FromRow, PgPool},
    Connection, Database,
};
use std::{
    // collections::HashMap,
    io::Cursor,
    sync::atomic::{AtomicU64, Ordering},
};
use uuid::Uuid;

struct VisitorCounter {
    visitor: AtomicU64,
}

impl VisitorCounter {
    fn increment_counter(&self) {
        self.visitor.fetch_add(1, Ordering::Relaxed);
        println!(
            "The number of visitor is: {}",
            self.visitor.load(Ordering::Relaxed)
        );
    }
}

#[rocket::async_trait]
impl Fairing for VisitorCounter {
    fn info(&self) -> Info {
        Info {
            name: "Visitor Counter",
            kind: Kind::Ignite | Kind::Liftoff | Kind::Request,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        println!("Setting up visitor counter");
        Ok(rocket)
    }

    async fn on_liftoff(&self, _: &Rocket<Orbit>) {
        println!("Finish setting up visitor counter");
    }

    async fn on_request(&self, _: &mut Request<'_>, _: &mut Data<'_>) {
        self.increment_counter();
    }
}

#[derive(Debug, FromRow)]
struct User {
    uuid: Uuid,
    name: String,
    age: i16,
    grade: i16,
    active: bool,
}

impl<'r> Responder<'r, 'r> for User {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        let base_response = default_response();
        let user = format!("Found user: {:?}", self);
        Response::build()
            .sized_body(user.len(), Cursor::new(user))
            .raw_header("X-USER-ID", self.uuid.to_string())
            .merge(base_response)
            .ok()
    }
}

struct NewUser(Vec<User>);

fn default_response<'r>() -> response::Response<'r> {
    Response::build()
        .header(ContentType::Plain)
        .raw_header("X-CUSTOM-ID", "CUSTOM")
        .finalize()
}

impl<'r> Responder<'r, 'r> for NewUser {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'r> {
        let user = self
            .0
            .iter()
            .map(|u| format!("{:?}", u))
            .collect::<Vec<String>>()
            .join(",");

        Response::build()
            .sized_body(user.len(), Cursor::new(user))
            .raw_header("X-CUSTOM-ID", "USERS")
            .join(default_response())
            .ok()
    }
}

// lazy_static! {
//     static ref USERS: HashMap<&'static str, User> = {
//         let mut map = HashMap::new();
//         map.insert(
//             "3e3dd4ae-3c37-40c6-aa64-7061f284ce28",
//             User {
//                 uuid: String::from("3e3dd4ae-3c37-40c6-aa64-7061f284ce28"),
//                 name: String::from("John Doe"),
//                 age: 18,
//                 grade: 1,
//                 active: true,
//             },
//         );

//         map
//     };
// }

// #[route(GET, uri = "/<name_grade>?<filters..>")]
// fn users<'a>(
//     counter: &State<VisitorCounter>,
//     name_grade: NameGrade,
//     filters: Option<Filters>,
// ) -> Result<NewUser<'a>, Status> {
//     counter.increment_counter();
//     let users: Vec<&User> = USERS
//         .values()
//         .filter(|user| user.name.contains(&name_grade.name) && user.grade == name_grade.grade)
//         .filter(|user| {
//             if let Some(fts) = &filters {
//                 user.age == fts.age && user.active == fts.active
//             } else {
//                 true
//             }
//         })
//         .collect();

//     if users.len() > 0 {
//         Ok(NewUser(users))
//     } else {
//         Err(Status::Forbidden)
//     }
// }

// #[get("/<uuid>", rank = 1)]
// fn user(uuid: &str) -> Result<&User, NotFound<&str>> {
//     let user = USERS.get(uuid);

//     user.ok_or(NotFound("User not found"))
// }

const X_TRACE_ID: &str = "X-TRACE-ID";
struct XTraceId {}

#[rocket::async_trait]
impl Fairing for XTraceId {
    fn info(&self) -> Info {
        Info {
            name: "X-TRACE-ID Injector",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _: &mut Data<'_>) {
        let header = Header::new(X_TRACE_ID, Uuid::new_v4().to_string());
        req.add_header(header);
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let header = req.headers().get_one(X_TRACE_ID).unwrap();
        res.set_header(Header::new(X_TRACE_ID, header));
    }
}

#[derive(Database)]
#[database("main_connection")]
struct DBConnection(PgPool);

#[get("/<uuid>", rank = 1)]
async fn user<'a>(mut db: Connection<DBConnection>, uuid: &str) -> Result<User, Status> {
    let parsed_uuid = Uuid::parse_str(uuid).map_err(|_| Status::BadRequest)?;

    let user = sqlx::query_as!(User, "select * from users where uuid = $1", parsed_uuid)
        .fetch_one(&mut *db)
        .await;

    // USERS.get(uuid)
    user.map_err(|_| Status::NotFound)
}

#[derive(FromForm)]
struct Filters {
    age: u8,
    active: bool,
}

struct NameGrade<'r> {
    name: &'r str,
    grade: u8,
}

impl<'r> FromParam<'r> for NameGrade<'r> {
    type Error = &'static str;
    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        const ERROR_MESSAGE: Result<NameGrade, &'static str> = Err("Error parsing user parameter");

        let name_grade_vec: Vec<&'r str> = param.split("_").collect();

        match name_grade_vec.len() {
            2 => match name_grade_vec[1].parse::<u8>() {
                Ok(n) => Ok(Self {
                    name: name_grade_vec[0],
                    grade: n,
                }),
                Err(_) => ERROR_MESSAGE,
            },
            _ => ERROR_MESSAGE,
        }
    }
}

#[route(GET, uri = "/<name_grade>?<filters..>")]
async fn users<'a>(
    mut db: Connection<DBConnection>,
    name_grade: NameGrade<'_>,
    filters: Option<Filters>,
) -> Result<NewUser, Status> {
    let mut query_str = String::from("Select * from users where name like $1 and grade = $2");
    if filters.is_none() {
        query_str.push_str(" and age $3 and active = $4");
    }

    let mut query = sqlx::query_as::<_, User>(&query_str)
        .bind(format!("%{}%", &name_grade.name))
        .bind(name_grade.grade as i16);

    if let Some(fts) = &filters {
        query = query.bind(fts.age as i16).bind(fts.active);
    }

    let unwrapped_users = query.fetch_all(&mut *db).await;
    let users: Vec<User> = unwrapped_users.map_err(|_| Status::InternalServerError)?;
    if users.is_empty() {
        Err(Status::NotFound)
    } else {
        Ok(NewUser(users))
    }
}

#[catch(404)]
fn not_found(req: &Request<'_>) -> String {
    format!("We cannot find this page {}.", req.uri())
}

#[catch(403)]
fn forbidden(req: &Request<'_>) -> String {
    format!("Access forbidden {}.", req.uri())
}

#[launch]
async fn rocket() -> Rocket<Build> {
    let visitor_counter = VisitorCounter {
        visitor: AtomicU64::new(0),
    };

    let x_trace_id = XTraceId {};

    rocket::build()
        .attach(DBConnection::init())
        .attach(visitor_counter)
        .attach(x_trace_id)
        .mount("/user", routes![user])
        .mount("/users", routes![users])
        .register("/", catchers![not_found, forbidden])
}
