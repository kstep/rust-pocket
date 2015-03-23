extern crate pocket;

use pocket::Pocket;
use std::io;

fn main() {
    let mut pocket = Pocket::new(&*option_env!("POCKET_CONSUMER_KEY").unwrap(), None);
    let url = pocket.get_auth_url().unwrap();
    println!("Follow auth URL to provide access: {}", url);
    let _ = io::stdin().read_line(&mut String::new());
    let username = pocket.authorize().unwrap();
    println!("username: {}", username);
    println!("access token: {:?}", pocket.access_token());

    let item = pocket.push("http://example.com").unwrap();
    println!("item: {:?}", item);

    let items = pocket.filter().complete().get();
    println!("items: {:?}", items);
}
