#![allow(unstable)]

extern crate hyper;
extern crate "rustc-serialize" as rustc_serialize;

#[cfg(test)] #[macro_use] extern crate log;

#[cfg(test)]
mod tests {

    use super::Pocket;

    #[test]
    fn it_works() {
        let mut pocket = Pocket::new(option_env!("POCKET_CONSUMER_KEY").unwrap());
        let url = pocket.get_auth_url().unwrap();
        debug!("Follow auth URL to provide access: {}", url);
        pocket.authorize().unwrap();

        let item = pocket.add("http://example.com").unwrap();
        debug!("item: {}", item);
    }
}
