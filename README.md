# paidy-assignment
Simple restaurant API for the Paidy interview process. See requirements [here](https://github.com/paidy/interview/blob/master/SimpleRestaurantApi.md).

Built with Rust 1.82.0, no effort was made for backwards compatibility.

## Running the application

Server:
```sh
cargo run --release --bin server [<host>:<port>]
```

Client:
```sh
cargo run --release --bin client [<host>:<port>] <command> [<args>...]
```

Available commands for the client are:
```sh
client get <table-number> [<item-id>]
client order <table-number> <item-name> [<item-name>...]
client delete <table-number> <item-id>
```

`table-id` and `item-id` are positive integers, `item-name` is a string. `table-id` and `item-name` entirely arbitrary. `item-id` is assigned by the server.

The output is very crude, I lacked the time to do something pretty (see below).

## API

### Creating a new order
```typescript
POST /orders
Request:
{
    "table_number": int,
    "items": [string]
}
Response:
{
    "table_number": int,
    "items": [
        {
            "id": int,
            "name": string,
            "time_to_completion": int
        }
    ]
}
```

### Querying the orders for a table
```typescript
GET /orders/<table_number>
Request: None
Response: {
    "table_number": int,
    "items": [
        {
            "id": int,
            "name": string,
            "time_to_completion": int
        }
    ]
}
```

### Querying a single item
```typescript
GET /orders/<table_number>/items/<item_id>
Request: None
Response: {
    "id": int,
    "name": string,
    "time_to_completion": int
}
```

### Deleting an item
```typescript
DELETE /orders/<table_number>/items/<item_id>
Request: None
Response: {
    "id": int,
    "name": string,
    "time_to_completion": int
}
```

## Notes on the implementation

I went far over the time limit for this assignment. I tagged the last commit I consider working on the assignment with `v1.0.0`. I'll keep working on some parts that interest me in a different branch.

This may be a misunderstanding on my part, but I failed to find a Rust crate handling HTTP that would allow me to control explicitely the threading model underneath, which seemed to be a requirement.
So I went with the option of writing my own threadpool-backed HTTP server.
In normal times this is not an issue, I can write a simple threadpool in comfortably less than an hour in C++ and I have experience parsing from a TCP data strem, so I figured that I would have no issues craming this in less than 2 hours. Lifetimes said no. I spent an inordinate amount of time trying to explain to the borrow checker that since my threadpool owns the worker threads, the lifetime of the router and database are obviously longer. I tried to play with scoped threads and propagating lifetimes throughout my datatypes for several hours until I figured out that I could just wrap everything in `Arc<Mutex<...>>` and move on. This is obviously not a great solution, as there is no reason to reference-count the router or the database. Both need to (and do) exist longer than the threads that use them. I also need to circle back to the part reading from the TCP stream and figure out why I can't just resize my buffer in a loop.

Because of this, I didn't find the time to integrate a proper SQL database (I was originally planning to back the data model with SQLite), and the server uses currently a very simple data structure that I only intended for testing. The data type used is not the right one for this kind of application, but since I'll be adding support for a proper database later, I didn't bother to change it. I don't think it'll take too long to integrate the database, but since I'm already over the time limit and this is not strictly speaking part of the assignment, I'll do it in a different branch.

The application is also insufficiently tested for my taste, I would normally expect to have more testing around edge cases and error handling. The application is properly architectured to be testable on multiple levels, so completing the test suite is only a matter of putting in the time. The big hole, in my opinion, is the lack of end-to-end tests, notably for some stress test to validate that we can handle a large number of requests concurrently. I lack the experience on how to set this up in a Rust project so I didn't attempt it (I would normally have CMake call into some custom testing script that would spawn a server and clients and run the tests). There is currently nothing validating that the server can actually handle requests simultaneously, but I think it's clear enough from the code that it does.

Some other thoughts on the current code (in no particular order):
- The data sharing model is pretty bad, and will be problematic if we swap in a connection to a real database. The mutex means that we may have many tasks on the threadpool, but only a single thread can really work at any one time. I would start by refactoring it to be inside the object representing the database, so the routing part is free of contention. This would still be problematic for an external database, as a single connection would be in constant contention from all the threads waiting to write onto it. A better solution would be to have a pool of connections (possibly a pool per thread), with interruptible coroutines that would yield on write until the response has been received. This would avoid waiting for the database to start processing more requests. Writting this kind of runtime is clearly above my Rust level at this point.
- The error handling is messy. Client-side and server-side errors are represented by the same type. I wouldn't be surprised if I am accidently boxing the same error multiple time. As I started running out of time I was heavy handed with the `unwrap` calls, which is not a good practice. The first thing I should do on this front (if this was really going to prod) is to write a panic handler that responds with a 500 error to the client. Still not ideal, but better than crashing the server because of a panic.
