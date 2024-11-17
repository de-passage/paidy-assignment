// This file contains the basic types used to communicate through the API
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct NewOrder {
    pub table_number: u32,
    pub items: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item {
    pub name: String,
    pub time_to_completion: u32,
    pub id: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    pub table_number: u32,
    pub items: Vec<Item>,
}
