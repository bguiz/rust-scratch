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

use nickel_postgres::{ PostgresMiddleware, PostgresRequestExtensions };
use models::{ ServerMessage, GetPersonsMessage };
use models::{Person, PersonByPost, PersonByPut };

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
    //TODO cannot use static files middleware because of middleware chaining bug
    //See: https://github.com/nickel-org/nickel.rs/issues/59
    server.utilize(Nickel::static_files("client/dist/"));

    //Routing
    server.get("/api/person/:ids", get_persons);
    server.post("/api/person", post_person);
    server.put("/api/person", put_person);
    server.delete("/api/person/:id", delete_person);

    println!("Listening on port {}", port);
    server.listen(Ipv4Addr(0,0,0,0), port);
}

//initialise database tables, if has not already been done
fn initialise_db_tables (db_pool_instance: PostgresConnectionPool) {
    let db_conn = db_pool_instance.get_connection();
    db_conn.execute("CREATE TABLE IF NOT EXISTS person (
            id          SERIAL PRIMARY KEY,
            name        VARCHAR NOT NULL,
            created     TIMESTAMP NOT NULL
        )", []).unwrap();
    db_conn.execute("CREATE TABLE IF NOT EXISTS post (
            id          SERIAL PRIMARY KEY,
            title       VARCHAR NOT NULL,
            text        VARCHAR NOT NULL
        )", []).unwrap();
    db_conn.execute("CREATE TABLE IF NOT EXISTS comment (
            id          SERIAL PRIMARY KEY,
            text        VARCHAR NOT NULL,
            post_id  SERIAL REFERENCES post (id)
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

#[test]
fn test_get_sole_iterable() {
    let input: Vec<int> = vec![1 as int];
    let (val, count) = get_sole_iterable(&mut input.iter());
    assert_eq!(count, 1);
    assert_eq!(*(val.unwrap()), 1);

    let input: Vec<int> = vec![1 as int, 2 as int];
    let (val, count) = get_sole_iterable(&mut input.iter());
    assert_eq!(count, 2);
    match val {
        None => { assert!(true); },
        Some(_) => { assert!(false); }
    }

    let input: Vec<int> = vec![];
    let (val, count) = get_sole_iterable(&mut input.iter());
    assert_eq!(count, 0);
    match val {
        None => { assert!(true); },
        Some(_) => { assert!(false); }
    }
}

fn set_response_content_type_json(response: &mut Response) {
    response.origin.headers.content_type =
        Some(response.origin.headers.content_type.clone().unwrap_or(
            http::headers::content_type::MediaType {
                type_: String::from_str("application"),
                subtype: String::from_str("json"),
                parameters: Vec::new()
            })
        );
}

//TODO find way to create a mock Response object for testing
//#[test]
// fn test_set_response_content_type_json() {
//     let mut resp = Response::new();
//     set_response_content_type_json(&mut resp);
// }

fn get_comma_seperated_ids(input: &str) -> Vec<i32> {
    let strs = input.as_slice().split_str(",");
    let ids: Vec<i32> = strs.filter_map(from_str::<i32>).collect();
    ids
}

#[test]
fn test_get_comma_separated_ids() {
    assert_eq!(get_comma_seperated_ids("123,4,56"), vec![123,4,56]);
    assert_eq!(get_comma_seperated_ids("1111"), vec![1111]);
    //negative numbers are allowed
    assert_eq!(get_comma_seperated_ids("123,-5,56"), vec![123,-5,56]);
    //non-numbers are not allowed
    assert_eq!(get_comma_seperated_ids("123,f,56"), vec![123,56]);
    //whitespace is not allowed
    assert_eq!(get_comma_seperated_ids("123, 4,56"), vec![123,56]);
}

fn get_persons (req: & Request, response: &mut Response) {
    set_response_content_type_json(response);
    //TODO find a less verbose way to extract an i32 parameter from request
    let idsStr: &String = (req.params.get(&"ids".to_string()));
    let ids = get_comma_seperated_ids(idsStr.as_slice());
    println!("get_persons 1 ids={}", ids);

    let db_conn = req.db_conn();
    let stmt = db_conn.prepare(
        "SELECT id, name, created FROM person
        WHERE id = ANY( $1 )").unwrap();
    let idOptions: Vec<Option<i32>> = ids.iter().map( |i| Some(*i) ).collect();
    let idsForDb: postgres::types::array::ArrayBase<Option<i32>> =
        postgres::types::array::ArrayBase::from_vec(idOptions, 0);
    let mut iter = stmt.query(
        [&idsForDb]).unwrap();
    let mut persons: Vec<Person> = Vec::new();
    for select in iter {
        let person = Person {
            id: select.get(0u),
            name: select.get(1u),
            created: select.get(2u)
        };
        persons.push(person);
    }
    let num_persons = persons.len();
    if num_persons == 0 {
        response.origin.status = http::status::NotFound;
        let result = ServerMessage {
            message: format!("{} persons were selected", num_persons)
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
    }
    else {
        response.origin.status = http::status::Ok;
        let result = GetPersonsMessage {
            persons: persons
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
    }
}

fn get_person (req: &Request, response: &mut Response) {
    set_response_content_type_json(response);
    //TODO find a less verbose way to extract an i32 parameter from request
    let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
    let id: i32 = idStr.unwrap() as i32;
    println!("get_person id={}", id);
    let db_conn = req.db_conn();
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
    let db_conn = req.db_conn();
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
    let db_conn = req.db_conn();
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
    let db_conn = req.db_conn();
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
