extern crate nickel;
extern crate postgres;

use nickel::{ Request, Response, Middleware, Action, Continue };
use postgres::{ NoSsl };
use postgres::pool::{ PostgresConnectionPool };

#[deriving(Clone)]
pub struct PostgresMiddleware {
    pub pool: PostgresConnectionPool
}

impl PostgresMiddleware {
    pub fn new () -> PostgresMiddleware {
        PostgresMiddleware {
            pool: PostgresConnectionPool::new("postgres://postgres:postgres@localhost", NoSsl, 5).unwrap()
        }
    }
}

impl Middleware for PostgresMiddleware {
    fn invoke (&self, req: &mut Request, _resp: &mut Response) -> Action {
        req.map.insert(self.pool.clone().get_connection());
        //NOTE not Action::Continue, like it should be intuitively - see https://github.com/rust-lang/rust/issues/10090
        nickel::Continue
    }
}
