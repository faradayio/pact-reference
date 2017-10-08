// Put these in your crate root. You can add `#[cfg(test)]` before any
// crate that you only use in test mode.
extern crate pact_consumer;
extern crate reqwest;

use pact_consumer::prelude::*;
use std::io::prelude::*;

/// This is supposed to be a doctest in lib.rs, but it's breaking there. This
/// is written in a "neutral" Rust style, using the standard test framework and
/// popular libraries.
#[test]
fn relocated_doctest() {
    // Define the Pact for the test, specify the names of the consuming
    // application and the provider application.
    let pact = PactBuilder::new("Consumer", "Alice Service")
        // Start a new interaction. We can add as many interactions as we want.
        .interaction("a retrieve Mallory request", |i| {
            // Defines a provider state. It is optional.
            i.given("there is some good mallory");
            // Define the request, a GET (default) request to '/mallory'.
            i.request.path("/mallory");
            // Define the response we want returned.
            i.response
                .status(200)
                .header("Content-Type", "text/plain")
                .body("That is some good Mallory.");
        })
        .build();

    // Execute the run method to have the mock server run (the URL to the mock server will be passed in).
    // It takes a closure to execute your requests and returns a Pact VerificationResult.
    let result = ConsumerPactRunner::new(pact).run(&|url| {
        // You would use your actual client code here.
        let mut response = reqwest::get(&format!("{}/mallory", url))
            .expect("could not fetch URL");
        let mut body = String::new();
        response.read_to_string(&mut body)
            .expect("could not read response body");
        assert_eq!(body, "That is some good Mallory.");
        Ok(())
    });

    // This means it is all good.
    assert_eq!(result, VerificationResult::PactVerified);
}
