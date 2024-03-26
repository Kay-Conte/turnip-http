use foxhole::{
    sys,
    websocket::{Upgrade, Websocket},
    App, Http1, Scope,
};

fn upgrade(upgrade: Upgrade) -> Websocket {
    println!("Running");
    upgrade.handle(|mut ws| loop {
        match ws.next_frame() {
            Ok(v) => println!("{:?}", v),
            Err(e) => println!("{e:?}"),
        }
    })
}

fn main() {
    let scope = Scope::new(sys![upgrade]);

    println!("Running on '127.0.0.1:8080'");

    App::builder(scope).run::<Http1>("127.0.0.1:8080");
}
