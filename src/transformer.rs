use std::str::FromStr;

use icalendar::{Calendar, Component};

/*
pozor Obseg po meri (12/10/22 – 17/10/23) ne vsebuje določenih openerjev
*/
pub fn transform(s: String) -> worker::Result<String> {
    let calendar = Calendar::from_str(&s).map_err(worker::Error::RustError)?;
    let mut new_calendar = Calendar::new();

    // copy global props (THEY ARE DOUBLED BY WRITER)
    //new_calendar.append_property(calendar.get);
    //println!("{:?}", calendar.properties);

    // in first pass we get all opener-closer events
    for component in calendar.iter() {
        //println!("------------------\n{:?}", component);
        if let Some(e) = component.as_event() {
            let summary = e.get_summary().unwrap().to_ascii_lowercase();
            // preveri če je opener
            if summary.contains("odpre") {
                let uid = e.get_uid().unwrap();
                // najdi closerja
                let next_uid = next_uid(uid);
                let next = calendar
                    .iter()
                    .filter_map(|x| x.as_event())
                    .find(|x| x.get_uid().unwrap() == next_uid);
                if let Some(n) = next {
                    // ustvarimo TODO
                    let mut todo = icalendar::Todo::new();
                    /*for prop in e.multi_properties() {
                        todo.append_property(prop.to_owned());
                    }*/
                    todo.summary(e.get_summary().unwrap())
                        .class(e.get_class().unwrap())
                        .add_multi_property(
                            "CATEGORIES",
                            e.properties().get("CATEGORIES").unwrap().value(),
                        )
                        .uid(uid)
                        .description(e.get_description().unwrap())
                        .starts(e.get_start().unwrap())
                        .ends(n.get_end().unwrap());
                    new_calendar.push(todo);
                }
            }
        }
    }
    Ok(new_calendar.to_string())
}

fn next_uid(uid: &str) -> String {
    let (s1, s2) = uid.split_once('@').unwrap();
    let num: usize = s1.parse().unwrap();
    (num + 1).to_string() + "@" + s2
}

#[test]
fn test_file() {
    let s = std::fs::read_to_string("./private/file.ics").unwrap();
    println!("{}", transform(s).unwrap());
    //transform(s).unwrap();
}
