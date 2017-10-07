use pact_matching::models::*;
#[cfg(test)]
use regex::Regex;
use serde_json;

use matchable::obj_key_for_path;
use prelude::*;

pub struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    /// Specify the request method. Defaults to `"GET"`.
    pub fn method<M: Into<String>>(&mut self, method: M) -> &mut Self {
        self
    }

    /// Specify the request path. Defaults to `"/"`.
    pub fn path<P: Into<JsonPattern>>(&mut self, path: P) -> &mut Self {
        let path = path.into();
        self.request.path = serde_json::from_value(path.to_example())
                // TODO: This panics, which is extremely rude anywhere but
                // tests. Do we want to panic, or return a runtime error?
                .expect("path must be a string");
        path.extract_matching_rules(
            "$.path",
            &mut self.request.matching_rules.get_or_insert_with(
                Default::default,
            ),
        );
        self
    }

    /// Specify a query parameter. You may pass either a single value or
    /// a list of values to represent a repeated parameter.
    ///
    /// ```
    /// #[macro_use]
    /// extern crate pact_consumer;
    /// extern crate regex;
    ///
    /// use pact_consumer::prelude::*;
    /// use pact_consumer::builders::RequestBuilder;
    /// use regex::Regex;
    ///
    /// # fn main() {
    /// let digits_re = Regex::new("^[0-9]+$").unwrap();
    /// RequestBuilder::default()
    ///   .query_param("simple", "value")
    ///   .query_param("pattern", Term::new(digits_re, "123"))
    ///   .query_param("list", json_pattern!(["a", "b"]));
    /// # }
    /// ```
    pub fn query_param<K, V>(&mut self, key: K, values: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<JsonPattern>,
    {
        let key = key.into();
        let values = values.into();

        // Extract our example JSON.
        //
        // TODO: These calls to `expect` are rude, as described above.
        let values_example = match values.to_example() {
            serde_json::Value::String(s) => vec![s],
            arr @ serde_json::Value::Array(_) => {
                serde_json::from_value(arr).expect("expected array of strings")
            }
            other => panic!("expected array of strings, found: {}", other),
        };
        self.request.query
            .get_or_insert_with(Default::default)
            .insert(key.clone(), values_example);

        // Extract our matching rules.
        values.extract_matching_rules(
            &format!("$.query{}", obj_key_for_path(&key)),
            &mut self.request.matching_rules.get_or_insert_with(
                Default::default,
            ),
        );

        self
    }

    /// Build the specified `Request` object.
    pub fn build(&self) -> Request {
        self.request.clone()
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        RequestBuilder { request: Request::default_request() }
    }
}

#[test]
fn path_pattern() {
    let greeting_regex = Regex::new("/greeting/.*").unwrap();
    let pattern = PactBuilder::new("C", "P")
        .interaction("I", |i| {
            i.request.path(Term::new(greeting_regex, "/greeting/hello"));
        })
        .build();
    let good = PactBuilder::new("C", "P")
        .interaction("I", |i| { i.request.path("/greeting/hi"); })
        .build();
    let bad = PactBuilder::new("C", "P")
        .interaction("I", |i| { i.request.path("/farewell/bye"); })
        .build();
    assert_requests_match!(good, pattern);
    assert_requests_do_not_match!(bad, pattern);
}

#[test]
fn query_param_pattern() {
    let greeting_regex = Regex::new("h.*").unwrap();
    let pattern = PactBuilder::new("C", "P")
        .interaction("I", |i| {
            i.request.query_param(
                "greeting",
                Term::new(greeting_regex, "hello"),
            );
        })
        .build();
    let good = PactBuilder::new("C", "P")
        .interaction("I", |i| { i.request.query_param("greeting", "hi"); })
        .build();
    let bad = PactBuilder::new("C", "P")
        .interaction("I", |i| { i.request.query_param("greeting", "bye"); })
        .build();
    assert_requests_match!(good, pattern);
    assert_requests_do_not_match!(bad, pattern);
}
