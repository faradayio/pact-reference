//! The `pact_consumer` crate provides the test DSL for writing consumer pact tests.
//! It implements the V2 Pact specification
//! (https://github.com/pact-foundation/pact-specification/tree/version-2).
//!
//! ## To use it
//!
//! To use it, add it to your dev-dependencies in your cargo manifest and add an extern crate definition for it.
//!
//! ```ignore
//! [dev-dependencies]
//! pact_consumer = "0.2.0"
//! ```
//!
//! You can now write a pact test using the consumer DSL.
//!
//! ```
//! // TODO: This doctest has been moved to pact_consumer/tests/tests.rs
//! // pending a fix for https://github.com/rust-lang/cargo/issues/4567
//! ```

#![warn(missing_docs)]

#[cfg(test)] extern crate env_logger;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
#[macro_use] extern crate maplit;
#[macro_use] extern crate pact_matching;
extern crate pact_mock_server;
extern crate regex;
#[macro_use] extern crate serde_json;
extern crate uuid;

use pact_matching::models::*;
pub use pact_matching::models::OptionalBody;
use pact_mock_server::*;
use std::collections::HashMap;
use uuid::Uuid;
use std::panic::{self, AssertUnwindSafe};
use std::error::Error;

mod matchable;
pub use self::matchable::*;

/// Result of running the pact test
#[derive(Debug, Clone, PartialEq)]
pub enum VerificationResult {
    /// The pact was verified OK
    PactVerified,
    /// There was a mismatch between the expectations and the actual requests
    PactMismatch(Vec<MatchResult>),
    /// The provided test code returned an error
    UserCodeFailed(String),
    /// There was a mismatch between the expectations and the actual requests and the user code
    /// returned an error
    PactMismatchAndUserCodeFailed(Vec<MatchResult>, String),
    /// There was an error trying to setup the pact test
    PactError(String)
}

/// Runner for a consumer pact test
#[derive(Debug, Clone)]
pub struct ConsumerPactRunner {
    /// The Pact that represents the expectations of the consumer test
    pact: Pact
}

impl ConsumerPactRunner {

    /// Starts a mock server for the pact and executes the closure
    pub fn run(&self, test: &Fn(String) -> Result<(), String>) -> VerificationResult {
        match start_mock_server(Uuid::new_v4().simple().to_string(), self.pact.clone(), 0) {
            Ok(mock_server_port) => {
                debug!("Mock server port is {}, running test ...", mock_server_port);
                let mock_server_url = lookup_mock_server_by_port(mock_server_port, &|ms| ms.url());
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    test(mock_server_url.unwrap())
                }));
                debug!("Test result = {:?}", result);
                let mock_server_result = lookup_mock_server_by_port(mock_server_port, &|ref mock_server| {
                    mock_server.mismatches().clone()
                }).unwrap();
                let test_result = match result {
                    Ok(result) => {
                        debug!("Pact test result: {:?}", result);
                        match result {
                            Ok(_) => {
                                if mock_server_result.is_empty() {
                                    VerificationResult::PactVerified
                                } else {
                                    VerificationResult::PactMismatch(mock_server_result)
                                }
                            },
                            Err(err) => {
                                if mock_server_result.is_empty() {
                                    VerificationResult::UserCodeFailed(err)
                                } else {
                                    VerificationResult::PactMismatchAndUserCodeFailed(
                                        mock_server_result, err)
                                }
                            }
                        }
                    },
                    Err(err) => {
                        debug!("Pact test panicked: {:?}", err);
                        if mock_server_result.is_empty() {
                            VerificationResult::UserCodeFailed(s!("Pact test panicked"))
                        } else {
                            VerificationResult::PactMismatchAndUserCodeFailed(mock_server_result,
                                s!("Pact test panicked"))
                        }
                    }
                };

                let final_test_result = match test_result {
                    VerificationResult::PactVerified => {
                        let write_pact_result = lookup_mock_server_by_port(mock_server_port, &|ref mock_server| {
                            mock_server.write_pact(&Some(s!("target/pacts")))
                        }).unwrap();
                        match write_pact_result {
                            Ok(_) => test_result,
                            Err(err) => VerificationResult::PactError(s!(err.description()))
                        }
                    },
                    _ => test_result
                };

                shutdown_mock_server_by_port(mock_server_port);

                final_test_result
            },
            Err(msg) => {
                error!("Could not start mock server: {}", msg);
                VerificationResult::PactError(msg)
            }
        }
    }

}

enum BuilderState {
    None,
    BuildingRequest,
    BuildingResponse
}

/// Struct to setup the consumer pact test expectations
pub struct ConsumerPactBuilder {
    pact: Pact,
    interaction: Interaction,
    state: BuilderState
}

impl ConsumerPactBuilder {

    /// Defines the consumer involved in the Pact
    pub fn consumer<S: Into<String>>(consumer_name: S) -> Self {
        ConsumerPactBuilder {
            pact: Pact { consumer: Consumer { name: consumer_name.into() }, .. Pact::default() },
            interaction: Interaction::default(),
            state: BuilderState::None
        }
    }

    /// Defines the provider involved in the Pact
    pub fn has_pact_with<S: Into<String>>(&mut self, provider_name: S) -> &mut Self {
        self.pact.provider.name = provider_name.into();
        self
    }

    /// Describe the state the provider needs to be in for the pact test to be verified. (Optional)
    pub fn given<S: Into<String>>(&mut self, provider_state: S) -> &mut Self {
        match self.state {
            BuilderState::None => (),
            _ => self.pact.interactions.push(self.interaction.clone())
        }
        self.interaction = Interaction {
            provider_state: Some(provider_state.into()),
            .. Interaction::default()
        };
        self.state = BuilderState::BuildingRequest;
        self
    }

    /// Description of the request that is expected to be received
    pub fn upon_receiving<S: Into<String>>(&mut self, description: S) -> &mut Self {
        self.push_interaction();
        self.interaction.description = description.into();
        self
    }

    /// The path of the request
    pub fn path<P: Into<JsonPattern>>(&mut self, path: P) -> &mut Self {
        let path = path.into();
        self.push_interaction();
        let path_val: String = serde_json::from_value(path.to_example())
            // TODO: This panics, which is extremely rude anywhere but tests.
            // Do we want to panic, or return a runtime error?
            .expect("path must be a string");
        self.interaction.request.path = path_val;
        path.extract_matching_rules(
            "$.path",
            self.interaction.request.matching_rules.get_or_insert_with(Default::default),
        );
        self
    }

    /// The HTTP method for the request
    pub fn method<S: Into<String>>(&mut self, method: S) -> &mut Self {
        self.push_interaction();
        self.interaction.request.method = method.into();
        self
    }

    /// Internal API for fetching a mutable version of our current `headers`.
    pub fn headers_mut(&mut self) -> &mut HashMap<String, String> {
        let opt_headers_mut: &mut Option<_> = match self.state {
            BuilderState::BuildingRequest => &mut self.interaction.request.headers,
            BuilderState::BuildingResponse => &mut self.interaction.response.headers,
            BuilderState::None => {
                self.state = BuilderState::BuildingRequest;
                &mut self.interaction.request.headers
            }
        };
        opt_headers_mut.get_or_insert_with(|| HashMap::new())
    }

    /// A header to be included in the request. Will overwrite previous headers
    /// of the same name.
    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<JsonPattern>,
    {
        unimplemented!("header");
        //self.headers_mut().insert(key.into(), value.into());
        self
    }

    /// Headers to be included in the request. If called multiple times, this
    /// will merge the new headers with the old, overriding any duplicates.
    pub fn headers<P: Into<JsonPattern>>(&mut self, headers: P) -> &mut Self {
        unimplemented!("headers");
        //self.headers_mut()
        //    .extend(headers.into_iter().map(|(k, v)| (k.into(), v.into())));
        self
    }

    /// Internal function which returns a mutable version of our query.
    fn query_mut(&mut self) -> &mut HashMap<String, Vec<String>> {
        self.push_interaction();
        self.interaction.request.query.get_or_insert_with(|| HashMap::new())
    }

    /// A query parameter to be included in the request.
    pub fn query_param<K, V>(&mut self, key: K, value: V) -> &mut Self {
        unimplemented!("query_param")
    }

    /// The query string for the request. If called multiple times, this will
    /// merge the new query parameters with the old, overriding any duplicates.
    pub fn query<P: Into<JsonPattern>>(&mut self, query: P) -> &mut Self {
        unimplemented!("query");
        //self.query_mut().extend(query.into_iter().map(|(k, v)| {
        //    (k.into(), v.into_iter().map(|s| s.into()).collect())
        //}));
        self
    }

    /// Internal function which returns a mutable version of our current body.
    fn body_mut(&mut self) -> &mut OptionalBody {
        match self.state {
            BuilderState::BuildingRequest => &mut self.interaction.request.body,
            BuilderState::BuildingResponse => &mut self.interaction.response.body,
            BuilderState::None => {
                self.state = BuilderState::BuildingRequest;
                &mut self.interaction.request.body
            }
        }
    }

    /// This method allows specifying a request or response body using the
    /// the full `OptionalBody` choices available.
    pub fn optional_body(&mut self, body: OptionalBody) -> &mut Self {
        *self.body_mut() = body;
        self
    }

    /// Specify an unstructured body.
    ///
    /// TODO: We may want to change this to `B: Into<Vec<u8>>` depending on what
    /// happens with https://github.com/pact-foundation/pact-reference/issues/19
    /// That will still do the right thing with `&str`.
    pub fn body<S: Into<String>>(&mut self, body: S) -> &mut Self {
        *self.body_mut() = OptionalBody::Present(body.into());
        self
    }

    /// The body of the request, which will be wrapped in
    /// `OptionalBody::Present` (the common default case).
    ///
    pub fn json_body<P: Into<JsonPattern>>(&mut self, body: P) -> &mut Self {
        unimplemented!("json_body");
        //*self.body_mut() = OptionalBody::Present(body.into());
        self
    }

    fn push_interaction(&mut self) {
        match self.state {
            BuilderState::BuildingRequest => (),
            BuilderState::None => (),
            _ => {
                self.pact.interactions.push(self.interaction.clone());
                self.interaction = Interaction::default();
                self.state = BuilderState::BuildingRequest;
            }
        }
    }

    /// Define the response to return
    pub fn will_respond_with(&mut self) -> &mut Self {
        self.state = BuilderState::BuildingResponse;
        self
    }

    /// Response status code
    pub fn status(&mut self, status: u16) -> &mut Self {
        self.interaction.response.status = status;
        self.state = BuilderState::BuildingResponse;
        self
    }

    /// Terminates the DSL and builds a pact fragment to represent the interactions
    pub fn build(&mut self) -> ConsumerPactRunner {
        ConsumerPactRunner { pact: self.build_pact() }
    }

    /// Like `build`, but returns a bare `Pact` object instead of a runner.
    pub fn build_pact(&mut self) -> Pact {
        self.pact.interactions.push(self.interaction.clone());
        self.state = BuilderState::None;
        self.pact.clone()
    }
}

#[cfg(test)]
mod tests {
    use pact_matching::match_request;
    use regex::Regex;

    use super::*;

    /// Check that all requests in `actual` match the patterns provide by
    /// `expected`, and raise an error if anything fails.
    fn check_requests_match(
        actual_label: &str,
        actual: &Pact,
        expected_label: &str,
        expected: &Pact,
    ) -> Result<(), String> {
        // First make sure we have the same number of interactions.
        if expected.interactions.len() != actual.interactions.len() {
            return Err(format!(
                "the pact `{}` has {} interactions, but `{}` has {}",
                expected_label,
                expected.interactions.len(),
                actual_label,
                actual.interactions.len(),
            ));
        }

        // Next, check each interaction to see if it matches.
        for (e, a) in expected.interactions.iter().zip(&actual.interactions) {
            let mismatches = match_request(e.request.clone(), a.request.clone());
            if !mismatches.is_empty() {
                let mut reasons = String::new();
                for mismatch in mismatches {
                    reasons.push_str(&format!("- {}\n", mismatch.description()));
                }
                return Err(format!(
                    "the pact `{}` does not match `{}` because:\n{}",
                    expected_label,
                    actual_label,
                    reasons,
                ));
            }
        }

        Ok(())
    }

    macro_rules! assert_requests_match {
        ($actual:expr, $expected:expr) => (
            {
                let result = check_requests_match(
                    stringify!($actual),
                    &($actual),
                    stringify!($expected),
                    &($expected),
                );
                if let Err(message) = result {
                    panic!("{}", message)
                }
            }
        )
    }

    macro_rules! assert_requests_do_not_match {
        ($actual:expr, $expected:expr) => (
            {
                let result = check_requests_match(
                    stringify!($actual),
                    &($actual),
                    stringify!($expected),
                    &($expected),
                );
                if let Ok(()) = result {
                    panic!(
                        "pact `{}` unexpectedly matched pattern `{}`",
                        stringify!($actual),
                        stringify!($expected),
                    );
                }
            }
        )
    }

    #[test]
    fn path_pattern() {
        let greeting_regex = Regex::new("/greeting/.*").unwrap();
        let pattern = ConsumerPactBuilder::consumer("A")
            .path(Term::new(greeting_regex, "/greeting/hello"))
            .build_pact();
        let good = ConsumerPactBuilder::consumer("A")
            .path("/greeting/hi")
            .build_pact();
        let bad = ConsumerPactBuilder::consumer("A")
            .path("/farewell/bye")
            .build_pact();
        assert_requests_match!(good, pattern);
        assert_requests_do_not_match!(bad, pattern);
    }
}
