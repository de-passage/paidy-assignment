// This file contains the basic types used to communicate through the API
use serde::{Deserialize, Serialize};

/// Body of new order request
#[derive(Serialize, Deserialize, Debug)]
pub struct NewOrder {
    /// Table number for the order
    pub table_number: u32,
    /// List of strings representing the items in the order
    pub items: Vec<String>,
}

/// An item, as returned by the API
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Item {
    /// Name given on creation
    pub name: String,
    /// Time to completion in minutes
    pub time_to_completion: u32,
    /// Unique ID, given by the server on creation
    pub id: u32,
}

/// A full order, as returned by the API
#[derive(Serialize, Deserialize, Debug)]
pub struct Order {
    /// Table number of the order
    pub table_number: u32,
    /// Items in the order
    pub items: Vec<Item>,
}
