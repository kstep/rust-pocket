# rust-pocket
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

Now you have two methods (for now) to get and add new URLs to your pocket:

```rust
let added_item = pocket.add("http://example.com").unwrap();
let items = pocket.get();
```

The API bindings will be improved with new methods and parameters. Keep tuned!
