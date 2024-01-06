# Route Protection
For the web server, I have authentication & login / registration for it, therefore I would like to have some mechansim of being able to protect some routes (by checking headers for a valid token) and not protecting some (for example, for login / register).

### Option 1
One option would be to manually check for each route handler, but this would result in quite a lot of boilerplate code, & would require lots of re-working should the authenticaiton mechanism change.

### Option 2
We could add a middleware to the actix-web server that first checked the url request path, & then either did the authentication checking or passed through to routes that didn't require it.
This has a benefit in that you don't have to worry about it for all the routes that have authentication, but if we were to expose more of the api to non-authenticated users (e.g for stats) then we would need to add the exceptions manually to the middleware.
This also doesn't immediately show which routes are protected or not, which is not quite as readable.a

### Option 3
I had a thought from my python days, in adding a decorator function that would add a proc macro function attribute that would intercept & add function calls to the authenticator, however I could never quite work out how to add these macro's as attribute macros seemed not to be executing functions.

### Option 4 (the one i chose)
Actix-web has a built-in method `FromRequest` that parses a request into a input type for the designated handler function. In the parsing function I can add the authentication code. This is known as an `Extractor` in actix-web, & there are many defaults [docs](https://actix.rs/docs/extractors/)
To distinguish between routes you can see the abscence or presence of the `Authentication` type. Routes that accept an Authentication type will have authentication checks, those without will not.
We can also change authentication 


```rust
impl FromRequest for Authentication {
    type Error = Error;
    type Future = Ready<Result<Authentication, Error>>;

    #[inline]
    fn from_request(
        req: &actix_web::HttpRequest,
        payload: &mut actix_web::dev::Payload,
    ) -> Self::Future {
        match req.headers().get("Authorization") {
            // todo, check things
            Some(header) => ok(Authentication {
                token: (header.to_str().unwrap().to_owned()),
            }),
            _ => err(ErrorUnauthorized("not authorized"))
        }
    }
}
```