use common::http::{HttpServer, Response};
use common::routes::HttpRouter;

fn main() {
    let server = HttpServer::new("127.0.0.1:9898").unwrap();
    let router = HttpRouter::new().unwrap();
    server.serve(|request| {
        println!("{:?}", request);
        Response::ok()
    });
}
