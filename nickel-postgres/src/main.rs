#![feature(phase)]

#[phase(plugin)]
extern crate lazy_static;

extern crate nickel;
extern crate postgres;
extern crate time;

use std::io::net::ip::Ipv4Addr;
use nickel::{ Nickel, Request, Response };
use postgres::{PostgresConnection, NoSsl};
use postgres::pool::{PostgresConnectionPool};
//use postgres::types::ToSql;
use time::Timespec;

lazy_static! {
    //TODO find if there is a way to avoid a global/ static initialisation of
    //the database connection pool
    static ref CONN_POOL: PostgresConnectionPool =
        PostgresConnectionPool::new("postgres://postgres:postgres@localhost", NoSsl, 5).unwrap();
}

fn main() {

    let mut server = Nickel::new();

    let pool_instance = CONN_POOL.clone();
    pool_instance.get_connection().execute("CREATE TABLE IF NOT EXISTS person (
            id          SERIAL PRIMARY KEY,
            name        VARCHAR NOT NULL,
            created     TIMESTAMP NOT NULL
        )", []).unwrap();

    fn get_person (req: &Request, response: &mut Response) -> () {
        //TODO find a less verbose way to extract an i32 parameter from request
        let idStr: Option<int> = from_str(req.params.get(&"id".to_string()).as_slice());
        let id: i32 = idStr.unwrap() as i32;
        println!("get_person id={}", id);
        let pool_instance = CONN_POOL.clone();
        //TODO figure out way to solit creation of statement nd calling query of statement
        //into two separate lines of code for readability,
        //without getting "error: borrowed value does not live long enough"
        for select in pool_instance.get_connection().prepare(
            "SELECT id, name, created FROM person WHERE id = $1").unwrap().query(
            [&id]).unwrap() {
            let resultId : i32 = select.get(0u);
            let resultName : String = select.get(1u);
            let resultCreated : Timespec = select.get(2u);
            //TODO serialise to proper JSON string
            let text = format!("id: {} name: {} created: {}",
                resultId, resultName, resultCreated);
            response.send(text.as_slice());
            break; //shortcut to safety - we only ever want the first row
            //TODO is there a better way to do queries where we only expect one row?
        }
    }

    fn post_person(req: &Request, response: &mut Response) -> () {
        let name = req.params.get(&"name".to_string());
        let pool_instance = CONN_POOL.clone();
        let inserts = pool_instance.get_connection().execute(
            "INSERT INTO person (name, created) VALUES ( $1, $2 )",
            [&name.as_slice(), &time::get_time()]).unwrap();
        let text = format!("{} users were inserted", inserts);
        response.send(text.as_slice());
    }

    //Routing
    //TODO switch to proper RESTful HTTP verbs
    server.get("/api/person/get/:id", get_person);
    server.get("/api/person/post/:name", post_person);

    server.listen(Ipv4Addr(0,0,0,0), 4321);
}
