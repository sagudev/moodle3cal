use std::collections::HashMap;
// toli bi bilo bolj varno kot MailExtention.
use worker::*;

#[durable_object]
pub struct Cache {
    /// user_id => task_id => task_completion in % (0, 25, 50, 75, 100 = done)
    storage: HashMap<String, HashMap<String, u8>>,
    state: State,
    env: Env, // access `Env` across requests, use inside `fetch`
}

#[durable_object]
impl DurableObject for Cache {
    fn new(state: State, env: Env) -> Self {
        Self {
            storage: HashMap::new(),
            state,
            env,
        }
    }

    async fn fetch(&mut self, _req: Request) -> Result<Response> {
        // do some work when a worker makes a request to this
        Response::ok(&format!("Storing data for {} users", self.storage.len()))
    }
}
