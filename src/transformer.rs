pub fn transform(s: String) -> String {
    s
}

#[test]
fn test_file() {
    let s = std::fs::read_to_string("./private/file.ics").unwrap();
    println!("{}", transform(s));
}
