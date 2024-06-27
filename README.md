# Zero2Prod: Mailing List

This is my implementation of the Mailing List project of the [Zero to Production book by Luca Palmieri](https://www.zero2prod.com/).

The project objective is to showcase various themes of software engineering for backend services using the [Rust](https://www.rust-lang.org/) language and the [Actix web framework](https://actix.rs/) with its crates ecosystem.

Table of contents:
1. Features
   1. Backend healthcheck
   2. Subscriber registration
      1. Input validation
      2. User confirmation
   3. REST API to send an issue
   4. Administration dashboard
      1. User authentication
      2. Flash messages
      3. Logout
      4. Password change
      5. Fault-tolerant delivery
2. Testing 
   1. integration testing using reqwest.
   2. unit testing locally using Rust's modules.
   3. parametric testing using proptest.
3. Exercises

# Comments
I loved reading the book and making the project. The overall soundness of the software produced is great considering that I could only afford to put my commute time on it (more or less 1h per day).
It's a great showcase of the technology and had never experienced a development experience so "smooth" even when figuring out some problems due to versions mismatched.

This project brought joy back to backend development. The test driven experience is great and I think that Rust expressiveness and soundness greatly contributes to writing tests that are hard to get semantically wrong.

The book is really well thought out and a life saver for people that needs to learn the profession or needs to be reminded the rush of doing a good job.

# Features
This section is meant to give a high level overview of how some features are implemented mentioning places in the codebase that should be checked out.

## Backend healthcheck
A simple endpoint to check if everything is working. The backend parses its configuration and set's up the connection pool if not already existing.

## Subscriber registration
Subscription happens through a form submission containing the subscriber name and email. Then a verification email is sent with a link to confirm the subscription, thus bypassing storing authentication details for the subscribers.

### Input validation
Input validation happens by parsing the `zero2prod::routes::subscriptions::SubscribeFormData` into a `zero2prod::domain::NewSubscriber` type. `NewSubscriber` does not have other available constructors making only well-formed data representable in the rest of the application.

### User confirmation
The `zero2prod::email_client` module contains the implementation of a specialized client to send emails. Following the book's reccomendation it models the interaction with Postmarks's REST API. This encapsulation allows for the email sender service to be swapped out without the rest of the application being affected.

## REST API to send an issue
The POST `/newsletters` route is used to publish a newsletter issue. The endpoint is protected using a Basic authentication scheme. The information about the issue is parsed into the type `zero2prod::routes::newsletters::BodyData` using the [`serde_json`](https://crates.io/crates/serde_json) crate. 

## Administration dashboard
An administration dashboard is provided under the GET `/admin/dashboard` route.
All the `/admin/*` routes are protected by session authentication checked using a middleware located inside `zero2prod::authentication::middleware`.  
Password based authentication flow starts from the form at GET `/login`.

### Admin authentication
User authentication is handled by the `zero2prod::authentication` module. Passwords are cryptographically hashed using the Argon2id algorithm before being stored in the database in [PHC format](https://github.com/P-H-C/phc-string-format/blob/master/phc-sf-spec.md).
Hash verification is non-blocking in the sense that while the request is waiting for the verification other requests can be handled by the backend.

### Flash messages
These are used to present feedback to the user regarding form-based interaction. For example when input is malformed or when the credentials are invalid. Under the hood they use session cookies protected with a [Message Authentication Code](https://en.wikipedia.org/wiki/Message_authentication_code) to avoid cross-site scripting attacks. The implementation has been refactored to using an external crate: [`actix-web-flash-messages`](https://crates.io/crates/actix-web-flash-messages).

### Logout
Logging out just confirms the authentication status of the user and purges the session information from the store. Right now sessions are stored using a Redis instance.

### Password change
The password change flow starts at GET `/admin/password` and requires the user to provide the old password to ensure authentication again. The new password should be provided twice to avoid typing errors.

### Fault-tolerant delivery
**best-effort delivery** of the newsletter issues. This happens through asynchronous processing of the delivery with respect of the issue submission to the system.

The processing happens through the `issue_delivery_worker` that is spawned on a different thread than the application ones. This worker queries a queue implemented in PostgreSQL. This allows for a very simple implementation of a distributed transaction as multiple worker processes would request for one task of the queue while skipping rows that are already locked by other transactions.

# Testing
Zero to Production philosophy is to follow the test-driven development approach to go from definition of any requirement to a minimal implementation that satisfies it.  

## Integration testing
Integration tests are stored in the test folder of the workspace.
The dependency with the project source code is minimal, only the backend setup is imported.

Right now the tests fail in GitHub's Actions because there is no database available when testing on those setups.

## Unit testing
Implemented unit tests are placed in the module file they are testing. An example of such tests are into `zero2prod::domain::subscriber_email` and `zero2prod::domain::subscriber_name`.

## Property testing
An example of property test is inside the domain module's unit tests. In the latest version of [`quickcheck`](https://crates.io/crates/quickcheck) available at the time of writing (1.0) the Gen trait has been converted into a struct.  From my understanding the interaction described in the book leveraging `SafeEmail().fake_with_rng` is no longer immidiate.
In the end I decided to use the `proptest` library instead. I would have to write similar boilerplate but `proptest` seems to be more active looking at the GitHub issues and CI state.  
My assessment may be wrong but after spending a couple of hours on trying to fit `quickcheck` 1.0 in the codebase I just moved to `proptest`. 

In the spirit of the book's chapter I didn't encode the properties of a well-formed email according to one standard. I assumed that the emails generated from the `fake` package are well formed and test that the implemented validation logic doesn't fail against them.

## Mocking services when testing
To test the implementation of `zero2prod::email_client` module without spamming emails through Postmark the unit tests leverage `wiremock::MockServer`. 
In this way the exposed methods can be tested to call the appropriate number of times the correct endpoint of the external services. This strategy has also been used in integration tests for sending the newsletter issues and confirmation links.

# Useful development commands
## SQLX checks in offline mode!
There are some queries in the test suite so also that target needs to be prepared for offline mode checks of the `sqlx` library in the CI/CD pipeline. 
```sh
cargo sqlx prepare --workspace -- --all-targets  
```

# Exercises
- [x] Send confirmation emails when subscribing email in pending confirmation status.
- [x] Check behaviour of multiple calls to `/subscriptions/confirm` endpoint.  
Multiple calls resulted in a 200 status response. This could be fine but I'd rather have the confirmation fail after the status is already confirmed. A status 410 GONE seems to be the most fitting because a the confirmation action for that token is no longer available. So I've tested this behavior in the `confirmation_link_should_be_gone_for_confirmed_users` test.  
The test passes thanks to the `zero2prod::routes::subscriptions_confirm::subscriber_status_from_id` function. The route also uses a transaction and a "SELECT FOR UPDATE" query to ensure that the `subscriptions` table row is locked between the status check and update.   
- [x] Handle non-existent confirmation tokens.  
Asking for confirmation using an unexisting confirmation token results in an UNAUTHENTICATED response. Tested into the `confirming_a_subscription_with_an_unexisting_token_is_unauthorized` test.
- [ ] Validate incoming confirmation tokens.
- [ ] Email templating.
- [ ] Implement OWASP's requirements for password strength.
- [x] Add a "`Send a newsletter issue`" link to the admin dashboard.
- [x] Add an HTML form at GET `/admin/newsletters` to submit the new issue.
- [x] Adapt POST `/newsletters` to process the form data.
  - [x] Change the route under `/admin/newsletters`.
  - [x] Migrate authentication from 'Basic' to session-based.
  - [x] Use the Form extractor instead of the Json extractor to handle the request body.
  - [x] Adapt the test suite.
  - [ ] Replicate best effort delivery through API too.
- [ ] Enhancements to `issue_delivery_queue`
  - [ ] Keep track of number of retries.
  - [ ] Wait for retry.