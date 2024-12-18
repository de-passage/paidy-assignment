use crate::api::{Item, Order};
use crate::errors::{Error, Result};
use rand::Rng;

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

pub mod mock {
    use super::*;

    type DBElement = (u32, Item);

    /// Mock database implementation
    ///
    /// This is a very simple database based on a Vec. I should probably have used a HashMap if I
    /// meant to deploy this in production, but for the very small datasets that I have been
    /// manipulating in the development, this is perfectly fine.
    pub struct MockDB(Vec<DBElement>, u32);

    impl MockDB {
        /// Retrieves an item based on its name
        ///
        /// If several items are named the same, only the first one will be returned
        /// Convenience function used to ease testing. Do not use in the production code
        pub fn find_by_name(&self, name: &str) -> Option<&Item> {
            self.0.iter().find(|(_, item)| item.name == name).map(|(_, item)| item)
        }
    }

    impl Database for MockDB {
        fn new() -> Result<Self> {
            Ok(MockDB(Vec::new(), 0))
        }

        fn insert_order(&mut self, item: &str, table_id: u32) -> Result<Item> {
            let id = self.1;
            let item = Item {
                name: item.to_string(),
                time_to_completion: rand::thread_rng().gen_range(5..15),
                id,
            };

            self.1 += 1;
            self.0.push((table_id, item.clone()));
            Ok(item)
        }

        fn insert_orders(&mut self, items: Vec<String>, table_id: u32) -> Result<Vec<Item>> {
            let db_items: Vec<_> = items
                .into_iter()
                .map(|item| {
                    let id = self.1;
                    self.1 += 1;
                    (
                        table_id,
                        Item {
                            name: item.to_string(),
                            time_to_completion: rand::thread_rng().gen_range(5..15),
                            id,
                        },
                    )
                })
                .collect();

            // I don't like duplicating the intermediary result but I don't have time to
            // look up a better solution
            let result = db_items.iter().map(|(_, item)| item.clone()).collect();

            self.0.extend(db_items.clone());

            Ok(result)
        }

        fn get_order(&self, table_id: u32) -> Result<Order> {
            let items: Vec<_> = self
                .0
                .iter()
                .filter(|(id, _)| *id == table_id)
                .map(|(_, item)| item.clone())
                .collect();

            if items.is_empty() {
                Err(Error::NotFound(format!("No orders for table {}", table_id)).into())
            } else {
                Ok(Order {
                    items,
                    table_number: table_id,
                })
            }
        }

        fn get_order_item(&self, table_id: u32, order_id: u32) -> Result<crate::api::Item> {
            self.0
                .iter()
                .find(|(id, item)| *id == table_id && item.id == order_id as u32)
                .map(|(_, item)| item.clone())
                .ok_or(
                    Error::NotFound(format!(
                        "No item with id {} for table {}",
                        order_id, table_id
                    ))
                    .into(),
                )
        }

        fn delete_item(&mut self, table_id: u32, order_id: u32) -> Result<Item> {
            let index = self
                .0
                .iter()
                .position(|(id, item)| *id == table_id && item.id == order_id as u32)
                .ok_or(Error::NotFound(format!(
                    "No item with id {} for table {}",
                    order_id, table_id
                )))?;

            Ok(self.0.remove(index).1)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_db() {
            let mut db = MockDB::new().unwrap();
            let pizza_id = db.insert_order("Pizza", 1).unwrap().id;
            let burger_id = db.insert_order("Burger", 2).unwrap().id;
            let pasta_id = db.insert_order("Pasta", 1).unwrap().id;

            let result = db.get_order(1).unwrap();
            assert_eq!(result.items.len(), 2);
            assert_eq!(result.items[0].name, "Pizza");
            assert_eq!(result.items[0].id, pizza_id as u32);
            assert_eq!(result.items[1].name, "Pasta");
            assert_eq!(result.items[1].id, pasta_id as u32);

            let result = db.get_order(2).unwrap();
            assert_eq!(result.items.len(), 1);
            assert_eq!(result.items[0].name, "Burger");
            assert_eq!(result.items[0].id, burger_id as u32);

            let result = db.get_order(3);
            assert!(result.is_err());

            assert_eq!(db.get_order_item(1, pizza_id).unwrap().name, "Pizza");
            assert_eq!(db.get_order_item(2, burger_id).unwrap().name, "Burger");
            assert_eq!(db.get_order_item(1, pasta_id).unwrap().name, "Pasta");

            assert!(db.delete_item(1, pizza_id).is_ok());
            assert!(db.delete_item(1, pizza_id).is_err());
            assert!(db.delete_item(1, burger_id).is_err());
            assert!(db.delete_item(2, burger_id).is_ok());
        }
    }
}
