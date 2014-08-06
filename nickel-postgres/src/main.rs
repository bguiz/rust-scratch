#![feature(phase)]

extern crate http;
extern crate nickel;
extern crate postgres;
extern crate time;
extern crate serialize;
extern crate nickel_postgres;

use std::io::net::ip::Ipv4Addr;
use nickel::{ Nickel, Request, Response };
use postgres::pool::{ PostgresConnectionPool, PooledPostgresConnection };

use serialize::json;

use nickel_postgres::PostgresMiddleware;
use models::{ ServerMessage, Person, PersonByPost, PersonByPut };

mod models;

fn main() {
    //Initialise server instance including all middleware
    let mut server = Nickel::new();
    let port: u16 = 4321;
    let postgres_middleware: PostgresMiddleware = PostgresMiddleware::new(
        "postgres://postgres:postgres@localhost", postgres::NoSsl, 5);
    initialise_db_tables(postgres_middleware.pool.clone());
    server.utilize(postgres_middleware);
    server.utilize(Nickel::json_body_parser());
    server.utilize(Nickel::static_files("client/dist/"));

    //Routing
    //TODO switch to proper RESTful HTTP verbs
    server.get("/api/person/:id", get_person);
    server.post("/api/person", post_person);
    server.put("/api/person", put_person);
    //NOTE See https://github.com/nickel-org/nickel.rs/pull/51
    server.delete("/api/person/:id", delete_person);

    println!("Listening on port {}", port);
    server.listen(Ipv4Addr(0,0,0,0), port);
}

//initialise database tables, if has not already been done
fn initialise_db_tables (db_pool_instance: PostgresConnectionPool) {
    db_pool_instance.get_connection().execute("CREATE TABLE IF NOT EXISTS person (
            id          SERIAL PRIMARY KEY,
            name        VARCHAR NOT NULL,
            created     TIMESTAMP NOT NULL
        )", []).unwrap();
}

fn get_sole_iterable<T, I: Iterator<T>>(iter: &mut I) -> (Option<T>, uint) {
    //NOTE thanks, @sfackler
    match (iter.next(), iter.next()) {
        (None, _) => (None, 0),
        (Some(result), None) => (Some(result), 1),
        _ => (None, (iter.count() + 2))
    }
}

fn set_response_content_type_json(response: &mut Response) {
    let has_set_content_type: bool = match response.origin.headers.content_type {
        None => false,
        Some(_) => true,
    };
    if !has_set_content_type {
        response.origin.headers.content_type = Some(http::headers::content_type::MediaType {
            type_: String::from_str("application"),
            subtype: String::from_str("json"),
            parameters: Vec::new()
        });
    }
}

fn get_person (req: &Request, response: &mut Response) {
    set_response_content_type_json(response);
    //TODO find a less verbose way to extract an i32 parameter from request
    let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
    let id: i32 = idStr.unwrap() as i32;
    println!("get_person id={}", id);
    let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
    let stmt = db_conn.prepare(
        "SELECT id, name, created FROM person WHERE id = $1").unwrap();
    let mut iter = stmt.query(
        [&id]).unwrap();

    let (maybeResult, selects) =
        get_sole_iterable(&mut iter);
    if selects == 1 {
        let select = maybeResult.unwrap();
        let result = Person {
            id: select.get(0u),
            name: select.get(1u),
            created: select.get(2u),
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
    }
    else {
        let result = ServerMessage {
            message: format!("{} persons were selected", selects)
        };
        let text = json::encode(&result);
        if selects == 0 {
            response.origin.status = http::status::NotFound;
        }
        else if selects > 1 {
            response.origin.status = http::status::InternalServerError;
        }
        response.send(text.as_slice());
    }
}

fn post_person(req: &Request, response: &mut Response) {
    set_response_content_type_json(response);
    println!("post_person called");
    let person: PersonByPost = req.json_as::<PersonByPost>().unwrap();
    let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
    let inserts = db_conn.execute(
        "INSERT INTO person (name, created) VALUES ( $1, $2 )",
        [&person.name.as_slice(), &time::get_time()]).unwrap();
    if inserts == 0 {
        response.origin.status = http::status::NotFound;
    }
    else if inserts > 1 {
        response.origin.status = http::status::InternalServerError;
    }
    let result = ServerMessage {
        message: format!("{} persons were inserted", inserts)
    };
    let text = json::encode(&result);
    response.send(text.as_slice());
    //TODO error checking to ensure that JSON decode succeeded
}

fn put_person(req: &Request, response: &mut Response) {
    set_response_content_type_json(response);
    println!("put_person called");
    let person: PersonByPut = req.json_as::<PersonByPut>().unwrap();
    let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
    let updates = db_conn.execute(
        "UPDATE person SET ( name ) = ( $2 ) WHERE id = $1",
        [&person.id, &person.name.as_slice()]).unwrap();
    if updates == 0 {
        response.origin.status = http::status::NotFound;
    }
    else if updates > 1 {
        response.origin.status = http::status::InternalServerError;
    }
    let result = ServerMessage {
        message: format!("{} persons were updated", updates)
    };
    let text = json::encode(&result);
    response.send(text.as_slice());
    //TODO error checking top ensure that JSON decode succeeded
}

fn delete_person (req: &Request, response: &mut Response) {
    set_response_content_type_json(response);
    let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
    let id: i32 = idStr.unwrap() as i32;
    println!("delete_person id={}", id);
    let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
    let deletes = db_conn.execute(
        "DELETE FROM person WHERE id = $1",
        [&id]).unwrap();
    if deletes == 0 {
        response.origin.status = http::status::NotFound;
    }
    else if deletes > 1 {
        response.origin.status = http::status::InternalServerError;
    }
    let result = ServerMessage {
        message: format!("{} persons were deleted", deletes)
    };
    let text = json::encode(&result);
    response.send(text.as_slice());
    //TODO error checking to ensure that JSON decode succeeded
}
