#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket_contrib;

use diesel::SqliteConnection;
use rocket::fairing::AdHoc;
use rocket::Rocket;
use rocket_contrib::json::{Json, JsonValue};

use users::{NewUser, User};

// This macro from `diesel_migrations` defines an `embedded_migrations` module
// containing a function named `run`. This allows the example to be run and
// tested without any outside setup of the database.
embed_migrations!();

#[database("sqlite_database")]
pub struct DbConn(SqliteConnection);

#[post("/", format = "json", data = "<new_user>")]
fn new(new_user: Json<NewUser>, conn: DbConn) -> JsonValue {
    if User::insert(new_user.0, &conn) {
        json!({"status": "ok"})
    } else {
        json!({"status": "error"})
    }
}

#[get("/<username>", format = "json")]
fn find(username: String, conn: DbConn) -> Option<Json<User>> {
    User::find(username, &conn).map(Json)
}

fn run_db_migrations(rocket: Rocket) -> Result<Rocket, Rocket> {
    let conn = DbConn::get_one(&rocket).expect("database connection");
    match embedded_migrations::run(&*conn) {
        Ok(()) => Ok(rocket),
        Err(e) => {
            eprintln!("Failed to run database migrations: {:?}", e);
            Err(rocket)
        }
    }
}

fn main() {
    rocket::ignite()
        .attach(DbConn::fairing())
        .attach(AdHoc::on_attach("Database Migrations", run_db_migrations))
        .mount("/users", routes![new, find])
        .launch();
}

mod users {
    use diesel::{self, prelude::*};

    mod schema {
        table! {
            users {
                id -> Nullable<Integer>,
                username -> Text,
                email -> Text,
            }
        }
    }

    use self::schema::users;

    #[derive(Serialize, Queryable, Insertable, Debug, Clone)]
    #[table_name = "users"]
    pub struct User {
        pub id: Option<i32>,
        pub username: String,
        pub email: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct NewUser {
        pub username: String,
        pub email: String,
    }

    impl User {
        pub fn insert(new_user: NewUser, conn: &SqliteConnection) -> bool {
            let t = User {
                id: None,
                username: new_user.username,
                email: new_user.email,
            };
            diesel::insert_into(users::table)
                .values(&t)
                .execute(conn)
                .is_ok()
        }

        pub fn find(username_to_find: String, conn: &SqliteConnection) -> Option<User> {
            use self::schema::users::dsl::*;
            users
                .filter(username.eq(username_to_find))
                .first::<User>(conn)
                .ok()
        }
    }
}
