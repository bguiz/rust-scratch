extern crate serialize;

use serialize::{ Decodable, Encodable };
use time::Timespec;

#[deriving(Encodable)]
pub struct GetPersonsMessage {
    pub persons: Vec<Person>
}

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

#[deriving(Decodable,Encodable)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub text: String
}

#[deriving(Decodable,Encodable)]
pub struct Comment {
    pub id: i32,
    pub text: String,
    pub post_id: String
}
