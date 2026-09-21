#![allow(unused_mut)]

use std::convert::Infallible;

use valuable::Valuable;

#[derive(Debug)]
pub struct Quit;

impl From<Infallible> for Quit {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

pub struct Mapper;
include!(concat!(env!("OUT_DIR"), "/form.rs"));

#[test]
fn compiled() {}
