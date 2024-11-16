use crate::errors;
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

pub fn router() -> errors::Result<Router<&'static str>> {
    let mut router = Router::new();
    add_path!(router, ORDERS, ORDER_BY_ID, ITEMS, ITEM_BY_ID);
    Ok(router)
}

#[cfg(test)]
mod test {
    use super::*;

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
}
