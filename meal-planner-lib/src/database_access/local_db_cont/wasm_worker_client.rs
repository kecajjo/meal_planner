use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType, console};

/// Thin convenience wrapper to spawn the DB worker and send typed requests over `postMessage`.
///
/// The worker must be built separately (see README notes) and exposed as a module script URL
/// that accepts JSON messages matching the `Request`/`Response` enums in `wasm_worker_main.rs`.
#[wasm_bindgen]
pub(super) struct DbWorkerHandle {
    worker: Worker,
}

#[wasm_bindgen]
impl DbWorkerHandle {
    /// Create a worker from a module URL (e.g., `new URL("./wasm_worker_main.js", import.meta.url)`).
    /// The constructor sets the worker type to `module` so it can import the wasm-bindgen glue.
    #[wasm_bindgen(constructor)]
    pub fn new(worker_script_url: &str) -> Result<DbWorkerHandle, JsValue> {
        let opts = WorkerOptions::new();
        opts.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(worker_script_url, &opts)?;
        console::log_1(&JsValue::from_str(
            "DbWorkerHandle: created worker; attaching debug logger",
        ));
        attach_debug_logger(&worker)?;
        Ok(DbWorkerHandle { worker })
    }

    /// Forward a typed request payload to the worker and await the next response message.
    /// You can build the request with `serde_json::json!` using the same shapes as
    /// `Request` in `wasm_worker_main.rs` (e.g., `{ "type": "GetProductsMatchingCriteria", "criteria": [...] }`).
    #[wasm_bindgen(js_name = send)]
    pub async fn send_raw(&self, request: JsValue) -> Result<JsValue, JsValue> {
        // Ensure the request is a string the worker expects.
        let request_text = match request.as_string() {
            Some(t) => t,
            None => js_sys::JSON::stringify(&request).and_then(|s| {
                s.as_string()
                    .ok_or_else(|| JsValue::from_str("Non-stringifiable request"))
            })?,
        };

        let serialized = JsValue::from_str(&request_text);
        let worker_ref: &Worker = &self.worker;
        let worker_ptr = worker_ref as *const Worker;

        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let reject_on_error = reject.clone();
            let reject_on_post = reject.clone();
            // onmessage stays installed and filters out Debug packets; resolves on first non-debug.
            let on_message =
                Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |evt: MessageEvent| {
                    let data = evt.data();
                    console::log_1(&JsValue::from_str("DbWorkerHandle: on_message fired"));
                    console::log_1(&data);

                    // Ignore any non-string packets (e.g., opfs-async-loaded objects) and keep waiting.
                    let Some(text) = data.as_string() else {
                        console::log_1(&JsValue::from_str(
                            "DbWorkerHandle: non-string message ignored",
                        ));
                        return;
                    };

                    // Skip debug chatter; keep listener alive for the real response.
                    if text.contains("\"type\":\"Debug\"") {
                        tracing::debug!("worker debug: {}", text);
                        console::log_1(&JsValue::from_str(&format!("worker debug: {}", text)));
                        return;
                    }

                    let worker = unsafe { &*worker_ptr };
                    worker.set_onmessage(None);
                    worker.set_onerror(None);
                    let _ = resolve.call1(&JsValue::UNDEFINED, &JsValue::from_str(&text));
                }));

            let on_error = Closure::<dyn FnMut(ErrorEvent)>::once(move |evt: ErrorEvent| {
                let worker = unsafe { &*worker_ptr };
                worker.set_onmessage(None);
                worker.set_onerror(None);
                let _ =
                    reject_on_error.call1(&JsValue::UNDEFINED, &JsValue::from_str(&evt.message()));
            });

            let worker = unsafe { &*worker_ptr };
            worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

            if let Err(err) = worker.post_message(&serialized) {
                worker.set_onmessage(None);
                worker.set_onerror(None);
                let _ = reject_on_post.call1(&JsValue::UNDEFINED, &err);
            }

            on_message.forget();
            on_error.forget();
        });

        JsFuture::from(promise).await
    }
}

fn attach_debug_logger(worker: &Worker) -> Result<(), JsValue> {
    console::log_1(&JsValue::from_str("DbWorkerHandle: attach_debug_logger"));
    let listener = Closure::<dyn FnMut(MessageEvent)>::wrap(Box::new(move |evt: MessageEvent| {
        if let Some(text) = evt.data().as_string() {
            if text.contains("\"type\":\"Debug\"") {
                tracing::debug!("worker debug: {}", text);
                console::log_1(&JsValue::from_str(&format!("worker debug: {}", text)));
            }
        }
    }));

    worker.add_event_listener_with_callback("message", listener.as_ref().unchecked_ref())?;
    listener.forget();
    Ok(())
}
