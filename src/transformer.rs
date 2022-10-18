use std::collections::HashSet;
use std::str::FromStr;

use icalendar::{
    Calendar, CalendarComponent, CalendarDateTime, Component, DatePerhapsTime, Event, Todo,
};

/*
pozor Obseg po meri (12/10/22 – 17/10/23) ne vsebuje določenih openerjev
*/
pub fn transform(s: String) -> worker::Result<Calendar> {
    let calendar = Calendar::from_str(&s).map_err(worker::Error::RustError)?;
    let mut new_calendar = Calendar::new();
    let mut consumed_uids: HashSet<String> = HashSet::new();

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
                    let mut todo = event_to_todo(e);
                    todo.ends(n.get_end().unwrap());
                    todo.due(n.get_end().unwrap());
                    //todo.description("cok");
                    new_calendar.push(todo);
                    consumed_uids.insert(uid.to_owned());
                    consumed_uids.insert(next_uid);
                }
            }
        }
    }
    let mut consumed_uids2 = consumed_uids.clone();
    // in second pass we deal with the rest
    for component in calendar
        .iter()
        .filter(|x| !consumed_uids.contains(get_uid(x).unwrap()))
    {
        if let Some(e) = component.as_event() {
            // ustvarimo todo
            let mut todo = event_to_todo(e);
            // as we have closers in second pass we set their start date to last modified
            todo.starts(try_from(
                e.properties().get("LAST-MODIFIED").unwrap().value(),
            )?);
            //todo.description("sok");
            consumed_uids2.insert(e.get_uid().unwrap().to_owned());
            new_calendar.push(todo);
        }
    }
    // for the third pass just copy all unconsumed events
    /*for component in calendar
        .iter()
        .filter(|x| !consumed_uids2.contains(get_uid(x).unwrap()))
    {
        println!("{:?}", component)
        //new_calendar.push(component.clone());
    }*/
    Ok(new_calendar)
}

fn next_uid(uid: &str) -> String {
    let (s1, s2) = uid.split_once('@').unwrap();
    let num: usize = s1.parse().unwrap();
    (num + 1).to_string() + "@" + s2
}

fn get_uid(c: &CalendarComponent) -> Option<&str> {
    if let Some(e) = c.as_event() {
        e.get_uid()
    } else if let Some(t) = c.as_todo() {
        t.get_uid()
    } else {
        None
    }
}

fn event_to_todo(e: &Event) -> Todo {
    icalendar::Todo::new()
        .summary(e.get_summary().unwrap())
        .add_multi_property(
            "CATEGORIES",
            e.properties().get("CATEGORIES").unwrap().value(),
        )
        .description(e.get_description().unwrap())
        .starts(e.get_start().unwrap())
        .ends(e.get_end().unwrap())
        .due(e.get_end().unwrap())
        .uid(e.get_uid().unwrap())
        .timestamp(e.get_timestamp().unwrap())
        .class(e.get_class().unwrap())
        .done()
}

// refixed from icalendar
fn try_from(val: &str) -> Result<DatePerhapsTime, &'static str> {
    use chrono::*;
    // UTC is here first because lots of fields MUST be UTC, so it should,
    // in practice, be more common that others.
    if let Ok(utc_dt) = Utc.datetime_from_str(val, "%Y%m%dT%H%M%SZ") {
        return Ok(DatePerhapsTime::DateTime(CalendarDateTime::Utc(utc_dt)));
    };

    if let Ok(naive_date) = NaiveDate::parse_from_str(val, "%Y%m%d") {
        return Ok(DatePerhapsTime::Date(naive_date));
    };

    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(val, "%Y%m%dT%H%M%S") {
        return Ok(DatePerhapsTime::DateTime(CalendarDateTime::Floating(
            naive_dt,
        )));
    };

    Err("Value does not look like a known DATE-TIME")
}

#[test]
fn test_file() {
    let s = std::fs::read_to_string("./private/file.ics").unwrap();
    println!("{}", transform(s).unwrap());
    //transform(s).unwrap();
}
