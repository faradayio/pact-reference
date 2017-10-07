use pact_matching::models::*;

use matchable::*;

pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    pub fn status(&mut self, status: u16) -> &mut Self {
        self
    }

    pub fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self
    }

    pub fn json_body<B: Into<JsonPattern>>(&mut self, body: B) -> &mut Self {
        self
    }

    pub fn build(&self) -> Response {
        self.response.clone()
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        ResponseBuilder { response: Response::default_response() }
    }
}
