# rust-pocket <a href="https://travis-ci.org/kstep/rust-pocket"><img src="https://img.shields.io/travis/kstep/rust-pocket.png?style=flat-square" /></a> <a href="https://crates.io/crates/pocket"><img src="https://img.shields.io/crates/d/pocket.png?style=flat-square" /></a> <a href="https://crates.io/crates/pocket"><img src="https://img.shields.io/crates/v/pocket.png?style=flat-square" /></a>

[Pocket API](http://getpocket.com/developer/docs/overview) bindings (http://getpocket.com), WIP

API is very easy, actually. The most complex code is for authorization.
You will need a `consumer_key` and an `access_token` in order to use the API.

A `consumer_key` can be obtained by creating an app at the [My Applications](http://getpocket.com/developer/apps/) page.
An `access_token` is obtained by walking through [OAuth authentication workflow](http://getpocket.com/developer/docs/authentication).

The OAuth workflow is implemented with a pair of methods in this implementation:

```rust
extern crate pocket;

use pocket::Pocket;

fn authenticate() {
  let mut pocket = Pocket::new("YOUR-CONSUMER-KEY-HERE", None);
  let url = pocket.get_auth_url().unwrap();
  println!("Follow the link to authorize the app: {}", url);
  // Here we should wait until user follows the URL and confirm app access
  
  let username = pocket.authorize().unwrap;
}
```

So you 1) generate OAuth access request URL with `pocket.get_auth_url()`, 2) let user follow the URL
and confirm app access,  3) call `pocket.authorize()` and either get an error,
or username of user just authorized.

I recommend storing the access token after you get it, so you don't have to repeat this workflow again next time.
The access token can be obtained with `pocket.access_token()` method. Store it somewhere and use to construct
`Pocket` object:

```rust
let access_token = "YOUR-STORED-ACCESS-TOKEN";
let mut pocket = Pocket::new("YOUR-CONSUMER-KEY-HERE", Some(access_token));
```

Now you have two methods (for now) to get and add new URLs to your pocket.

To add an item, use `Pocket::add()` or `Pocket::push()` method:

```rust
// Quick add by URL only
let added_item = pocket.push("http://example.com").unwrap();

// Add with all meta-info provided (title, tags, tweet id)
let added_item = pocket.push("http://example.com", Some("Example title"), Some("example-tag"), Some("example_tweet_id")).unwrap();
```

To query your pocket, use `Pocket::filter()` method:

```rust
let items = pocket.filter()
    .complete() // complete data
    .archived() // archived items only
    .videos()   // videos only
    .offset(10) // items 10-20
    .count(10)
    .sort_by_title() // sorted by title
    .get(); // get items

// There are other methods, see `PocketGetRequest` struct for details
```

The API bindings will be improved with new methods and parameters. Keep tuned!

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
