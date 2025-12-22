use std::{cell::RefCell, collections::HashMap, rc::Rc};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use strum::{EnumCount, IntoEnumIterator};
use wasm_bindgen::JsValue;

use crate::data_types::{
    AllowedUnits, AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients,
    MicroNutrientsType, Product, UnitData,
};
use crate::database_access::local_db_cont::wasm_worker_client::DbWorkerHandle;
use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};

const WORKER_URL: &str = "/meal-planner-lib/local-db/wasm_worker.js";

thread_local! {
    static WORKER: RefCell<Option<Rc<DbWorkerHandle>>> = RefCell::new(None);
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum WorkerResponse {
    Ok,
    Rows { rows: Vec<Map<String, Value>> },
    Err { message: String },
}

#[derive(Serialize)]
struct SqlStatement {
    sql: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    bind: Option<Vec<Value>>,
}

/// WASM implementation backed by the official SQLite WASM OPFS worker.
pub struct LocalProductDbConcrete {
    worker: Rc<DbWorkerHandle>,
    key: String,
}

impl LocalProductDbConcrete {
    fn get_or_create_worker() -> Result<Rc<DbWorkerHandle>, String> {
        WORKER
            .try_with(|cell| {
                if let Some(existing) = cell.borrow().as_ref() {
                    return Ok(existing.clone());
                }
                tracing::debug!("Creating wasm DB worker");
                let handle = DbWorkerHandle::new(WORKER_URL)
                    .map_err(|e| format!("Failed to create worker: {e:?}"))?;
                let handle = Rc::new(handle);
                *cell.borrow_mut() = Some(handle.clone());
                tracing::debug!("Worker created");
                Ok(handle)
            })
            .map_err(|_| "Failed to access worker cell".to_string())?
    }

    async fn send_request(
        worker: &DbWorkerHandle,
        req: &serde_json::Value,
    ) -> Result<WorkerResponse, String> {
        let payload =
            serde_json::to_string(req).map_err(|e| format!("Failed to serialise request: {e}"))?;

        let js_res = worker
            .send_raw(JsValue::from_str(&payload))
            .await
            .map_err(|e| format!("Worker request failed: {e:?}"))?;

        let text = js_res
            .as_string()
            .ok_or_else(|| "Worker response was not a string".to_string())?;

        serde_json::from_str(&text).map_err(|e| format!("Failed to parse worker response: {e}"))
    }

    async fn send_exec(&self, statements: Vec<SqlStatement>) -> Result<(), String> {
        let req = json!({
            "type": "Exec",
            "database_file": self.key,
            "statements": statements,
        });

        match Self::send_request(&self.worker, &req).await {
            Ok(WorkerResponse::Ok) => Ok(()),
            Ok(WorkerResponse::Err { message }) => Err(message),
            Ok(WorkerResponse::Rows { .. }) => Err("Unexpected rows for Exec".to_string()),
            Err(e) => Err(e),
        }
    }

    async fn send_query(&self, sql: String, bind: Vec<Value>) -> Result<Vec<Map<String, Value>>, String> {
        let req = json!({
            "type": "Query",
            "database_file": self.key,
            "sql": sql,
            "bind": bind,
        });

        match Self::send_request(&self.worker, &req).await {
            Ok(WorkerResponse::Rows { rows }) => Ok(rows),
            Ok(WorkerResponse::Ok) => Err("Query returned Ok without rows".to_string()),
            Ok(WorkerResponse::Err { message }) => Err(message),
            Err(e) => Err(e),
        }
    }

    fn map_row_to_product(row: &Map<String, Value>) -> Result<(String, Product), String> {
        let id = Self::get_string(row, "id")?;
        let name = Self::get_string(row, "name")?;
        let brand = Self::get_string_opt(row, "brand")?;

        let mut macro_values = Vec::new();
        for macro_type in MacroElementsType::iter() {
            if macro_type == MacroElementsType::Calories {
                continue;
            }
            let key = macro_type.to_string();
            let val = Self::get_f32(row, &key)?;
            macro_values.push(val);
        }
        let macro_elems = MacroElements::new(
            macro_values[0],
            macro_values[1],
            macro_values[2],
            macro_values[3],
            macro_values[4],
        );

        let mut micro = Box::new(MicroNutrients::default());
        for micro_type in MicroNutrientsType::iter() {
            let key = micro_type.to_string();
            micro[micro_type] = Self::get_f32_opt(row, &key)?;
        }

        let mut allowed: AllowedUnits = HashMap::new();
        for unit in AllowedUnitsType::iter() {
            let base = unit.to_string();
            let amt_key = base.clone();
            let div_key = format!("{base} divider");
            let amount = Self::get_u16_opt(row, &amt_key)?;
            let divider = Self::get_u16_opt(row, &div_key)?;
            if let (Some(amount), Some(divider)) = (amount, divider) {
                allowed.insert(unit, UnitData { amount, divider });
            }
        }

        let product = Product::new(name, brand, Box::new(macro_elems), micro, allowed);
        Ok((id, product))
    }

    fn get_string(row: &Map<String, Value>, key: &str) -> Result<String, String> {
        Self::get_string_opt(row, key)?.ok_or_else(|| format!("Missing string column '{key}'"))
    }

    fn get_string_opt(row: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
        match row.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(v) => Ok(Some(v.to_string())),
        }
    }

    fn get_f32(row: &Map<String, Value>, key: &str) -> Result<f32, String> {
        Self::get_f32_opt(row, key)?.ok_or_else(|| format!("Missing float column '{key}'"))
    }

    fn get_f32_opt(row: &Map<String, Value>, key: &str) -> Result<Option<f32>, String> {
        match row.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(n)) => n
                .as_f64()
                .map(|f| f as f32)
                .ok_or_else(|| format!("Invalid number for '{key}'"))
                .map(Some),
            Some(v) => Err(format!("Unexpected type for '{key}': {v}")),
        }
    }

    fn get_u16_opt(row: &Map<String, Value>, key: &str) -> Result<Option<u16>, String> {
        match row.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Number(n)) => n
                .as_u64()
                .ok_or_else(|| format!("Invalid integer for '{key}'"))
                .and_then(|v| u16::try_from(v).map_err(|_| format!("Out of range for '{key}'")))
                .map(Some),
            Some(v) => Err(format!("Unexpected type for '{key}': {v}")),
        }
    }

    /// Create a new DB handle backed by the OPFS worker.
    pub async fn new(key: &str) -> Option<Self> {
        let worker = Self::get_or_create_worker().ok()?;

        let db = Self {
            worker,
            key: key.to_string(),
        };

        if let Err(e) = db.init_db().await {
            tracing::error!("Failed to initialise wasm local DB: {e}");
            return None;
        }
        Some(db)
    }

    async fn init_db(&self) -> Result<(), String> {
        let init_req = json!({
            "type": "InitDbFile",
            "database_file": self.key,
        });
        match Self::send_request(&self.worker, &init_req).await {
            Ok(WorkerResponse::Ok) => {
                tracing::debug!("Worker init succeeded");
            }
            Ok(WorkerResponse::Err { message }) => {
                return Err(format!("worker init error: {message}"));
            }
            Ok(other) => {
                return Err(format!("worker init unexpected response: {:?}", other));
            }
            Err(e) => {
                return Err(format!("worker init failed: {e}"));
            }
        }

        self.send_exec(schema_statements()).await
    }
}

#[async_trait::async_trait(?Send)]
impl Database for LocalProductDbConcrete {
    async fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, Product> {
        let (sql, bind) = build_select_query(criteria);
        match self.send_query(sql, bind).await {
            Ok(rows) => rows
                .into_iter()
                .filter_map(|row| match Self::map_row_to_product(&row) {
                    Ok((id, product)) => Some((id, product)),
                    Err(e) => {
                        tracing::error!("Failed to map row to product: {e}");
                        None
                    }
                })
                .collect(),
            Err(e) => {
                tracing::error!("worker query failed: {e}");
                HashMap::new()
            }
        }
    }

    async fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: AllowedUnitsType,
        unit_data: UnitData,
    ) -> Result<(), String> {
        let stmt = SqlStatement {
            sql: format!(
                "UPDATE allowed_units SET \"{col}\" = ?, \"{col} divider\" = ? WHERE id = ?;",
                col = allowed_unit.to_string()
            ),
            bind: Some(vec![
                unit_data.amount.into(),
                unit_data.divider.into(),
                product_id.into(),
            ]),
        };
        self.send_exec(vec![stmt]).await
    }
}

#[async_trait::async_trait(?Send)]
impl MutableDatabase for LocalProductDbConcrete {
    async fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let stmts = build_insert_statements(product_id, &product);
        self.send_exec(stmts).await
    }

    async fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let mut stmts = Vec::new();
        stmts.push(SqlStatement {
            sql: "UPDATE products SET name = ?, brand = ? WHERE id = ?;".to_string(),
            bind: Some(vec![
                product.name().into(),
                product.brand().map(|b| b.into()).unwrap_or(Value::Null),
                product_id.into(),
            ]),
        });

        for macro_type in MacroElementsType::iter() {
            if macro_type == MacroElementsType::Calories {
                continue;
            }
            stmts.push(SqlStatement {
                sql: format!(
                    "UPDATE macro_elements SET \"{col}\" = ? WHERE id = ?;",
                    col = macro_type.to_string()
                ),
                bind: Some(vec![
                    product.macro_elements[macro_type].into(),
                    product_id.into(),
                ]),
            });
        }

        for micro_type in MicroNutrientsType::iter() {
            let val = match product.micro_nutrients[micro_type] {
                Some(v) => Value::from(v),
                None => Value::Null,
            };
            stmts.push(SqlStatement {
                sql: format!(
                    "UPDATE micronutrients SET \"{col}\" = ? WHERE id = ?;",
                    col = micro_type.to_string()
                ),
                bind: Some(vec![val, product_id.into()]),
            });
        }

        for unit in AllowedUnitsType::iter() {
            let entry = product.allowed_units.get(&unit);
            let amount = entry
                .map(|u| u.amount)
                .map(Value::from)
                .unwrap_or(Value::Null);
            let divider = entry
                .map(|u| u.divider)
                .map(Value::from)
                .unwrap_or(Value::Null);
            let col = unit.to_string();
            stmts.push(SqlStatement {
                sql: format!(
                    "UPDATE allowed_units SET \"{col}\" = ?, \"{col} divider\" = ? WHERE id = ?;"
                ),
                bind: Some(vec![amount, divider, product_id.into()]),
            });
        }

        self.send_exec(stmts).await
    }

    async fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
        let stmt = SqlStatement {
            sql: "DELETE FROM products WHERE id = ?;".to_string(),
            bind: Some(vec![product_id.into()]),
        };
        self.send_exec(vec![stmt]).await
    }
}

fn build_select_query(criteria: &[DbSearchCriteria]) -> (String, Vec<Value>) {
    let mut sql = format!(
        "SELECT p.id, p.name, p.brand, {} , {} , {} FROM products p \
         INNER JOIN macro_elements me ON p.id = me.id \
         INNER JOIN allowed_units au ON p.id = au.id \
         LEFT JOIN micronutrients mn ON p.id = mn.id",
        macro_columns_select(),
        micro_columns_select(),
        allowed_columns_select()
    );

    let mut bind = Vec::new();
    if criteria.is_empty() {
        sql.push_str(";");
        return (sql, bind);
    }

    // Only ById supported today
    if let Some(DbSearchCriteria::ById(name)) = criteria.first() {
        sql.push_str(" WHERE p.name LIKE ? || '%';");
        bind.push(Value::from(name.clone()));
    } else {
        sql.push_str(";");
    }

    (sql, bind)
}

fn macro_columns_select() -> String {
    MacroElementsType::iter()
        .filter(|m| *m != MacroElementsType::Calories)
        .map(|m| format!("me.\"{col}\" AS \"{col}\"", col = m.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn micro_columns_select() -> String {
    MicroNutrientsType::iter()
        .map(|m| format!("mn.\"{col}\" AS \"{col}\"", col = m.to_string()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn allowed_columns_select() -> String {
    AllowedUnitsType::iter()
        .flat_map(|u| {
            let base = u.to_string();
            [
                format!("au.\"{col}\" AS \"{col}\"", col = base),
                format!("au.\"{col} divider\" AS \"{col} divider\"", col = base),
            ]
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn build_insert_statements(product_id: &str, product: &Product) -> Vec<SqlStatement> {
    let mut stmts = Vec::new();
    stmts.push(SqlStatement {
        sql: "INSERT OR REPLACE INTO products (id, name, brand) VALUES (?, ?, ?);".to_string(),
        bind: Some(vec![
            product_id.into(),
            product.name().into(),
            product.brand().map(|b| b.into()).unwrap_or(Value::Null),
        ]),
    });

    let macro_binds: Vec<Value> = MacroElementsType::iter()
        .filter(|m| *m != MacroElementsType::Calories)
        .map(|m| Value::from(product.macro_elements[m]))
        .collect();
    let macro_cols = MacroElementsType::iter()
        .filter(|m| *m != MacroElementsType::Calories)
        .map(|m| format!("\"{col}\"", col = m.to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    let macro_placeholders = std::iter::repeat("?")
        .take(macro_binds.len() + 1)
        .collect::<Vec<_>>()
        .join(", ");
    let mut macro_bind_all = Vec::with_capacity(macro_binds.len() + 1);
    macro_bind_all.push(Value::from(product_id));
    macro_bind_all.extend(macro_binds);
    stmts.push(SqlStatement {
        sql: format!(
            "INSERT OR REPLACE INTO macro_elements (id, {cols}) VALUES ({ph});",
            cols = macro_cols,
            ph = macro_placeholders
        ),
        bind: Some(macro_bind_all),
    });

    let micro_cols = MicroNutrientsType::iter()
        .map(|m| format!("\"{col}\"", col = m.to_string()))
        .collect::<Vec<_>>()
        .join(", ");
    let micro_placeholders = std::iter::repeat("?")
        .take(MicroNutrientsType::COUNT + 1)
        .collect::<Vec<_>>()
        .join(", ");
    let mut micro_bind_all = Vec::with_capacity(MicroNutrientsType::COUNT + 1);
    micro_bind_all.push(Value::from(product_id));
    for micro in MicroNutrientsType::iter() {
        let val = match product.micro_nutrients[micro] {
            Some(v) => Value::from(v),
            None => Value::Null,
        };
        micro_bind_all.push(val);
    }
    stmts.push(SqlStatement {
        sql: format!(
            "INSERT OR REPLACE INTO micronutrients (id, {cols}) VALUES ({ph});",
            cols = micro_cols,
            ph = micro_placeholders
        ),
        bind: Some(micro_bind_all),
    });

    let allowed_cols = AllowedUnitsType::iter()
        .flat_map(|u| {
            let base = u.to_string();
            vec![
                format!("\"{col}\"", col = base),
                format!("\"{col} divider\"", col = base),
            ]
        })
        .collect::<Vec<_>>()
        .join(", ");
    let allowed_placeholders = std::iter::repeat("?")
        .take((AllowedUnitsType::COUNT * 2) + 1)
        .collect::<Vec<_>>()
        .join(", ");
    let mut allowed_bind_all = Vec::with_capacity((AllowedUnitsType::COUNT * 2) + 1);
    allowed_bind_all.push(Value::from(product_id));
    for unit in AllowedUnitsType::iter() {
        let entry = product.allowed_units.get(&unit);
        allowed_bind_all.push(entry.map(|u| Value::from(u.amount)).unwrap_or(Value::Null));
        allowed_bind_all.push(entry.map(|u| Value::from(u.divider)).unwrap_or(Value::Null));
    }
    stmts.push(SqlStatement {
        sql: format!(
            "INSERT OR REPLACE INTO allowed_units (id, {cols}) VALUES ({ph});",
            cols = allowed_cols,
            ph = allowed_placeholders
        ),
        bind: Some(allowed_bind_all),
    });

    stmts
}

fn schema_statements() -> Vec<SqlStatement> {
    vec![
        SqlStatement {
            sql: "PRAGMA foreign_keys=ON;".to_string(),
            bind: None,
        },
        SqlStatement {
            sql: r#"CREATE TABLE IF NOT EXISTS products (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    brand TEXT
);"#
            .to_string(),
            bind: None,
        },
        SqlStatement {
            sql: r#"CREATE TABLE IF NOT EXISTS macro_elements (
    id TEXT NOT NULL PRIMARY KEY,
    "Fat" FLOAT NOT NULL,
    "Saturated Fat" FLOAT NOT NULL,
    "Carbohydrates" FLOAT NOT NULL,
    "Sugar" FLOAT NOT NULL,
    "Protein" FLOAT NOT NULL,
    FOREIGN KEY(id) REFERENCES products(id) ON DELETE CASCADE
);"#
            .to_string(),
            bind: None,
        },
        SqlStatement {
            sql: r#"CREATE TABLE IF NOT EXISTS micronutrients (
    id TEXT NOT NULL PRIMARY KEY,
    "Fiber" FLOAT,
    "Zinc" FLOAT,
    "Sodium" FLOAT,
    "Alcohol" FLOAT,
    FOREIGN KEY(id) REFERENCES products(id) ON DELETE CASCADE
);"#
            .to_string(),
            bind: None,
        },
        SqlStatement {
            sql: r#"CREATE TABLE IF NOT EXISTS allowed_units (
    id TEXT NOT NULL PRIMARY KEY,
    "gram" INTEGER NOT NULL DEFAULT 1,
    "gram divider" INTEGER NOT NULL DEFAULT 1,
    "piece" INTEGER,
    "piece divider" INTEGER,
    "cup" INTEGER,
    "cup divider" INTEGER,
    "tablespoon" INTEGER,
    "tablespoon divider" INTEGER,
    "teaspoon" INTEGER,
    "teaspoon divider" INTEGER,
    "box" INTEGER,
    "box divider" INTEGER,
    "custom" INTEGER,
    "custom divider" INTEGER,
    FOREIGN KEY(id) REFERENCES products(id) ON DELETE CASCADE
);"#
            .to_string(),
            bind: None,
        },
    ]
}
