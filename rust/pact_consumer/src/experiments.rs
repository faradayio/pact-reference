use pact_matching::models::*;

struct PactBuilder {
    pact: Pact,
}

impl PactBuilder {
    fn new<C, P>(consumer: C, provider: P) -> Self
    where
        C: Into<String>,
        P: Into<String>,
    {
        let mut pact = Pact::default();
        pact.consumer = Consumer { name: consumer.into() };
        pact.provider = Provider { name: provider.into() };
        PactBuilder { pact: pact }
    }

    fn interaction<D, F>(&mut self, description: D, build_fn: F) -> &mut Self
    where
        D: Into<String>,
        F: FnOnce(&mut InteractionBuilder),
    {
        let mut interaction = InteractionBuilder::new(description.into());
        build_fn(&mut interaction);
        self.push_interaction(interaction.build())
    }

    fn push_interaction(&mut self, interaction: Interaction) -> &mut Self {
        self.pact.interactions.push(interaction);
        self
    }

    fn build(&self) -> Pact {
        self.pact.clone()
    }
}

struct InteractionBuilder {
    description: String,
    provider_state: Option<String>,

    pub request: RequestBuilder,
    pub response: ResponseBuilder,
}

impl InteractionBuilder {
    fn new<D: Into<String>>(description: D) -> Self {
        InteractionBuilder {
            description: description.into(),
            provider_state: None,
            request: RequestBuilder::default(),
            response: ResponseBuilder::default(),
        }
    }

    fn given<G: Into<String>>(&mut self, given: G) -> &mut Self {
        self.provider_state = Some(given.into());
        self
    }

    fn build(&self) -> Interaction {
        Interaction {
            description: self.description.clone(),
            provider_state: self.provider_state.clone(),
            request: self.request.build(),
            response: self.response.build(),
        }
    }
}

struct RequestBuilder {
    request: Request,
}

impl RequestBuilder {
    fn method<M: Into<String>>(&mut self, method: M) -> &mut Self {
        self
    }

    fn path<P: Into<String>>(&mut self, path: P) -> &mut Self {
        self
    }

    fn build(&self) -> Request {
        self.request.clone()
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        RequestBuilder { request: Request::default_request() }
    }
}

struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    fn status(&mut self, status: u16) -> &mut Self {
        self
    }

    fn header<K, V>(&mut self, key: K, value: V) -> &mut Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self
    }

    fn body<B: Into<String>>(&mut self, body: B) -> &mut Self {
        self
    }

    fn build(&self) -> Response {
        self.response.clone()
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        ResponseBuilder { response: Response::default_response() }
    }
}

#[test]
fn new_api() {
    let pact = PactBuilder::new("Consumer", "Provider")
        .interaction("GET /greeting/hello", |i| {
            i.given("a greeting named hello");
            i.request.method("GET").path("/greeting/hello");
            i.response
                .status(200)
                .header("Content-Type", "application/json")
                .body("Hello!");
        })
        .build();
}
