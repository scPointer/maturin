#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{get_time, yield_};

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 1000;
    loop {
        let now = get_time();
        println!("timer {} now {} wait {}", current_timer, now, wait_for);
        if now < wait_for {
            yield_();
        } else {
            break;
        }
    }
    println!("Test sleep OK!");
    0
}
