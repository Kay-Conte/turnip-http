use foxhole::{
    action::Html, connection::Http1, layers::Layer, resolve::Get, routing::Router, run, sys,
    Request, Response, Scope,
};

pub struct Logger;

// This implementation will be run before any handling of the request.
impl Layer<Request> for Logger {
    fn execute(&self, data: &mut Request) {
        println!("Request url: {}", data.uri())
    }
}

// This implementation will run right before sending to the client.
impl Layer<Response> for Logger {
    fn execute(&self, data: &mut Response) {
        println!("Response: {:?}", data);
    }
}

fn get(_get: Get) -> Html {
    Html("<h1> Foxhole </h1>".to_string())
}

fn main() {
    let scope = Scope::new(sys![get]);

    println!("Running on '127.0.0.1:8080'");

    run::<Http1>(
        "127.0.0.1:8080",
        Router::builder(scope)
            .request_layer(Logger)
            .response_layer(Logger),
    );
}