#![feature(phase)]

extern crate nickel;
extern crate postgres;
extern crate time;
extern crate serialize;

use std::io::net::ip::Ipv4Addr;
use nickel::{ Nickel, Request, Response };
use postgres::pool::{ PostgresConnectionPool, PooledPostgresConnection };

use serialize::json;

use postgres_middleware::PostgresMiddleware;
use models::{ ServerMessage, Person, PersonByPost, PersonByPut };

mod postgres_middleware;
mod models;

fn main() {

    //initialise database tables, if has not already been done
    fn initialise_db_tables (db_pool_instance: PostgresConnectionPool) {
        db_pool_instance.get_connection().execute("CREATE TABLE IF NOT EXISTS person (
                id          SERIAL PRIMARY KEY,
                name        VARCHAR NOT NULL,
                created     TIMESTAMP NOT NULL
            )", []).unwrap();
    }

    fn get_person (req: &Request, response: &mut Response) -> () {
        //TODO find a less verbose way to extract an i32 parameter from request
        let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
        let id: i32 = idStr.unwrap() as i32;
        println!("get_person id={}", id);
        let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
        let stmt = db_conn.prepare(
            "SELECT id, name, created FROM person WHERE id = $1").unwrap();
        let mut iter = stmt.query(
            [&id]).unwrap();
        let firstSelect = iter.next();
        //NOTE add 1 to take into account already having advanced by one
        let selects: uint = iter.count() + 1;
        if selects == 1 {
            let select = firstSelect.unwrap();
            let result = Person {
                id: select.get(0u),
                name: select.get(1u),
                created: select.get(2u),
            };
            let text = json::encode(&result);
            response.send(text.as_slice());
            //TODO is there a better way to do queries where we only expect one row?
        }
        else {
            let result = ServerMessage {
                message: format!("{} persons were selected", selects)
            };
            let text = json::encode(&result);
            response.send(text.as_slice());
        }
        //TODO proper HTTP error codes for not found, invalid input
    }

    fn post_person(req: &Request, response: &mut Response) -> () {
        // let name = req.params.get(&"name".to_string());
        println!("post_person called");
        let person: PersonByPost = req.json_as::<PersonByPost>().unwrap();
        let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
        let inserts = db_conn.execute(
            "INSERT INTO person (name, created) VALUES ( $1, $2 )",
            [&person.name.as_slice(), &time::get_time()]).unwrap();
        let result = ServerMessage {
            message: format!("{} persons were inserted", inserts)
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
        //TODO error checking top ensure that JSON decode succeeded
        //TODO proper HTTP error codes for not found, and invalid input
    }

    fn put_person(req: &Request, response: &mut Response) -> () {
        println!("put_person called");
        let person: PersonByPut = req.json_as::<PersonByPut>().unwrap();
        let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
        let updates = db_conn.execute(
            "UPDATE person SET ( name ) = ( $2 ) WHERE id = $1",
            [&person.id, &person.name.as_slice()]).unwrap();
        let result = ServerMessage {
            message: format!("{} persons were updated", updates)
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
        //TODO error checking top ensure that JSON decode succeeded
        //TODO proper HTTP error codes for not found, and invalid input
    }

    fn delete_person (req: &Request, response: &mut Response) -> () {
        let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
        let id: i32 = idStr.unwrap() as i32;
        println!("delete_person id={}", id);
        let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
        let deletes = db_conn.execute(
            "DELETE FROM person WHERE id = $1",
            [&id]).unwrap();
        let result = ServerMessage {
            message: format!("{} persons were deleted", deletes)
        };
        let text = json::encode(&result);
        response.send(text.as_slice());
        //TODO error checking top ensure that JSON decode succeeded
        //TODO proper HTTP error codes for not found, and invalid input
    }

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
