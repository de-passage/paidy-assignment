use common::cli;
use common::database::{mock::MockDB, Database};
use common::endpoints;
use common::errors::*;
use common::http::{HttpServer, Response};
use std::sync::{Arc, Mutex};

fn main() {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or(cli::DEFAULT_ADDRESS.to_string());

    let server = HttpServer::new(&addr).unwrap();
    let router = Arc::new(endpoints::create_http_router().unwrap());
    let db = Arc::new(Mutex::new(MockDB::new().unwrap()));

    server.serve(move |request| {
        println!("{:?}", request);
        let result = db
            .lock()
            .map_err(|e| Error::InternalServerError(e.to_string()).into())
            .and_then(|mut db| router.route(request, &mut *db));

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                eprintln!("Error processing request: {:?}", &err); // can't downcast without moving
                                                                   // apparently
                if let Ok(err) = err.downcast::<common::errors::Error>() {
                    match *err {
                        Error::NotFound(_) => Response::error(404),
                        Error::BadRequest(_) => Response::error(400),
                        _ => Response::internal_server_error(),
                    }
                } else {
                    Response::internal_server_error()
                }
            }
        };
        println!("{:?}", response);
        response
    });
}
