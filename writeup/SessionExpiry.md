# User login
A user can make a form request to `/login`, & get returned a valid temporary authentication token. This token exists for a set amount of time (maybe configurable in future), & has to expire at a certain time.

There is a TokenStore struct, which just contains a hashmap of the valid auth tokens to `UserLoginValues`, another struct.
```rust
#[derive(Debug)]
struct UserLoginValues {
    user_id: i32,
    login_time: DateTime<Utc>,
    expiry_time: DateTime<Utc>,
}

#[derive(Debug)]
pub struct TokenStore {
    tokens: Mutex<HashMap<String, UserLoginValues>>
}
```

A problem I seeked to fix was how to invalidate tokens after they're expired, some solutions are as follows:
1. When the user's request is being checked, do a quick check for the expiry time, if it is expired then remove the token from the hashmap & return an invalid response.
2. Have a background thread/daemon which periodically checks all user tokens, scheduled a bit like a cron job.

The former would require more time for checking the token as we'd also need to add another check for every request. It also would leave tokens lingering in the hash-map long after they would be expired, which is memory that could easily be free'd.

The latter would mean we could check tokens, say, every minute. This would keep the hash-map lean, but has a runtime overhead as we'd need a separate thread to the actix-web rutime.

I have opted for the latter option for now, though really a combination of the two could be viable, which would involve a less frequent threaded check of all users tokens, but also would have user's requests be checked on every request. This still has a larger latency for authentication (perhaps immesuable impact), but keeps the hashmap from being too large.

```rust
// main.rs' main fn
    let token_store = Arc::new(TokenStore::new());

    let thread_token_store = Arc::clone(&token_store);
    thread::spawn(move || {
        loop {
            println!("checking expiry");
            thread_token_store.check_expiry();
            sleep(time::Duration::from_secs(60));
        }
    });
    let token_storage = web::Data::new(token_store);
```
As the `TokenStore` is going to be used in both the actix-web runtime & the `std::thread`, it is put behind an atomic reference counter. When I call `Arc::clone`, it increments the atomic count by one & creates a new reference to the same underlying object, which itself is a mutex & so is threadsafe, it doesn't create a duplicate `token_store` (otherwise a great many things would break).

