use icalendar::{Calendar, CalendarComponent, Component};
use worker::kv::KvStore;

pub async fn compute_state(
    mut calendar: Calendar,
    user_id: &str,
    kv: KvStore,
) -> worker::Result<Calendar> {
    for component in calendar.iter_mut() {
        if let CalendarComponent::Todo(t) = component {
            let uid = t.get_uid().unwrap();
            if let Some(val) = kv.get(&(user_id.to_owned() + uid)).text().await? {
                let (dts, pcs) = val.split_once(';').unwrap();
                t.completed(dts.parse().unwrap());
                t.percent_complete(pcs.parse().unwrap());
            }
        }
    }
    //kv.put("key", "value")?.execute().await?;
    Ok(calendar)
}

pub async fn set_state(calendar: Calendar, user_id: &str, kv: &KvStore) -> worker::Result<()> {
    for component in calendar.iter() {
        if let CalendarComponent::Todo(t) = component {
            let uid = t.get_uid().unwrap();
            kv.put(
                &(user_id.to_owned() + uid),
                t.get_completed().unwrap().to_string()
                    + ";"
                    + &t.get_percent_complete().unwrap().to_string(),
            )?
            .execute()
            .await?;
        }
    }
    //kv.put("key", "value")?.execute().await?;
    Ok(())
}
