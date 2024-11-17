use crate::api::{Item, Order};
use crate::errors::Result;

/// Trait hiding the database implementation
///
/// I like to have at least a mock for unit tests, but I would also have a real
/// SQL database in a real project. The trait allows to swap one for the other
/// without touching the rest of the code.
pub trait Database {
    /// Create a new empty database
    fn new() -> Result<Self>
    where
        Self: Sized;

    /// Retrieve the full order associated with the given table
    ///
    /// On success, return the order, on failure a database-dependent error, but should
    /// return a NotFound error if the table is not found or has no item associated
    fn get_order(&self, table_id: u32) -> Result<Order>;

    /// Retrieve the item with the given id, associated with the given table id
    ///
    /// On success, return the order, on failure a database-dependent error, but should
    /// return a NotFound error if the requests succeeds but the item is not found
    fn get_order_item(&self, table_id: u32, order_id: u32) -> Result<Item>;


    /// Insert a new order with a single item in the database
    ///
    /// On success, return the inserted item, on failure a database-dependent error
    fn insert_order(&mut self, item: &str, table_id: u32) -> Result<Item>;

    /// Insert a new order in the database
    ///
    /// On success, return the inserted items, on failure a database-dependent error
    fn insert_orders(&mut self, items: Vec<String>, table_id: u32) -> Result<Vec<Item>>;


    /// Delete from the database the item with the given id that is associated with the
    /// given table id.
    ///
    /// On success, return the inserted items, on failure a database-dependent error
    fn delete_item(&mut self, table_id: u32, order_id: u32) -> Result<Item>;
}
