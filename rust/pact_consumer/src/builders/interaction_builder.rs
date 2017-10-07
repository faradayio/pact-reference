use pact_matching::models::*;

use super::request_builder::RequestBuilder;
use super::response_builder::ResponseBuilder;

pub struct InteractionBuilder {
    description: String,
    provider_state: Option<String>,

    pub request: RequestBuilder,
    pub response: ResponseBuilder,
}

impl InteractionBuilder {
    pub fn new<D: Into<String>>(description: D) -> Self {
        InteractionBuilder {
            description: description.into(),
            provider_state: None,
            request: RequestBuilder::default(),
            response: ResponseBuilder::default(),
        }
    }

    pub fn given<G: Into<String>>(&mut self, given: G) -> &mut Self {
        self.provider_state = Some(given.into());
        self
    }

    pub fn build(&self) -> Interaction {
        Interaction {
            description: self.description.clone(),
            provider_state: self.provider_state.clone(),
            request: self.request.build(),
            response: self.response.build(),
        }
    }
}
