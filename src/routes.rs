use std::collections::HashMap;

use crate::database::Database;
use crate::{
    errors,
    http::{Request, Response},
};
use errors::Result;
use matchit::Router;

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
    ITEMS: "/orders/{order_id}/items",
    ITEM_BY_ID: "/orders/{order_id}/items/{item_id}",
}

macro_rules! add_path{
    ($router:ident $(, $path:ident)*) => {
        $(
            $router.insert(paths::$path, endpoints::$path)?;
        )*
    }
}

fn router() -> errors::Result<Router<&'static str>> {
    let mut router = Router::new();
    add_path!(router, ORDERS, ORDER_BY_ID, ITEMS, ITEM_BY_ID);
    Ok(router)
}

pub type HttpParams = HashMap<String, String>;
pub type HttpHandler = fn(Request, HttpParams, &mut dyn Database) -> Result<Response>;
type MethodToHandler = HashMap<&'static str, HttpHandler>;

pub struct HttpRouter {
    routes: Router<&'static str>,
    handlers: HashMap<&'static str, MethodToHandler>,
}

impl HttpRouter {
    pub fn new() -> Result<Self> {
        let routes = router()?;
        Ok(HttpRouter {
            routes,
            handlers: HashMap::new(),
        })
    }

    pub fn add_route(&mut self, method: &'static str, route: &'static str, handler: HttpHandler) {
        let method_to_handler = self.handlers.entry(route).or_insert_with(HashMap::new);
        method_to_handler.insert(method, handler);
    }

    pub fn route(&self, request: Request, db: &mut dyn Database) -> Result<Response> {
        let route = self
            .routes
            .at(&request.path)
            .map_err(|err| errors::Error::NotFound(err.to_string()))?;
        let method_to_handler = self.handlers.get(route.value).ok_or_else(|| {
            errors::Error::NotFound(format!(
                "No method associated to this route: {}",
                route.value
            ))
        })?;
        let handler = method_to_handler
            .get(request.method.as_str())
            .ok_or_else(|| {
                errors::Error::NotFound(format!(
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
        let router = router().unwrap();
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
        let router = router().unwrap();
        let route = router.at("/api/v1/orders/1/items/2").unwrap();
        let params = route.params;
        assert_eq!(params.get("order_id"), Some("1"));
        assert_eq!(params.get("item_id"), Some("2"));
    }

    #[test]
    fn test_missing_routes() {
        let router = router().unwrap();
        assert!(router.at("/api/v1/missing").is_err());
        assert!(router.at("/api/v2/orders/1").is_err());
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
            .route(Request::post("/api/v1/orders/42/items/24", "".to_string()), &mut db)
            .unwrap();

        assert_eq!(response.body, "42:24");
    }
}
