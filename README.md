# Zero2Prod: Mailing List

This is my implementation of the Mailing List project of the Zero to Production book by Luca Palmieri.

The project objective is to showcase various themes of software engineering for backend services using the Rust language and the Actix HTTP framework with its crates ecosystem.

The topics are:
1. Testing 
   1. integration testing using reqwest.
   2. unit testing locally using Rust's modules.
   3. parametric testing using proptest.
2. Type-driven development for data sanitization.
3. Continuous Integration using GitHub Actions.
4. Code instrumentation.
5. Containerization and Deployment with Docker and Digital Ocean.

# Testing
Zero to Production philosophy is to follow the test-driven development approach to go from definition of any requirement to a minimal implementation that satisfies it.  

## Integration testing
Integration tests are stored in the test folder of the workspace.
The dependency with the project source code is minimal, only the backend setup is imported.

Right now the tests fail in GitHub's Actions because there is no database available when testing on those setups.

## Unit testing
Implemented unit tests are placed in the module file they are testing. An example of such tests are into `domain/subscriber_email.rs` and `domain/subscriber_name.rs`.

## Property testing
An example of property test is inside the domain module's unit tests. In the last version of `quickcheck` available at the time of writing (1.0) the Gen trait has been converted into a struct making the interaction described in the book no longer viable.
So I decided to use the `proptest` library that seems to be more actively maintained looking at the GitHub issues.

In the spirit of the book's chapter I didn't encode the properties of a well-formed email according to one standard. I assumed that the emails generated from the `fake` package are well formed and test that the implemented validation logic doesn't fail against them.
