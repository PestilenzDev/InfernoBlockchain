// src/bin/test_serde.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct TestStruct {
    id: u64,
}

fn main() {
    let test = TestStruct { id: 42 };
    let serialized = serde_json::to_string(&test).unwrap();
    println!("{}", serialized);
}