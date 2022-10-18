use transformer::transform;

use worker::kv::KvStore;
use worker::*;

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

async fn response(url: Url, kv: KvStore) -> Result<Response> {
    let mut headers = Headers::new();
    //https://github.com/moodle/moodle/blob/master/calendar/export_execute.php
    headers.set("Pragma", "no-cache")?;
    headers.set("Content-disposition", "attachment; filename=calendar.ics")?;
    headers.set("Content-type", "text/calendar; charset=utf-8")?;

    let user_id = url
        .query_pairs()
        .find_map(|(p, q)| {
            if p == "user_id" {
                Some(q.to_string())
            } else {
                None
            }
        })
        .unwrap();
    Ok(Response::from_body(ResponseBody::Body(
        state::compute_state(transform(fetch(url).await?)?, &user_id, kv)
            .await?
            .to_string()
            .into_bytes(),
    ))?
    .with_headers(headers))
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
        .post_async("/transform/:url", |req, ctx| async move {
            console_log!("{:?}", req);
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
        // https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid=69696&authtoken=longtokendata&preset_what=all&preset_time=custom
        .get_async("/fri", |req, ctx| async move {
            if let Some(q) = req.url()?.query() {
                let url = Url::parse(&format!(
                    "https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?{q}"
                ))?;

                let kv = ctx.kv("TODO")?;
                return response(url, kv).await;
            }

            Response::error("Bad Request", 400)
        })
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}
