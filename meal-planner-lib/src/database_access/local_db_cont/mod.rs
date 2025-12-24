pub(super) mod local_db;

#[cfg(not(target_arch = "wasm32"))]
mod local_db_generic;
#[cfg(target_arch = "wasm32")]
mod local_db_wasm;
#[cfg(target_arch = "wasm32")]
mod wasm_worker_client;
