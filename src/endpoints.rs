use crate::routes::*;
use crate::errors::{Result, Error};
use crate::database::Database;
use crate::http::{Request,  Response};

pub fn create_http_router() -> Result<HttpRouter> {
    let mut router = HttpRouter::new()?;

    router.add_route(paths::ORDERS, endpoints::ORDERS, get_orders);

    Ok(router)
}

fn get_orders(_: Request, params: HttpParams, db: &mut dyn Database) -> Result<Response> {
    let table_id = params.get("table_id")
       .ok_or(Error::BadRequest("Missing table_id".to_string()))
       .and_then(|id| id.parse::<u32>().map_err(|err| Error::BadRequest(err.to_string())))?;
    let order = db.get_order(table_id).map_err(|err| Error::NotFound(err.to_string()))?;
    let body = serde_json::to_string(&order).map_err(|err| Error::BadRequest(err.to_string()))?;
    Ok(Response::ok_with_body(body))
}
