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
            // there is now completed field if its not completed
            if let Some(completed) = t.get_completed() {
                kv.put(
                    &(user_id.to_owned() + uid),
                    completed.to_string()
                        + ";"
                        + &t.get_percent_complete().unwrap_or(0).to_string(),
                )?
                .execute()
                .await?;
            }
        }
    }
    //kv.put("key", "value")?.execute().await?;
    Ok(())
}
