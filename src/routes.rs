use std::collections::HashMap;

use crate::database::Database;
use crate::{
    errors,
    http::{Request, Response},
};
use errors::{Result, Error};
use matchit::Router;

/// Utility macro generating a constant for the HTTP endpoint, and associate it with
/// an identifier. Matchit requires both
macro_rules! make_paths {
        ($($name:ident: $path:expr,)*) => {

        pub mod paths {
                    $(
                        pub const $name: &str = concat!("/api/v1", $path);
                    )*
        }
        pub mod endpoints {
            $(
                pub const $name: &str = stringify!($name);
            )*
        }

        }
    }

make_paths! {
    ORDERS: "/orders",
    ORDER_BY_ID: "/orders/{order_id}",
    ITEMS: "/orders/{order_id}/items", // not actually used, but someday maybe
    ITEM_BY_ID: "/orders/{order_id}/items/{item_id}",
}

/// Utility to add a list of paths to the router automatically
macro_rules! add_path{
    ($router:ident $(, $path:ident)*) => {
        $(
            $router.insert(paths::$path, endpoints::$path)?;
        )*
    }
}

/// Names of the parameters in the HTTP paths, used to extract them
/// from the parameters inside of request handling
pub mod params {
    /// Key of order ids in HTTP paths
    pub const ORDER_ID: &str = "order_id";

    /// Key of item ids in HTTP paths
    pub const ITEM_ID: &str = "item_id";
}

/// Return the HTTP path for an order based on its id
pub fn order_by_id(order_id: u32) -> String {
    paths::ORDER_BY_ID.replace("{order_id}", &order_id.to_string())
}

/// Return the HTTP path for an item based on its order id and item id
pub fn item_by_id(order_id: u32, item_id: u32) -> String {
    paths::ITEM_BY_ID
        .replace("{order_id}", &order_id.to_string())
        .replace("{item_id}", &item_id.to_string())
}


// spurious warning, I am using this in tests
#[allow(unused_macros)]
/// Utility to create easily hashmaps of parameters for testing
macro_rules! make_params {
    () => {
        std::collections::HashMap::new()
    };
    ($name:ident: $value:expr $(, $name2:ident: $value2:expr)* ) => {
        {
            let mut map = std::collections::HashMap::new();
            map.insert(params::$name.to_string(), $value.to_string());
            $(
                map.insert(params::$name2.to_string(), $value2.to_string());
            )*
            map
        }
        }
    }

#[allow(unused_imports)]
pub(crate) use make_params;

/// Create a new router with the paths defined in this module
///
/// Errors from this functions are programming errors, most likely steming from a
/// misuse of matchit
/// TODO: refactor this to separate generic functions and data types and those specific to this
/// application
fn new_router() -> errors::Result<Router<&'static str>> {
    let mut router = Router::new();
    add_path!(router, ORDERS, ORDER_BY_ID, ITEMS, ITEM_BY_ID);
    Ok(router)
}

/// Type of the object containing the HTTP path parameters passed to handlers
pub type HttpParams = HashMap<String, String>;
/// Type of the function that handles HTTP requests
pub type HttpHandler = fn(Request, HttpParams, &mut dyn Database) -> Result<Response>;

/// The router is in charge of taking in raw HTTP requests and to dispatch them to
/// the appropriate handler function.
pub struct HttpRouter {
    routes: Router<&'static str>,
    handlers: HashMap<&'static str, HashMap<&'static str, HttpHandler>>,
}

impl HttpRouter {
    /// Creates a new empty router
    ///
    /// Although the matchit router is not empty, there are no methods associated
    /// to the routes yet, so no request can be processed
    /// Errors in this function are programming errors.
    pub fn new() -> Result<Self> {
        let routes = new_router()?;
        Ok(HttpRouter {
            routes,
            handlers: HashMap::new(),
        })
    }

    /// Add a new route to the router
    pub fn add_route(&mut self, method: &'static str, route: &'static str, handler: HttpHandler) {
        let method_to_handler = self.handlers.entry(route).or_insert_with(HashMap::new);
        method_to_handler.insert(method, handler);
    }

    /// Sends a request to the appropriate handler if it exists
    ///
    /// If there is a route matching the request, its handler will be called and the result of the
    /// function will be the result of the handler. If no route is defined for this request,
    /// return Error::NotFound
    ///
    /// Checking that all parameters are presents and that the body is correct is the
    /// responsibility of the handler
    pub fn route(&self, request: Request, db: &mut dyn Database) -> Result<Response> {
        let route = self
            .routes
            .at(&request.path)
            .map_err(|err| errors::Error::NotFound(err.to_string()))?;
        let method_to_handler = self.handlers.get(route.value).ok_or_else(|| {
            Error::NotFound(format!(
                "No method associated to this route: {}",
                route.value
            ))
        })?;
        let handler = method_to_handler
            .get(request.method.as_str())
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "No handler for {} {}",
                    request.method.as_str(),
                    route.value
                ))
            })?;

        let params: HashMap<String, String> = route
            .params
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        handler(request, params, db)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::database::mock::MockDB;

    #[test]
    fn test_routes() {
        let router = new_router().unwrap();
        assert_eq!(
            *router.at("/api/v1/orders").unwrap().value,
            endpoints::ORDERS
        );
        assert_eq!(
            *router.at("/api/v1/orders/1").unwrap().value,
            endpoints::ORDER_BY_ID
        );
        assert_eq!(
            *router.at("/api/v1/orders/1/items").unwrap().value,
            endpoints::ITEMS
        );
        assert_eq!(
            *router.at("/api/v1/orders/1/items/2").unwrap().value,
            endpoints::ITEM_BY_ID
        );
    }

    #[test]
    fn test_route_ids() {
        let router = new_router().unwrap();
        let route = router.at("/api/v1/orders/1/items/2").unwrap();
        let params = route.params;
        assert_eq!(params.get("order_id"), Some("1"));
        assert_eq!(params.get("item_id"), Some("2"));
    }

    #[test]
    fn test_missing_routes() {
        let router = new_router().unwrap();
        assert!(router.at("/api/v1/missing").is_err());
        assert!(router.at("/api/v2/orders/1").is_err());
    }

    #[test]
    fn test_make_params() {
        let params = make_params!(ORDER_ID : "1", ITEM_ID : "2");
        assert_eq!(params.get(params::ORDER_ID).unwrap(), "1");
        assert_eq!(params.get(params::ITEM_ID).unwrap(), "2");
    }

    #[test]
    fn test_router() {
        const EXPECTED_GET_ORDER: &str = "get_orders";
        const EXPECTED_POST_ORDER: &str = "post_orders";
        const EXPECTED_DELETE_ITEM: &str = "delete_item";

        let mut db = MockDB::new().unwrap();

        let mut router = HttpRouter::new().unwrap();
        router.add_route("GET", endpoints::ORDERS, |_, _, _| {
            Ok(Response::ok_with_body(EXPECTED_GET_ORDER.to_string()))
        });
        router.add_route("POST", endpoints::ORDERS, |_, _, _| {
            Ok(Response::ok_with_body(EXPECTED_POST_ORDER.to_string()))
        });
        router.add_route("DELETE", endpoints::ITEMS, |_, _, _| {
            Ok(Response::ok_with_body(EXPECTED_DELETE_ITEM.to_string()))
        });

        let response = router.route(Request::get(paths::ORDERS), &mut db).unwrap();
        assert_eq!(response.body, EXPECTED_GET_ORDER);

        let response = router
            .route(Request::post(paths::ORDERS, "".to_string()), &mut db)
            .unwrap();
        assert_eq!(response.body, EXPECTED_POST_ORDER);

        assert!(router
            .route(Request::delete(paths::ORDERS, "".to_string()), &mut db)
            .is_err());

        let response = router
            .route(Request::delete(paths::ITEMS, "".to_string()), &mut db)
            .unwrap();
        assert_eq!(response.body, EXPECTED_DELETE_ITEM);
    }

    #[test]
    fn test_route_parameters() {
        let mut router = HttpRouter::new().unwrap();
        let mut db = MockDB::new().unwrap();

        router.add_route("POST", endpoints::ITEM_BY_ID, |_, params, _| {
            let order_id = params.get("order_id").unwrap();
            let item_id = params.get("item_id").unwrap();
            Ok(Response::ok_with_body(format!("{}:{}", order_id, item_id)))
        });

        let response = router
            .route(
                Request::post("/api/v1/orders/42/items/24", "".to_string()),
                &mut db,
            )
            .unwrap();

        assert_eq!(response.body, "42:24");
    }
}
