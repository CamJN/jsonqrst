#[allow(unused_imports)]
#[macro_use] extern crate serde_json;
#[macro_use] extern crate nom;
extern crate atty;

use serde_json::{Value, Error};
use std::io::Read;


named!(not_sep(&[u8]) -> String, map!(
    escaped_transform!(is_not_s!(r".\"), '\\', alt!(
        tag!(r".") => {|_| &b"."[..]} |
        tag!(r"\") => {|_| &b"\\"[..]}
    )), |i| String::from_utf8_lossy(&i).into_owned()
));

named!(split_with_escapes(&[u8]) -> Vec<String>, separated_list_complete!( char!('.'), not_sep));

fn apply_query(data: &str, query: &str) -> Result<Value, Error> {
    let mut v = serde_json::from_str(data);

    let r:&mut Value = v.as_mut().unwrap();

    let ret = split_with_escapes(query.as_bytes()).unwrap().1.iter().fold(r,|acc,i| match i.parse::<usize>() {
        Ok(i) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i)),
        Err(_) => acc.get_mut(i).expect(&format!("index {} doesn't exist",i))
    });
    Ok(ret.clone())
}

fn main() {
    let default = "Failed to get panic message.".to_string();
    std::panic::set_hook(Box::new(move |p|{
        eprintln!("{}",p.payload().downcast_ref::<std::string::String>().unwrap_or(&default));
        std::process::exit(-1);
    }));

    let query = std::env::args().skip(1).fold(String::new(),|mut s,a|{s.push_str(&a);s});

    let mut data = String::new();
    let stdin = std::io::stdin();
    let _:usize = stdin.lock().read_to_string(&mut data).expect("failed to read stdin");

    let ret = apply_query(&data, &query).expect("Error parsing");

    let stdout = std::io::stdout();
    let handle = stdout.lock();
    match if atty::is(atty::Stream::Stdout) {
        serde_json::to_writer_pretty(handle, &ret)
    } else {
        serde_json::to_writer(handle, &ret)
    } {
        Ok(_) => println!(""),
        Err(e) => eprintln!("{}",e),
    }
}

// handle text keys that only contain #s
// idea: allow * as wildcard for a level (might only make sense for arrays, i dunno)
// idea: allow multiple queries

#[cfg(test)]
mod tests {
    use super::apply_query;
    #[test]
    fn test_get_value() {
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

    use super::not_sep;
    use nom::IResult;
    #[test]
    fn test_not_sep() {
        assert_eq!(IResult::Done(&b""[..], "b.c".to_string()), not_sep(b"b\\.c"));
        assert_eq!(IResult::Done(&b".c"[..], "b".to_string()), not_sep(b"b.c"));
    }

    use super::split_with_escapes;
    #[test]
    fn test_split_with_escapes() {
        assert_eq!(IResult::Done(&b""[..], vec!["a".to_string(),"b".to_string()]), split_with_escapes(b"a.b"));
        assert_eq!(IResult::Done(&b""[..], vec!["a".to_string(),"b.c".to_string()]), split_with_escapes(b"a.b\\.c"));
    }
}
