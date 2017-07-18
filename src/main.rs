#[macro_use] extern crate serde_json;
//extern crate serde_json;
extern crate regex;
extern crate atty;

use serde_json::{Value, Error};
use std::io::Read;

fn apply_query(data: &str, query: &str) -> Result<Value, Error> {
    let mut v = serde_json::from_str(data);

    let r:&mut Value = v.as_mut().unwrap();

    let ret = query.split(".").fold(r,|acc,i| match i.parse::<usize>() {
        Ok(i) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i)),
        Err(_) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i))
    });
    Ok(ret.clone())
}

fn main() {
    let query = std::env::args().skip(1).fold(String::new(),|mut s,a|{s.push_str(&a);s});

    let mut data = String::new();
    let stdin = std::io::stdin();
    let _:usize = stdin.lock().read_to_string(&mut data).expect("failed to read stdin");

    let ret = apply_query(&data, &query).expect("Error parsing");

    let stdout = std::io::stdout();
    let handle = stdout.lock();
    let ok = if atty::is(atty::Stream::Stdout) {
        serde_json::to_writer_pretty(handle, &ret)
    } else {
        serde_json::to_writer(handle, &ret)
    };
    println!("");
}

// idea: allow * as wildcard for a level (might only make sense for arrays, i dunno)
// idea: allow multiple queries
// corner case: . in string (allow escaping?)

#[cfg(test)]
mod tests {
    //#[macro_use] extern crate serde_json;
    use super::apply_query;

    #[test]
    fn get_value() {
        let data = r#"{
                        "name": "John Doe",
                        "age": 43,
                        "phones": [
                          "+44 1234567",
                          "+44 2345678"
                        ]
                      }"#;

        assert_eq!(json!("+44 1234567"), apply_query(data, "phones.0").unwrap());
        assert_eq!(json!("John Doe"), apply_query(data, "name").unwrap());
    }

    use regex::Regex;

    #[test]
    fn split_with_escapes() {
        let re = Regex::new(r"(?:[^\\])\.|^\.").unwrap();
        let fields:Vec<String> = re.split(r"a.b\.c").map(|s|s.replace(r"\.",".")).collect();

        assert_eq!(vec!["a","b.c"], fields);
    }
}
