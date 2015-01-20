
extern crate pocket;

use pocket::Pocket;
use std::io::stdio;

fn main() {
    let mut pocket = Pocket::new(&*option_env!("POCKET_CONSUMER_KEY").unwrap(), None);
    let url = pocket.get_auth_url().unwrap();
    println!("Follow auth URL to provide access: {}", url);
    let _ = stdio::stdin().read_line();
    let username = pocket.authorize().unwrap();
    println!("username: {}", username);
    println!("access token: {:?}", pocket.access_token());

    let item = pocket.add("http://example.com").unwrap();
    println!("item: {:?}", item);
}
