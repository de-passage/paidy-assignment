pub mod server;
pub use server::*;

pub mod request;
pub use request::*;

pub mod response;
pub use response::*;

pub mod client;
pub use client::*;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple_http_request() {
        // I normally would not do this kind of tests here but time is short
        // It may fail if started several times in a row since the OS may takes some time
        // to make the port available again (or if it is already in use by something else)
        // . If I get around to it I'll extract this into something cleaner
        static ADDR: &str = "127.0.0.1:18422";

        let handle = std::thread::spawn(|| {
            eprintln!("Connecting to {}", ADDR);
            let server = HttpServer::new(ADDR);
            match server {
                Ok(s) => s.serve_once(|_| Response::ok()),
                Err(err) => eprintln!("Failed to spawn server: {}", err),
            }
        });

        let mut client = (|| {
            for _ in 1..10 {
                match HttpClient::new(ADDR) {
                    Ok(c) => return Some(c),
                    Err(err) => {
                        eprintln!("Trying to connect to {}: {}", ADDR, err);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
            None
        })()
        .expect("Failed to connect client");

        let resp = client
            .send("POST", "/", "{\"content\": \"Hello\"}")
            .expect("Failed to communicate with server");

        assert_eq!(resp.status.unwrap(), 204);

        handle.join().unwrap();
    }
}
