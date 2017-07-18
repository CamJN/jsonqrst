extern crate serde_json;

use serde_json::{Value, Error};

fn untyped_example(data: &str, query: &str) -> Result<Value, Error> {
    let mut v = serde_json::from_str(data);

    let r:&mut Value = v.as_mut().unwrap();

    // corner cases: . in string
    let ret = query.split(".").fold(r,|acc,i| match i.parse::<usize>() {
        Ok(i) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i)),
        Err(_) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i))
    });
    Ok(ret.clone())
}

fn main() {
    // Some JSON input data as a &str. Maybe this comes from the user.
    let data = r#"{
                    "name": "John Doe",
                    "age": 43,
                    "phones": [
                      "+44 1234567",
                      "+44 2345678"
                    ]
                  }"#;
    let query = "phones.0";
    println!("{}",untyped_example(data, query).expect("Error parsing"));
}
