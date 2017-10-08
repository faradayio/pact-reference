//! The `pact_consumer` crate provides the test DSL for writing consumer pact
//! tests. It implements the [V2 Pact specification][spec].
//!
//! ## How to use it
//!
//! To use this crate, add it to your `[dev-dependencies]` in your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! pact_consumer = "0.2"
//! ```
//!
//! Then add the following to your top-level `lib.rs` or `main.rs` file:
//!
//! ```
//! #[cfg(test)]
//! #[macro_use]
//! extern crate pact_consumer;
//! ```
//!
//! Once this is done, you can then write the following inside a function marked
//! with `#[test]`:
//!
//! ```
//! # #[macro_use] extern crate pact_consumer;
//! # fn main() {
//! use pact_consumer::prelude::*;
//!
//! // Define the Pact for the test, specify the names of the consuming
//! // application and the provider application.
//! let pact = PactBuilder::new("Consumer", "Alice Service")
//!     // Start a new interaction. We can add as many interactions as we want.
//!     .interaction("a retrieve Mallory request", |i| {
//!         // Defines a provider state. It is optional.
//!         i.given("there is some good mallory");
//!         // Define the request, a GET (default) request to '/mallory'.
//!         i.request.path("/mallory");
//!         // Define the response we want returned.
//!         i.response
//!             .status(200)
//!             .header("Content-Type", "text/plain")
//!             .body("That is some good Mallory.");
//!     })
//!     .build();
//! # }
//! ```
//!
//! You can than use an HTTP client like `reqwest` to make requests against your
//! server.
//!
//! ```rust,no_run
//! # // This is marked `no_run` because of the issues described in
//! # // https://github.com/rust-lang/cargo/issues/4567. An executable
//! # // version is checked in tests/tests.rs.
//! # #[macro_use] extern crate pact_consumer;
//! # extern crate pact_matching;
//! # extern crate reqwest;
//! # use pact_matching::models::Pact;
//! # use std::io::Read;
//! # fn main() {
//! #     use pact_consumer::prelude::*;
//! #     let pact: Pact = unimplemented!();
//! // Start the mock server running.
//! let alice_service = pact.start_mock_server();
//!
//! // You would use your actual client code here.
//! let mallory_url = alice_service.url().join("/mallory").unwrap();
//! let mut response = reqwest::get(mallory_url).expect("could not fetch URL");
//! let mut body = String::new();
//! response.read_to_string(&mut body).expect("could not read response body");
//! assert_eq!(body, "That is some good Mallory.");
//!
//! // When `alice_service` goes out of scope, your pact will be validated,
//! // and the test will fail if the mock server didn't receive matching
//! // requests.
//! # }
//! ```

#![warn(missing_docs)]

#[cfg(test)]
extern crate env_logger;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate pact_matching;
extern crate pact_mock_server;
extern crate regex;
#[macro_use]
extern crate serde_json;
extern crate url;
extern crate uuid;

// Child modules which define macros (must be first because macros are resolved)
// in source inclusion order).
#[macro_use]
pub mod matchable;
#[cfg(test)]
#[macro_use]
mod test_support;

// Other child modules.
pub mod builders;
pub mod mock_server;

/// A "prelude" or a default list of import types to include. This includes
/// the basic DSL, but it avoids including rarely-used types.
///
/// ```
/// use pact_consumer::prelude::*;
/// ```
pub mod prelude {
    pub use builders::{HttpPartBuilder, PactBuilder};
    pub use matchable::{ArrayLike, JsonPattern, Matchable, SomethingLike, Term};
    pub use mock_server::{StartMockServer, ValidatingMockServer};
}
