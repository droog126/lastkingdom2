use std::{fs::File, io::Read};

use serde_json::Value;

pub fn read_file_to_json(path: String) -> Value {
    let prvPath = "assets/".to_string();
    let fullPath = prvPath + &path;

    let mut mapJsonFile = File::open(fullPath).unwrap();
    let mut data = String::new();
    mapJsonFile.read_to_string(&mut data).unwrap();

    let v: Value = serde_json::from_str(&data).unwrap();

    println!("Please call {}", v);
    v
}
