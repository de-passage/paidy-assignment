use crate::api::{Item, Order};
use crate::errors::{Error, Result};
use rand::Rng;

pub trait Database {
    fn new() -> Result<Self>
    where
        Self: Sized;

    fn get_order(&self, table_id: u32) -> Result<Order>;
    fn get_order_item(&self, table_id: u32, order_id: u32) -> Result<Item>;

    fn insert_order(&mut self, item: &str, table_id: u32) -> Result<u32>;

    fn delete_order(&mut self, table_id: u32, order_id: u32) -> bool;
}

pub mod mock {

    use super::*;

    type DBElement = (u32, Item);
    pub struct MockDB(Vec<DBElement>, u32);

    impl Database for MockDB {
        fn new() -> Result<Self> {
            Ok(MockDB(Vec::new(), 0))
        }

        fn insert_order(&mut self, item: &str, table_id: u32) -> Result<u32> {
            let id = self.1;
            self.1 += 1;
            self.0.push((
                table_id,
                Item {
                    name: item.to_string(),
                    time_to_completion: rand::thread_rng().gen_range(5..15),
                    id,
                },
            ));
            Ok(id)
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

        fn delete_order(&mut self, table_id: u32, order_id: u32) -> bool {
            let old_len = self.0.len();
            self.0
                .retain(|(id, item)| *id != table_id || item.id != order_id as u32);

            old_len != self.0.len()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_db() {
            let mut db = MockDB::new().unwrap();
            let pizza_id = db.insert_order("Pizza", 1).unwrap();
            let burger_id = db.insert_order("Burger", 2).unwrap();
            let pasta_id = db.insert_order("Pasta", 1).unwrap();

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

            assert!(db.delete_order(1, pizza_id));
            assert!(!db.delete_order(1, pizza_id));
            assert!(!db.delete_order(1, burger_id));
            assert!(db.delete_order(2, burger_id));
        }
    }
}
