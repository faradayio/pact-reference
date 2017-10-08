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

#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate pact_matching;
extern crate pact_mock_server;
extern crate regex;
#[macro_use]
extern crate serde_json;
extern crate uuid;

use pact_matching::models::*;
pub use pact_matching::models::OptionalBody;
use pact_mock_server::*;
use std::collections::HashMap;
use uuid::Uuid;
use std::panic::{self, AssertUnwindSafe};
use std::error::Error;

// Child modules which define macros (must be first because macros are resolved)
// in inclusion order).
#[macro_use]
pub mod matchable;
use matchable::obj_key_for_path;
#[cfg(test)]
#[macro_use]
mod test_support;

// Other child modules.
pub mod builders;

/// A "prelude" or a default list of import types to include. This includes
/// the basic DSL, but it avoids including rarely-used types.
///
/// ```
/// use pact_consumer::prelude::*;
/// ```
pub mod prelude {
    // TODO: These two will go away soon.
    pub use {ConsumerPactRunner, VerificationResult};
    pub use builders::{HttpPartBuilder, PactBuilder};
    pub use matchable::{ArrayLike, JsonPattern, Matchable, SomethingLike, Term};
}
use prelude::*;

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
    PactError(String),
}

/// Runner for a consumer pact test
#[derive(Debug, Clone)]
pub struct ConsumerPactRunner {
    /// The Pact that represents the expectations of the consumer test
    pact: Pact,
}

impl ConsumerPactRunner {
    /// Create a new `ConsumerPactRunner` from the specified `Pact`.
    pub fn new(pact: Pact) -> ConsumerPactRunner {
        ConsumerPactRunner { pact: pact }
    }

    /// Starts a mock server for the pact and executes the closure
    pub fn run(&self, test: &Fn(String) -> Result<(), String>) -> VerificationResult {
        match start_mock_server(Uuid::new_v4().simple().to_string(), self.pact.clone(), 0) {
            Ok(mock_server_port) => {
                debug!("Mock server port is {}, running test ...", mock_server_port);
                let mock_server_url = lookup_mock_server_by_port(mock_server_port, &|ms| ms.url());
                let result =
                    panic::catch_unwind(AssertUnwindSafe(|| test(mock_server_url.unwrap())));
                debug!("Test result = {:?}", result);
                let mock_server_result = lookup_mock_server_by_port(
                    mock_server_port,
                    &|ref mock_server| mock_server.mismatches().clone(),
                ).unwrap();
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
                            }
                            Err(err) => {
                                if mock_server_result.is_empty() {
                                    VerificationResult::UserCodeFailed(err)
                                } else {
                                    VerificationResult::PactMismatchAndUserCodeFailed(
                                        mock_server_result,
                                        err,
                                    )
                                }
                            }
                        }
                    }
                    Err(err) => {
                        debug!("Pact test panicked: {:?}", err);
                        if mock_server_result.is_empty() {
                            VerificationResult::UserCodeFailed(s!("Pact test panicked"))
                        } else {
                            VerificationResult::PactMismatchAndUserCodeFailed(
                                mock_server_result,
                                s!("Pact test panicked"),
                            )
                        }
                    }
                };

                let final_test_result = match test_result {
                    VerificationResult::PactVerified => {
                        let write_pact_result =
                            lookup_mock_server_by_port(mock_server_port, &|ref mock_server| {
                                mock_server.write_pact(&Some(s!("target/pacts")))
                            }).unwrap();
                        match write_pact_result {
                            Ok(_) => test_result,
                            Err(err) => VerificationResult::PactError(s!(err.description())),
                        }
                    }
                    _ => test_result,
                };

                shutdown_mock_server_by_port(mock_server_port);

                final_test_result
            }
            Err(msg) => {
                error!("Could not start mock server: {}", msg);
                VerificationResult::PactError(msg)
            }
        }
    }
}

