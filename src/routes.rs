use matchit::Router;
use crate::errors;

pub mod paths {
    macro_rules! make_paths {
        ($($name:ident: $path:expr,)*) => {
            $(
                pub const $name: &str = concat!("/api/v1", $path);
            )*
        }
    }

    make_paths!{
        ORDERS: "/order",
        ORDER_BY_ID: "/order/{id}",
        ITEMS: "/order/{id}/item",
        ITEM_BY_ID: "/order/{id}/item",
    }
}

fn router() -> errors::Result<Router<&'static str>> {
    todo!()
}
