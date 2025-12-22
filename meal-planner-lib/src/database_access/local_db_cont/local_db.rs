#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::collections::HashMap;

    use crate::data_types::{AllowedUnitsType, Product, UnitData};
    use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};

    use crate::database_access::local_db_cont::local_db_concrete;

    pub struct LocalProductDb {
        inner: local_db_concrete::LocalProductDb,
    }

    impl LocalProductDb {
        /// Creates a new local database instance backed by `SQLite`.
        pub fn new(database_file: &str) -> Option<Self> {
            local_db_concrete::LocalProductDb::new(database_file).map(|inner| Self { inner })
        }
    }

    impl Database for LocalProductDb {
        fn get_products_matching_criteria(
            &self,
            criteria: &[DbSearchCriteria],
        ) -> HashMap<String, Product> {
            self.inner.get_products_matching_criteria(criteria)
        }

        fn set_product_unit(
            &mut self,
            product_id: &str,
            allowed_unit: AllowedUnitsType,
            unit_data: UnitData,
        ) -> Result<(), String> {
            self.inner
                .set_product_unit(product_id, allowed_unit, unit_data)
        }
    }

    impl MutableDatabase for LocalProductDb {
        fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
            self.inner.add_product(product_id, product)
        }

        fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
            self.inner.update_product(product_id, product)
        }

        fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
            self.inner.delete_product(product_id)
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use futures::executor::block_on;
    use serde::Deserialize;
    use serde_json::{json, to_value};
    use wasm_bindgen::JsValue;

    use crate::database_access::local_db_cont::wasm_worker_client::DbWorkerHandle;

    use crate::data_types::{AllowedUnitsType, Product, UnitData};
    use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};

    thread_local! {
        static WORKERS: RefCell<HashMap<String, Rc<DbWorkerHandle>>> = RefCell::new(HashMap::new());
    }

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum WorkerResponse {
        Ok,
        Products { items: HashMap<String, Product> },
        Err { message: String },
    }

    /// WASM implementation backed by a shared worker pool.
    pub struct LocalProductDb {
        worker: Rc<DbWorkerHandle>,
        key: String,
    }

    impl LocalProductDb {
        fn get_or_create_worker(key: &str) -> Result<Rc<DbWorkerHandle>, String> {
            WORKERS
                .try_with(|cell| {
                    let mut map = cell.borrow_mut();
                    if let Some(existing) = map.get(key) {
                        return Ok(existing.clone());
                    }

                    let handle = DbWorkerHandle::new(key)
                        .map_err(|e| format!("Failed to create worker: {e:?}"))?;
                    let handle = Rc::new(handle);
                    map.insert(key.to_string(), handle.clone());
                    Ok(handle)
                })
                .map_err(|_| "Failed to access worker map".to_string())?
        }

        fn send_request(
            worker: &DbWorkerHandle,
            req: &serde_json::Value,
        ) -> Result<WorkerResponse, String> {
            let payload = serde_json::to_string(&req)
                .map_err(|e| format!("Failed to serialise request: {e}"))?;

            let js_res = block_on(worker.send_raw(JsValue::from_str(&payload)))
                .map_err(|e| format!("Worker request failed: {e:?}"))?;

            let text = js_res
                .as_string()
                .ok_or_else(|| "Worker response was not a string".to_string())?;

            serde_json::from_str(&text).map_err(|e| format!("Failed to parse worker response: {e}"))
        }

        pub fn new(key: &str) -> Option<Self> {
            let worker = Self::get_or_create_worker(key).ok()?;
            if let Err(e) = block_on(worker.init(key.to_string())) {
                // Failed to initialise; do not create DB instance.
                tracing::error!("worker init failed: {e:?}");
                return None;
            }

            Some(Self {
                worker,
                key: key.to_string(),
            })
        }
    }

    impl Database for LocalProductDb {
        fn get_products_matching_criteria(
            &self,
            criteria: &[DbSearchCriteria],
        ) -> HashMap<String, Product> {
            let criteria_json = match to_value(criteria) {
                Ok(val) => val,
                Err(e) => {
                    tracing::error!("failed to serialise criteria: {e}");
                    json!([])
                }
            };

            let req = json!({
                "type": "GetProductsMatchingCriteria",
                "criteria": criteria_json,
            });

            match Self::send_request(&self.worker, &req) {
                Ok(WorkerResponse::Products { items }) => items,
                Ok(WorkerResponse::Ok) => HashMap::new(),
                Ok(WorkerResponse::Err { message }) => {
                    tracing::error!("worker error: {message}");
                    HashMap::new()
                }
                Err(e) => {
                    tracing::error!("worker request failed: {e}");
                    HashMap::new()
                }
            }
        }

        fn set_product_unit(
            &mut self,
            product_id: &str,
            allowed_unit: AllowedUnitsType,
            unit_data: UnitData,
        ) -> Result<(), String> {
            let req = json!({
                "type": "SetProductUnit",
                "product_id": product_id,
                "allowed_unit": allowed_unit,
                "unit_data": unit_data,
            });

            match Self::send_request(&self.worker, &req) {
                Ok(WorkerResponse::Ok) => Ok(()),
                Ok(WorkerResponse::Err { message }) => Err(message),
                Ok(WorkerResponse::Products { .. }) => {
                    Err("Unexpected worker response".to_string())
                }
                Err(e) => Err(e),
            }
        }
    }

    impl MutableDatabase for LocalProductDb {
        fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
            let req = json!({
                "type": "AddProduct",
                "product_id": product_id,
                "product": product,
            });
            match Self::send_request(&self.worker, &req) {
                Ok(WorkerResponse::Ok) => Ok(()),
                Ok(WorkerResponse::Err { message }) => Err(message),
                Ok(WorkerResponse::Products { .. }) => {
                    Err("Unexpected worker response".to_string())
                }
                Err(e) => Err(e),
            }
        }

        fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
            let req = json!({
                "type": "UpdateProduct",
                "product_id": product_id,
                "product": product,
            });
            match Self::send_request(&self.worker, &req) {
                Ok(WorkerResponse::Ok) => Ok(()),
                Ok(WorkerResponse::Err { message }) => Err(message),
                Ok(WorkerResponse::Products { .. }) => {
                    Err("Unexpected worker response".to_string())
                }
                Err(e) => Err(e),
            }
        }

        fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
            let req = json!({
                "type": "DeleteProduct",
                "product_id": product_id,
            });
            match Self::send_request(&self.worker, &req) {
                Ok(WorkerResponse::Ok) => Ok(()),
                Ok(WorkerResponse::Err { message }) => Err(message),
                Ok(WorkerResponse::Products { .. }) => {
                    Err("Unexpected worker response".to_string())
                }
                Err(e) => Err(e),
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::LocalProductDb;
#[cfg(target_arch = "wasm32")]
pub use wasm::LocalProductDb;
