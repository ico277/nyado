use std::env::args;
use std::process::Command;
use users::{get_current_uid, get_user_by_uid};

fn main() {
    let mut args = args();
    args.next();
    let args: Vec<String> = args.collect();
    if let Some(user) = get_user_by_uid(get_current_uid()) {
        if let Some(username) = user.name().to_str() {
            println!("User running the executable: {}", username);
        } else {
            println!("Could not retrieve username.");
        }
    } else {
        println!("User not found.");
    }
}
