#![feature(phase)]

extern crate nickel;
extern crate postgres;
extern crate time;

use std::io::net::ip::Ipv4Addr;
use nickel::{ Nickel, Request, Response };
use postgres::pool::{ PostgresConnectionPool, PooledPostgresConnection };
use time::Timespec;

use postgres_middleware::PostgresMiddleware;
mod postgres_middleware;

fn main() {

    fn initialise_db_tables (db_pool_instance: PostgresConnectionPool) {
        //initialise database tables, if has not already been done
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
        //TODO figure out way to solit creation of statement and calling query of statement
        //into two separate lines of code for readability,
        //without getting "error: borrowed value does not live long enough"
        for select in req.map.find::<PooledPostgresConnection>().unwrap().prepare(
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
        let db_conn = req.map.find::<PooledPostgresConnection>().unwrap();
        let inserts = db_conn.execute(
            "INSERT INTO person (name, created) VALUES ( $1, $2 )",
            [&name.as_slice(), &time::get_time()]).unwrap();
        let text = format!("{} users were inserted", inserts);
        response.send(text.as_slice());
    }

    let mut server = Nickel::new();
    let postgres_middleware: PostgresMiddleware = PostgresMiddleware::new();
    initialise_db_tables(postgres_middleware.pool.clone());
    server.utilize(postgres_middleware);

    //Routing
    //TODO switch to proper RESTful HTTP verbs
    server.get("/api/person/get/:id", get_person);
    server.get("/api/person/post/:name", post_person);

    server.listen(Ipv4Addr(0,0,0,0), 4321);
    println!("Listening on port");
}
