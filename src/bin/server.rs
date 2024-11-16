use common::http::{HttpServer, Response};

fn main() {
        let server = HttpServer::new("127.0.0.1:9898").unwrap();
        server.serve(|request| {
            println!("{:?}", request);
            Response::ok()
        });
}
