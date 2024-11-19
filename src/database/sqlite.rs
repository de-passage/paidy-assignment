use crate::api::{Item, Order};
use crate::database::Database;
use crate::errors::{Error, Result};
use rand::Rng;
use rusqlite::{params, Connection};
use std::sync::{atomic::AtomicU32, Arc};

/// Contains the SQL queries used to interact with the database
pub mod sql_queries {
    // TODO: There is a better type for the time, look it up
    pub const CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS orders (id INTEGER PRIMARY KEY, item TEXT, table_number INTEGER, time_to_completion INTEGER)";

    pub const INSERT_ORDER: &str =
        "INSERT INTO orders (id, item, table_number, time_to_completion) VALUES (?1, ?2, ?3, ?4)";
    pub const SELECT_ORDER: &str = "SELECT * FROM orders WHERE table_number = ?1";
    pub const SELECT_ITEM: &str = "SELECT * FROM orders WHERE table_number = ?1 AND id = ?2";
    pub const DELETE_ITEM: &str = "DELETE FROM orders WHERE table_number = ?1 AND id = ?2";
}

pub struct SQLiteConnection {
    /// The connection
    conn: Connection,

    /// The ID to assign to the next order.
    ///
    /// I'm managing this locally because there doesn't seem to be a great way to get the last
    /// inserted ID from SQLite in the case of multiple inserts.
    current_id: Arc<AtomicU32>,
}

impl Database for SQLiteConnection {
    fn new() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute(sql_queries::CREATE_TABLE, [])?;
        Ok(SQLiteConnection {
            conn,
            current_id: Arc::new(AtomicU32::new(0)),
        })
    }

    fn insert_order(&mut self, item: &str, table_id: u32) -> Result<Item> {
        let id = self
            .current_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let time_to_completion = rand::thread_rng().gen_range(5..15);
        self.conn
            .prepare(sql_queries::INSERT_ORDER)
            .unwrap()
            .execute(params![id, item, table_id, time_to_completion])
            .map(|_| Item {
                id,
                time_to_completion,
                name: item.to_string(),
            })
            .map_err(|err| err.into())
    }

    fn get_order(&self, table_id: u32) -> Result<Order> {
        self.conn
            .prepare(sql_queries::SELECT_ORDER)
            .unwrap()
            .query_map(params![table_id], |row| {
                Ok(Item {
                    name: row.get(1)?,
                    time_to_completion: row.get(3)?,
                    id: row.get(0)?,
                })
            })
            .and_then(|row| row.collect::<std::result::Result<Vec<_>, _>>())
            .map(|items| Order {
                table_number: table_id,
                items,
            })
            .map_err(|err| err.into())
    }

    fn get_order_item(&self, table_id: u32, order_id: u32) -> crate::errors::Result<Item> {
        let rows = self
            .conn
            .prepare(sql_queries::SELECT_ITEM)
            .unwrap()
            .query_map(params![table_id, order_id], |row| {
                Ok(Item {
                    name: row.get(1)?,
                    time_to_completion: row.get(3)?,
                    id: row.get(0)?,
                })
            })
            .and_then(|row| row.collect::<std::result::Result<Vec<_>, _>>())?;

        // I spent an hour doing type tetris, and I give up, copy the data again
        rows.first().cloned().ok_or(
            Error::NotFound(format!(
                "No order with ID {} for table {}",
                order_id, table_id
            ))
            .into(),
        )
    }

    fn insert_orders(&mut self, items: Vec<String>, table_id: u32) -> Result<Vec<Item>> {
        let items = items
            .into_iter()
            .map(|item| Item {
                id: self
                    .current_id
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
                name: item.to_string(),
                time_to_completion: rand::thread_rng().gen_range(5..15),
            })
            .collect::<Vec<Item>>();

        let tx = self.conn.transaction()?;
        insert_data(&tx, &items, table_id)?;

        tx.commit()?;

        Ok(items)
    }

    fn delete_item(&mut self, _table_id: u32, _order_id: u32) -> Result<Item> {
        // This can probably be improved to avoid the extra query
        let item = self.get_order_item(_table_id, _order_id)?;
        self.conn
            .prepare(sql_queries::DELETE_ITEM)
            .unwrap()
            .execute(params![_table_id, _order_id])?;
        Ok(item)
    }
}

/// Insert data from a vector of items into the database
/// This exists only to make the borrow checker happy
fn insert_data(tx: &rusqlite::Transaction, items: &Vec<Item>, table_id: u32) -> Result<()> {
    let mut stmt = tx.prepare(sql_queries::INSERT_ORDER)?;

    for item in items.iter() {
        let params = params![item.id, item.name, table_id, item.time_to_completion];
        stmt.execute(params)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn compare_items(left: &Item, right: &Item) {
        assert_eq!(left.name, right.name);
        assert_eq!(left.time_to_completion, right.time_to_completion);
        assert_eq!(left.id, right.id);
    }

    #[test]
    fn test_insert_order() {
        let mut db = SQLiteConnection::new().unwrap();
        let item = db.insert_order("Pizza", 1).unwrap();
        assert_eq!(item.name, "Pizza");
        assert!(item.time_to_completion >= 5 && item.time_to_completion <= 15);
    }

    #[test]
    fn test_get_order() {
        let mut db = SQLiteConnection::new().unwrap();
        db.insert_order("Soda", 2).unwrap();
        let expected_pizza = db.insert_order("Pizza", 1).unwrap();
        let expected_burger = db.insert_order("Burger", 1).unwrap();
        db.insert_order("Sushi", 3).unwrap();

        let order = db.get_order(1).unwrap();
        assert_eq!(order.items.len(), 2);
        let result_pizza = order
            .items
            .iter()
            .find(|item| item.name == "Pizza")
            .unwrap();
        let result_burger = order
            .items
            .iter()
            .find(|item| item.name == "Burger")
            .unwrap();
        compare_items(result_pizza, &expected_pizza);
        compare_items(result_burger, &expected_burger);
    }

    #[test]
    fn test_get_order_item() {
        let mut db = SQLiteConnection::new().unwrap();
        db.insert_order("Pizza", 1).unwrap();
        let burger = db.insert_order("Burger", 1).unwrap();

        let item = db.get_order_item(1, burger.id).unwrap();
        compare_items(&item, &burger);
        assert_eq!(item.name, burger.name);
        assert_eq!(item.id, burger.id);
        assert_eq!(item.time_to_completion, burger.time_to_completion);
    }

    #[test]
    fn test_delete_order_item() {
        let mut db = SQLiteConnection::new().unwrap();
        db.insert_order("Pizza", 1).unwrap();
        let burger = db.insert_order("Burger", 1).unwrap();

        let item = db.delete_item(1, burger.id).unwrap();
        compare_items(&item, &burger);

        let order = db.get_order(1).unwrap();
        assert_eq!(order.items.len(), 1);
        assert_eq!(order.items[0].name, "Pizza");
    }
}