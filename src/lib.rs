use std::str::FromStr;

use icalendar::Calendar;
use state::set_state;
use transformer::transform;

use worker::kv::KvStore;
use worker::*;

//mod durable;
mod state;
mod transformer;
mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

async fn fetch(url: Url) -> Result<String> {
    Fetch::Url(url).send().await?.text().await
}

pub fn parse_calendar(s: &str) -> worker::Result<Calendar> {
    Calendar::from_str(s).map_err(worker::Error::RustError)
}

async fn fetch_calendar(url: Url) -> Result<Calendar> {
    parse_calendar(&fetch(url).await?)
}

async fn response(url: Url, kv: KvStore) -> Result<Response> {
    let mut headers = Headers::new();
    //https://github.com/moodle/moodle/blob/master/calendar/export_execute.php
    headers.set("Pragma", "no-cache")?;
    headers.set("Content-disposition", "attachment; filename=calendar.ics")?;
    headers.set("Content-type", "text/calendar; charset=utf-8")?;

    let user_id = url
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "userid" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .unwrap();
    Ok(Response::from_body(ResponseBody::Body(
        state::compute_state(transform(fetch_calendar(url).await?)?, &user_id, kv)
            .await?
            .to_string()
            .into_bytes(),
    ))?
    .with_headers(headers))
}

async fn update_state(req: &mut Request, kv: &KvStore) -> Result<()> {
    let calendar = Calendar::from_str(&req.text().await?).map_err(worker::Error::RustError)?;
    let user_id = req
        .url()?
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "userid" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .unwrap();
    set_state(calendar, &user_id, kv).await?;
    Ok(())
}

fn get_params(req: &Request) -> Result<(String, String, String)> {
    let user_id = req
        .url()?
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "userid" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| worker::Error::RustError("userid missing".to_owned()))?;
    //preset_what
    let auth_token = req
        .url()?
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "authtoken" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| worker::Error::RustError("authtoken missing".to_owned()))?;
    let present_what = req
        .url()?
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "present_what" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| String::from("all"));
    Ok((user_id, auth_token, present_what))
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/", |_, _| Response::ok("Hello from Workers!"))
        .get_async("/transform/:url", |_req, ctx| async move {
            if let Some(encoded) = ctx.param("url") {
                let url = Url::parse(
                    std::str::from_utf8(
                        &base64::decode(encoded)
                            .map_err(|e| worker::Error::RustError(e.to_string()))?,
                    )
                    .map_err(|e| worker::Error::RustError(e.to_string()))?,
                )?;

                let kv = ctx.kv("TODO")?;
                return response(url, kv).await;
            }

            Response::error("Bad Request", 400)
        })
        .put_async("/transform/:url", |mut req, ctx| async move {
            if let Some(encoded) = ctx.param("url") {
                let url = Url::parse(
                    std::str::from_utf8(
                        &base64::decode(encoded)
                            .map_err(|e| worker::Error::RustError(e.to_string()))?,
                    )
                    .map_err(|e| worker::Error::RustError(e.to_string()))?,
                )?;

                let kv = ctx.kv("TODO")?;
                update_state(&mut req, &kv).await?;
                return response(url, kv).await;
            }

            Response::error("Bad Request", 400)
        })
        // https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid=69696&authtoken=longtokendata&preset_what=all&preset_time=custom
        .get_async("/fri", |req, ctx| async move {
            // get params
            let (user_id, auth_token, present_what) = get_params(&req)?;
            // fetch and merge
            let calendar = transformer::merge(
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=custom"
                ))?)
                .await?,
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=monthnow"
                ))?)
                .await?,
            )?;
            // get kv store
            let kv = ctx.kv("TODO")?;
            let mut headers = Headers::new();
            //https://github.com/moodle/moodle/blob/master/calendar/export_execute.php
            headers.set("Pragma", "no-cache")?;
            headers.set("Content-disposition", "attachment; filename=calendar.ics")?;
            headers.set("Content-type", "text/calendar; charset=utf-8")?;

            Ok(Response::from_body(ResponseBody::Body(
                    //state::compute_state(transform(calendar)?, &user_id, kv)
                        //.await?
                        calendar
                        .to_string()
                        .into_bytes(),
                ))?
                .with_headers(headers))
        })
        .put_async("/fri", |mut req, ctx| async move {
            // get params
            let (user_id, auth_token, present_what) = get_params(&req)?;
            // fetch and merge
            let calendar = transformer::merge(
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=custom"
                ))?)
                .await?,
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=monthnow"
                ))?)
                .await?,
            )?;

            let kv = ctx.kv("TODO")?;
            update_state(&mut req, &kv).await?;
            let mut headers = Headers::new();
            //https://github.com/moodle/moodle/blob/master/calendar/export_execute.php
            headers.set("Pragma", "no-cache")?;
            headers.set("Content-disposition", "attachment; filename=calendar.ics")?;
            headers.set("Content-type", "text/calendar; charset=utf-8")?;

            Ok(Response::from_body(ResponseBody::Body(
                    state::compute_state(transform(calendar)?, &user_id, kv)
                        .await?
                        .to_string()
                        .into_bytes(),
                ))?
                .with_headers(headers))
        })
        // fri2 endpoints or as what i call it friz
        // it should get username and password
        // so make sure we are using https
        /*.post_async("/friz", |req, ctx| async move {
            //req.url()?.set_password(password);
            // get params
            let (user_id, auth_token, present_what) = get_params(&req)?;
            // fetch and merge
            let calendar = transformer::merge(
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=custom"
                ))?)
                .await?,
                fetch_calendar(Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid={user_id}&authtoken={auth_token}&preset_what={present_what}&preset_time=monthnow"
                ))?)
                .await?,
            )?;
            // get kv store
            let kv = ctx.kv("TODO")?;
            let mut headers = Headers::new();
            //https://github.com/moodle/moodle/blob/master/calendar/export_execute.php
            headers.set("Pragma", "no-cache")?;
            headers.set("Content-disposition", "attachment; filename=calendar.ics")?;
            headers.set("Content-type", "text/calendar; charset=utf-8")?;

            Ok(Response::from_body(ResponseBody::Body(
                    //state::compute_state(transform(calendar)?, &user_id, kv)
                        //.await?
                        calendar
                        .to_string()
                        .into_bytes(),
                ))?
                .with_headers(headers))
        })*/
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}
