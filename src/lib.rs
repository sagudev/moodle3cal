use worker::*;

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
                let url = String::from_utf8(
                    base64::decode(encoded).map_err(|e| worker::Error::RustError(e.to_string()))?,
                )
                .map_err(|e| worker::Error::RustError(e.to_string()))?;
                return Response::error(url, 400);
            }

            Response::error("Bad Request", 400)
        })
        // https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?userid=69696&authtoken=longtokendata&preset_what=all&preset_time=custom
        .get_async("/fri", |req, _ctx| async move {
            if let Some(q) = req.url()?.query() {
                let url = format!("https://ucilnica.fri.uni-lj.si/calendar/export_execute.php?{q}");
                return Response::error(url, 400);
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
