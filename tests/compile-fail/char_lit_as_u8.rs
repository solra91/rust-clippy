#![feature(plugin)]
#![plugin(clippy)]

#![deny(char_lit_as_u8)]
#![allow(unused_variables)]
fn main() {
    let c = 'a' as u8; //~ERROR casting character literal
}
