use icalendar::parser::unfold;
use icalendar::Calendar;

/*
pozor Obseg po meri (12/10/22 – 17/10/23) ne vsebuje določenih openerjev
*/
pub fn transform(s: String) -> worker::Result<String> {
    let unfolded = unfold(&s);
    let calendar = icalendar::parser::read_calendar(&unfolded).map_err(worker::Error::RustError)?;
    let mut new_calendar = Calendar::new();
    // copy global props (THEY ARE DOUBLED BY WRITER)
    //new_calendar.append_property(calendar.get);
    //println!("{:?}", calendar.properties);

    // in first pass we get all opener-closer events
    for component in &calendar.components {
        println!("------------------\n{:?}", component.properties);
    }
    Ok(new_calendar.to_string())
}

#[test]
fn test_file() {
    let s = std::fs::read_to_string("./private/file.ics").unwrap();
    //println!("{}", transform(s).unwrap());
    transform(s).unwrap();
}
