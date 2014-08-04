extern crate serialize;

use serialize::{ Decodable, Encodable };
use time::Timespec;

#[deriving(Decodable,Encodable)]
pub struct Person {
    pub id: i32,
    pub name: String,
    pub created: Timespec
}

#[deriving(Decodable)]
pub struct PersonByPost {
    pub name: String
}

#[deriving(Decodable)]
pub struct PersonByPut {
    pub id: i32,
    pub name: String
}

#[deriving(Encodable)]
pub struct ServerMessage {
    pub message: String
}
