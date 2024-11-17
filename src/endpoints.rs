use crate::api::*;
use crate::database::Database;
use crate::errors::{Error, Result};
use crate::http::{Request, Response};
use crate::routes::*;

pub fn create_http_router() -> Result<HttpRouter> {
    let mut router = HttpRouter::new()?;

    router.add_route("POST", endpoints::ORDERS, new_order);
    router.add_route("GET", endpoints::ORDER_BY_ID, get_items);
    router.add_route("GET", endpoints::ITEM_BY_ID, get_order_item);
    router.add_route("DELETE", endpoints::ITEM_BY_ID, delete_order_item);

    Ok(router)
}

fn get_id(params: &HttpParams, key: &str) -> Result<u32> {
    params
        .get(key)
        .ok_or(Error::BadRequest(format!("Missing '{}'", key)))
        .and_then(|id| {
            id.parse::<u32>()
                .map_err(|err| Error::BadRequest(err.to_string()))
        })
        .map_err(|err| err.into())
}

fn serialize<T: serde::Serialize>(data: T) -> Result<String> {
    serde_json::to_string(&data).map_err(|err| Error::InternalServerError(err.to_string()).into())
}

fn new_order(req: Request, _: HttpParams, db: &mut dyn Database) -> Result<Response> {
    let body = serde_json::from_str::<NewOrder>(&req.body)
        .map_err(|err| Error::BadRequest(err.to_string()))?;

    db.insert_orders(body.items, body.table_number)
        .map(|vec| Order {
            table_number: body.table_number,
            items: vec,
        })
        .and_then(&serialize)
        .map(Response::ok_with_body)
        .and_then(Ok)
}

fn get_items(_: Request, params: HttpParams, db: &mut dyn Database) -> Result<Response> {
    let order_id = get_id(&params, params::ORDER_ID)?;

    db.get_order(order_id)
        .and_then(&serialize)
        .map(Response::ok_with_body)
        .and_then(Ok)
}

fn get_order_item(_: Request, params: HttpParams, db: &mut dyn Database) -> Result<Response> {
    let order_id = get_id(&params, params::ORDER_ID)?;
    let item_id = get_id(&params, params::ITEM_ID)?;

    db.get_order_item(order_id, item_id)
        .and_then(&serialize)
        .map(Response::ok_with_body)
        .and_then(Ok)
}
fn delete_order_item(_: Request, params: HttpParams, db: &mut dyn Database) -> Result<Response> {
    let order_id = get_id(&params, params::ORDER_ID)?;
    let item_id = get_id(&params, params::ITEM_ID)?;

    db.delete_order(order_id, item_id)
        .and_then(&serialize)
        .map(Response::ok_with_body)
        .and_then(Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::mock::MockDB;

    fn to_item(resp: &Response) -> Result<Item> {
        serde_json::from_str(&resp.body).map_err(|e| e.into())
    }
    fn to_order(resp: &Response) -> Result<Order> {
        serde_json::from_str(&resp.body).map_err(|e| e.into())
    }
    fn to_items(resp: &Response) -> Result<Vec<Item>> {
        serde_json::from_str(&resp.body).map_err(|e| e.into())
    }

    macro_rules! make_db {
        () => {
            MockDB::new().unwrap()
        };

        ( $( ($table_number:literal: $order:expr $(, $orders:expr)*))*)=> {{
            let mut db = MockDB::new().unwrap();
            $(
                db.insert_orders(
                    vec![$order.to_string(), $($orders.to_string(),)* ], $table_number)
                .unwrap();
            )*
            db
        }};
    }

    fn request_from<T: serde::Serialize>(obj:&T) -> Request {
        // We don't actually care about the method in these tests
        Request::new("GET", "", vec![], serde_json::to_string(obj).unwrap())
    }
    fn empty_request() -> Request {
        Request::new("GET", "", vec![], "".to_string())
    }

    #[test]
    fn test_get_items() {
        let mut db = make_db!(
            (1: "Pizza", "Burger", "Soda")
            (2: "Sushi", "Pizza")
        );

        let response = get_items(
            empty_request(),
            make_params!(ORDER_ID: 1),
            &mut db,
        )
        .unwrap();

        println!("response: {:?}", response.body);
        let item = to_order(&response).unwrap();
        assert_eq!(item.table_number, 1);
        assert_eq!(item.items.len(), 3);
        assert!(item.items.iter().find(|i| i.name == "Pizza").is_some());
        assert!(item.items.iter().find(|i| i.name == "Burger").is_some());
        assert!(item.items.iter().find(|i| i.name == "Soda").is_some());
    }

    #[test]
    fn test_new_order() {
        let mut db = make_db!();

        let new_items = NewOrder {
            items: vec!["Pizza".to_string(), "Burger".to_string()],
            table_number: 1,
        };

        let response = new_order(
            request_from(&new_items),
            make_params!(),
            &mut db,
        )
        .unwrap();

        let order = to_items(&response).unwrap();
        assert_eq!(order.len(), 2);
        assert!(order.iter().find(|i| i.name == "Pizza").is_some());
        assert!(order.iter().find(|i| i.name == "Burger").is_some());
    }

    #[test]
    fn test_get_order_item() {
        let mut db = make_db!(
            (1: "Pizza", "Soda")
            (2: "Sushi", "Burger")
        );

        let item = db.find_by_name("Soda").unwrap();

        let response = get_order_item(
            empty_request(),
            make_params!(ORDER_ID: 1, ITEM_ID: item.id),
            &mut db,
        ).unwrap();

        let item = to_item(&response).unwrap();
        assert_eq!(item.name, "Soda");
    }

    #[test]
    fn test_delete_item() {
        let mut db = make_db!(
            (1: "Pizza", "Soda")
            (2: "Sushi", "Burger")
        );

        let item = db.find_by_name("Pizza").unwrap();

        let response = delete_order_item(
            empty_request(),
            make_params!(ORDER_ID: 1, ITEM_ID: item.id),
            &mut db,
        ).unwrap();

        let item = to_item(&response).unwrap();
        assert_eq!(item.name, "Pizza");
    }
}
