#[allow(unused_imports)]
#[macro_use] extern crate serde_json;
#[macro_use] extern crate nom;
extern crate atty;

use serde_json::Value;
use std::io::Read;

named!(not_sep(&[u8]) -> String, map!(
    escaped_transform!(is_not_s!(r".\"), '\\', alt!(
        tag!(r".") => {|_| &b"."[..]} |
        tag!(r"\") => {|_| &b"\\"[..]}
    )), |i| String::from_utf8_lossy(&i).into_owned()
));

named!(split_with_escapes(&[u8]) -> Vec<String>, separated_list_complete!( char!('.'), not_sep));

fn apply_query<T:Read>(stdin: T, query: &str) -> Value {
    let mut v = serde_json::from_reader(stdin);
    let r:&mut Value = v.as_mut().unwrap();

    split_with_escapes(query.as_bytes()).unwrap().1.iter().fold(r,|acc,i| match *acc {
        Value::Array(ref mut a) => a.get_mut(i.parse::<usize>().expect(&format!("index {} isn't an unsigned integer",i))).expect(&format!("index {} doesn't exist",i)),
        Value::Object(ref mut o) => o.get_mut(i).expect(&format!("index {} doesn't exist",i)),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => panic!("tried to index a non-collection type")
    }).clone()
}

fn apply_query_dynamic(r: &Value, query: &[String]) -> Value {
    if let Some((first, rest)) = query.split_first() {
        if rest.is_empty() {
            if first == "*" {
                if r.is_array() {
                    eprintln!("a trailing * is unnecessary, only use with maps");
                    r.clone()
                } else {
                    panic!("this element is not an array, so cannot apply *");
                }
            } else {
                match *r {
                    Value::Array(ref a) => a.get(first.parse::<usize>().expect(&format!("index {} isn't an unsigned integer",first))).expect(&format!("index {} doesn't exist",first)),
                    Value::Object(ref o) => o.get(first).expect(&format!("index {} doesn't exist",first)),
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => panic!("tried to index a non-collection type")
                }.clone()
            }
        } else {
            if first == "*" {
                r.as_array().expect("this element is not an array, so cannot apply *").iter().map(|e|apply_query_dynamic(e,rest)).collect()
            } else {
                match *r {
                    Value::Array(ref a) => apply_query_dynamic(a.get(first.parse::<usize>().expect(&format!("index {} isn't an unsigned integer",first))).expect(&format!("index {} doesn't exist",first)),rest),
                    Value::Object(ref o) => apply_query_dynamic(o.get(first).expect(&format!("index {} doesn't exist",first)),rest),
                    Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => panic!("tried to index a non-collection type")
                }.clone()
            }
        }
    } else {
        panic!("called with empty query slice");
    }
}

fn schema(r: &Value) -> Value {
    match *r {
        Value::Null => r.clone(),
        Value::Bool(_) => json!("bool"),
        Value::Number(_) => json!("number"),
        Value::String(_) => json!("string"),
        Value::Array(ref a) => match a.first() {
            Some(one) => json!([schema(one)]),
            None => json!([])
        },
        Value::Object(ref o) => Value::Object(o.iter().map(|(k,v)|(k.clone(),schema(v))).collect())
    }
}

fn main() {
    std::panic::set_hook(Box::new(move |p|{
        if let Some(msg) = p.payload().downcast_ref::<&str>() {
            eprintln!("{}",msg);
        } else if let Some(msg) = p.payload().downcast_ref::<std::string::String>() {
            eprintln!("{}",msg);
        } else {
            eprintln!("Failed to get panic message.");
        }
        std::process::exit(-1);
    }));

    let mut args = std::env::args().skip(1).peekable();
    let literal_mode = args.peek() == Some(&"-F".to_string());
    let schema_mode = args.peek() == Some(&"-s".to_string());

    let stdin = std::io::stdin();
    let stdin = stdin.lock();

    let ret = if literal_mode {
        let query = args.skip(1).fold(String::new(),|mut s,a|{s.push_str(&a);s});
        apply_query(stdin, &query)
    } else if schema_mode {
        let value = serde_json::from_reader(stdin).expect("invalid json");
        schema(&value)
    } else {
        let query = args.fold(String::new(),|mut s,a|{s.push_str(&a);s});
        let value = serde_json::from_reader(stdin).expect("invalid json");
        apply_query_dynamic(
            &value,
            &split_with_escapes(query.as_bytes()).unwrap().1
        )
    };

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

// idea: allow * as wildcard for maps (keep keys?)
// idea: allow multiple queries

#[cfg(test)]
mod tests {
    use super::apply_query;
    use std::io::BufReader;

    #[test]
    fn test_get_value() {
        let s = r#"{
                        "name": "John Doe",
                        "age": 43,
                        "phones": [
                          "+44 1234567",
                          "+44 2345678"
                        ]
                      }"#;
        let data1 = BufReader::new(s.as_bytes());
        let data2 = BufReader::new(s.as_bytes());

        assert_eq!(json!("+44 1234567"), apply_query(data1, "phones.0").unwrap());
        assert_eq!(json!("John Doe"), apply_query(data2, "name").unwrap());
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
