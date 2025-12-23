use core::fmt;
use core::fmt::Write;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::ffi::{CStr, CString};
use std::marker::PhantomData;
use std::os::raw::c_void;
use std::ptr;
use std::slice;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::EnumIter;

use crate::data_types::{
    AllowedUnits, AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients,
    MicroNutrientsType, Product, UnitData,
};
use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};

use libsqlite3_sys as ffi;

#[cfg(test)]
pub(crate) const DATABASE_FILENAME: &str = "src/database_access/local_db_cont/test_local_db.sqlite";

struct SqliteConnection {
    raw: *mut ffi::sqlite3,
}

impl SqliteConnection {
    fn open(path: &str) -> Result<Self, String> {
        let c_path = CString::new(path)
            .map_err(|_| "Database path contains interior null byte".to_string())?;
        let mut db_ptr: *mut ffi::sqlite3 = ptr::null_mut();
        let flags = ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_READWRITE;
        let rc =
            unsafe { ffi::sqlite3_open_v2(c_path.as_ptr(), &raw mut db_ptr, flags, ptr::null()) };
        if rc != ffi::SQLITE_OK {
            let message = Self::extract_errmsg(db_ptr, rc);
            if !db_ptr.is_null() {
                unsafe {
                    ffi::sqlite3_close(db_ptr);
                }
            }
            tracing::error!("Failed to open SQLite database at '{c_path:?}': {message}");
            return Err(message);
        }
        tracing::debug!("Opened SQLite database at '{c_path:?}' successfully");
        Ok(Self { raw: db_ptr })
    }

    fn enable_foreign_keys(&self) -> Result<(), String> {
        self.execute("PRAGMA foreign_keys = ON;")
    }

    fn execute(&self, sql: &str) -> Result<(), String> {
        let c_sql = CString::new(sql).map_err(|_| "SQL contains interior null byte".to_string())?;
        let rc = unsafe {
            ffi::sqlite3_exec(
                self.raw,
                c_sql.as_ptr(),
                None,
                ptr::null_mut::<c_void>(),
                ptr::null_mut(),
            )
        };
        if rc != ffi::SQLITE_OK {
            return Err(self.last_error(rc));
        }
        Ok(())
    }

    fn prepare(&self, sql: &str) -> Result<Statement<'_>, String> {
        let c_sql = CString::new(sql).map_err(|_| "SQL contains interior null byte".to_string())?;
        let mut stmt_ptr: *mut ffi::sqlite3_stmt = ptr::null_mut();
        let rc = unsafe {
            ffi::sqlite3_prepare_v2(
                self.raw,
                c_sql.as_ptr(),
                -1,
                &raw mut stmt_ptr,
                ptr::null_mut(),
            )
        };
        if rc != ffi::SQLITE_OK {
            return Err(self.last_error(rc));
        }
        Ok(Statement {
            conn: self,
            stmt: stmt_ptr,
        })
    }

    fn query_map<T, F>(&self, sql: &str, mut mapper: F) -> Result<Vec<T>, String>
    where
        F: FnMut(&Row) -> Result<T, String>,
    {
        let mut stmt = self.prepare(sql)?;
        let mut results = Vec::new();
        while let Some(row) = stmt.next()? {
            results.push(mapper(&row)?);
        }
        Ok(results)
    }

    fn query_first<T, F>(&self, sql: &str, mut mapper: F) -> Result<Option<T>, String>
    where
        F: FnMut(&Row) -> Result<T, String>,
    {
        let mut stmt = self.prepare(sql)?;
        if let Some(row) = stmt.next()? {
            let value = mapper(&row)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    fn table_exists(&self, table_name: &str) -> Result<bool, String> {
        let sql = format!(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='{table_name}' LIMIT 1;"
        );
        let mut stmt = self.prepare(&sql)?;
        Ok(stmt.next()?.is_some())
    }

    fn last_error(&self, code: i32) -> String {
        let message = unsafe {
            let msg_ptr = ffi::sqlite3_errmsg(self.raw);
            if msg_ptr.is_null() {
                "unknown error".to_string()
            } else {
                CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
            }
        };
        format!("SQLite error {code}: {message}")
    }

    fn extract_errmsg(db_ptr: *mut ffi::sqlite3, code: i32) -> String {
        if db_ptr.is_null() {
            return format!("SQLite error {code}: unknown error");
        }
        let message = unsafe {
            let msg_ptr = ffi::sqlite3_errmsg(db_ptr);
            if msg_ptr.is_null() {
                "unknown error".to_string()
            } else {
                CStr::from_ptr(msg_ptr).to_string_lossy().into_owned()
            }
        };
        format!("SQLite error {code}: {message}")
    }
}

impl Drop for SqliteConnection {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                let _ = ffi::sqlite3_close(self.raw);
            }
        }
    }
}

struct Statement<'conn> {
    conn: &'conn SqliteConnection,
    stmt: *mut ffi::sqlite3_stmt,
}

impl Statement<'_> {
    fn next(&mut self) -> Result<Option<Row<'_>>, String> {
        let rc = unsafe { ffi::sqlite3_step(self.stmt) };
        match rc {
            ffi::SQLITE_ROW => Ok(Some(Row {
                stmt: self.stmt,
                _marker: PhantomData,
            })),
            ffi::SQLITE_DONE => Ok(None),
            code => Err(self.conn.last_error(code)),
        }
    }
}

impl Drop for Statement<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = ffi::sqlite3_finalize(self.stmt);
        }
    }
}

struct Row<'stmt> {
    stmt: *mut ffi::sqlite3_stmt,
    _marker: PhantomData<&'stmt ffi::sqlite3_stmt>,
}

impl Row<'_> {
    fn column_index(index: usize) -> Result<i32, String> {
        i32::try_from(index).map_err(|_| "Column index exceeds SQLite limits".to_string())
    }

    fn column_details(&self, index: usize) -> Result<(i32, i32), String> {
        let idx = Self::column_index(index)?;
        let column_type = unsafe { ffi::sqlite3_column_type(self.stmt, idx) };
        Ok((idx, column_type))
    }

    #[allow(clippy::cast_possible_truncation)]
    fn f64_to_f32(value: f64) -> Result<f32, String> {
        if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
            Err("SQLite double value out of range for f32".to_string())
        } else {
            Ok(value as f32)
        }
    }

    fn get_string(&self, index: usize) -> Result<String, String> {
        self.get_string_optional(index)
            .and_then(|opt| opt.ok_or_else(|| "Unexpected NULL text column".to_string()))
    }

    fn get_string_optional(&self, index: usize) -> Result<Option<String>, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Ok(None);
        }
        let text_ptr = unsafe { ffi::sqlite3_column_text(self.stmt, idx) };
        if text_ptr.is_null() {
            return Ok(Some(String::new()));
        }
        let byte_len = unsafe { ffi::sqlite3_column_bytes(self.stmt, idx) };
        let len = usize::try_from(byte_len)
            .map_err(|_| "Negative text length reported by SQLite".to_string())?;
        let slice = unsafe { slice::from_raw_parts(text_ptr.cast::<u8>(), len) };
        Ok(Some(String::from_utf8_lossy(slice).into_owned()))
    }

    fn get_f32(&self, index: usize) -> Result<f32, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Err("Unexpected NULL float column".to_string());
        }
        let value = unsafe { ffi::sqlite3_column_double(self.stmt, idx) };
        Self::f64_to_f32(value)
    }

    fn get_f32_optional(&self, index: usize) -> Result<Option<f32>, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Ok(None);
        }
        let value = unsafe { ffi::sqlite3_column_double(self.stmt, idx) };
        Self::f64_to_f32(value).map(Some)
    }

    fn get_u16_optional(&self, index: usize) -> Result<Option<u16>, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Ok(None);
        }
        let value = unsafe { ffi::sqlite3_column_int64(self.stmt, idx) };
        let converted = value
            .try_into()
            .map_err(|_| "Value out of range for u16".to_string())?;
        Ok(Some(converted))
    }

    fn get_i64(&self, index: usize) -> Result<i64, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Err("Unexpected NULL integer column".to_string());
        }
        Ok(unsafe { ffi::sqlite3_column_int64(self.stmt, idx) })
    }

    fn get_i64_optional(&self, index: usize) -> Result<Option<i64>, String> {
        let (idx, column_type) = self.column_details(index)?;
        if column_type == ffi::SQLITE_NULL {
            return Ok(None);
        }
        Ok(Some(unsafe { ffi::sqlite3_column_int64(self.stmt, idx) }))
    }
}

pub struct LocalProductDbConcrete {
    sqlite_con: SqliteConnection,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, EnumIter)]
enum SqlTablesNames {
    Products,
    MacroElements,
    MicroNutrients,
    AllowedUnits,
}

impl fmt::Display for SqlTablesNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let table_name = match self {
            SqlTablesNames::Products => "products",
            SqlTablesNames::MacroElements => "macro_elements",
            SqlTablesNames::MicroNutrients => "micronutrients",
            SqlTablesNames::AllowedUnits => "allowed_units",
        };
        write!(f, "{table_name}")
    }
}

// TODO panicking to be replaced with proper error handling
impl LocalProductDbConcrete {
    /// Creates a SQLite-backed product database.
    #[allow(clippy::unused_async)]
    pub async fn new(database_file: &str) -> Option<Self> {
        let con = SqliteConnection::open(database_file).ok()?;
        if con.enable_foreign_keys().is_err() {
            return None;
        }
        Self::init_db_if_new_created(&con);
        Some(LocalProductDbConcrete { sqlite_con: con })
    }

    fn init_db_if_new_created(sqlite_con: &SqliteConnection) {
        let products_table = SqlTablesNames::Products.to_string();
        let table_exists = sqlite_con
            .table_exists(&products_table)
            .unwrap_or_else(|_| panic!("Failed to check table existence for '{products_table}'"));
        if !table_exists {
            Self::create_tables(sqlite_con);
        }
    }

    // this function should be run only after upgrading version of this library (i.e., when new micro nutrient or unit is added)
    fn _update_table_columns(&self, table_name: SqlTablesNames) -> Result<(), String> {
        let (all_columns, col_type) = match table_name {
            t @ (SqlTablesNames::Products | SqlTablesNames::MacroElements) => {
                return Err(format!("{t} table should have all necessary columns"));
            }
            SqlTablesNames::MicroNutrients => (
                MicroNutrientsType::iter()
                    .map(|x| x.to_string())
                    .collect::<HashSet<String>>(),
                "FLOAT",
            ),
            SqlTablesNames::AllowedUnits => (
                AllowedUnitsType::iter()
                    .map(|x: AllowedUnitsType| x.to_string())
                    .collect::<HashSet<String>>(),
                "INTEGER",
            ),
        };

        let db_columns = self
            .sqlite_con
            .query_map(
                format!("SELECT name FROM pragma_table_info('{table_name}')").as_str(),
                |row| row.get_string(0),
            )
            .unwrap_or_else(|_| panic!("Getting column names of the {table_name} table failed"));

        let mut missing_columns = Vec::new();
        for col_name in db_columns {
            if !all_columns.contains(&col_name) {
                missing_columns.push(col_name);
            }
        }

        if missing_columns.is_empty() {
            return Ok(());
        }

        for col in missing_columns {
            let alter_table_query =
                format!("ALTER TABLE {table_name} ADD COLUMN \"{col}\" {col_type};");
            self.sqlite_con
                .execute(alter_table_query.as_str())
                .map_err(|e| {
                    format!("Failed to add column '{col}' to table '{table_name}': {e}")
                })?;
        }

        Ok(())
    }

    fn create_tables(sqlite_con: &SqliteConnection) {
        sqlite_con
            .execute(
                format!(
                    "CREATE TABLE {} (
                    id TEXT NOT NULL PRIMARY KEY,
                    name CHAR NOT NULL,
                    brand CHAR
                )",
                    SqlTablesNames::Products
                )
                .as_str(),
            )
            .unwrap_or_else(|_| panic!("Failed to create '{}' table", SqlTablesNames::Products));

        let macro_elem_to_text = |x: MacroElementsType| {
            if MacroElementsType::Calories == x {
                String::new()
            } else {
                format!("\"{x}\" FLOAT NOT NULL,\n")
            }
        };
        let macro_elements_fields: String =
            MacroElementsType::iter().map(macro_elem_to_text).collect();
        Self::create_table_for_table_name(
            sqlite_con,
            SqlTablesNames::MacroElements.to_string().as_str(),
            macro_elements_fields.as_str(),
        )
        .unwrap_or_else(|_| panic!("Failed to create '{}' table", SqlTablesNames::MacroElements));

        let micronutrients_to_text = |x: MicroNutrientsType| format!("\"{x}\" FLOAT,\n");
        let micronutrients_fields: String = MicroNutrientsType::iter()
            .map(micronutrients_to_text)
            .collect();
        Self::create_table_for_table_name(
            sqlite_con,
            SqlTablesNames::MicroNutrients.to_string().as_str(),
            micronutrients_fields.as_str(),
        )
        .unwrap_or_else(|_| {
            panic!(
                "Failed to create '{}' table",
                SqlTablesNames::MicroNutrients
            )
        });

        let allowed_units_to_text = |x: AllowedUnitsType| match x {
            AllowedUnitsType::Gram => format!(
                "\"{x}\" INTEGER NOT NULL DEFAULT 1,\n\"{x} divider\" INTEGER NOT NULL DEFAULT 1,\n"
            ),
            _ => format!("\"{x}\" INTEGER,\n\"{x} divider\" INTEGER,\n"),
        };
        let allowed_units_fields: String = AllowedUnitsType::iter()
            .map(allowed_units_to_text)
            .collect();
        Self::create_table_for_table_name(
            sqlite_con,
            SqlTablesNames::AllowedUnits.to_string().as_str(),
            allowed_units_fields.as_str(),
        )
        .unwrap_or_else(|_| panic!("Failed to create '{}' table", SqlTablesNames::AllowedUnits));
    }

    fn create_table_for_table_name(
        sqlite_con: &SqliteConnection,
        table_name: &str,
        data: &str,
    ) -> Result<(), String> {
        sqlite_con
            .execute(&format!(
                "CREATE TABLE {} (
                        id TEXT NOT NULL PRIMARY KEY,
                        {}
                        FOREIGN KEY(id) REFERENCES {}(id) ON DELETE CASCADE
                    )",
                table_name,
                data,
                SqlTablesNames::Products
            ))
            .map_err(|e| format!("Failed to create '{table_name}' table: {e}"))?;
        Ok(())
    }
}

fn db_search_criteria_to_sql_query_fragment(
    criteria: &[DbSearchCriteria],
) -> Result<String, String> {
    if criteria.is_empty() {
        return Err("Empty search criteria".to_string());
    }
    let mut query_fragment = " WHERE ".to_string();
    for (i, criterion) in criteria.iter().enumerate() {
        if i > 0 {
            query_fragment.push_str(" AND ");
        }
        match criterion {
            DbSearchCriteria::ById(name) => {
                write!(
                    query_fragment,
                    "{}.name LIKE '{}%'",
                    SqlTablesNames::Products,
                    name
                )
                .unwrap();
            }
        }
    }
    Ok(query_fragment)
}

fn map_query_row_to_product(row: &Row) -> Result<(String, Product), String> {
    let id = row.get_string(0)?;
    let name = row.get_string(1)?;
    let brand = row.get_string_optional(2)?;

    let mut offset = 3;

    let mut macro_values = Vec::with_capacity(MacroElementsType::COUNT - 1);
    for macro_type in MacroElementsType::iter() {
        if macro_type == MacroElementsType::Calories {
            continue;
        }
        macro_values.push(row.get_f32(offset)?);
        offset += 1;
    }
    if macro_values.len() != (MacroElementsType::COUNT - 1) {
        return Err("Unexpected number of macro nutrient columns".to_string());
    }
    let macro_elems = MacroElements::new(
        macro_values[0],
        macro_values[1],
        macro_values[2],
        macro_values[3],
        macro_values[4],
    );

    let mut micronutrients = Box::new(MicroNutrients::default());
    for micro_type in MicroNutrientsType::iter() {
        micronutrients[micro_type] = row.get_f32_optional(offset)?;
        offset += 1;
    }

    let mut allowed_units: AllowedUnits = HashMap::new();
    for unit in AllowedUnitsType::iter() {
        let quantity = row.get_u16_optional(offset)?;
        let divider = row.get_u16_optional(offset + 1)?;
        offset += 2;
        if let (Some(amount), Some(divider)) = (quantity, divider) {
            allowed_units.insert(unit, UnitData { amount, divider });
        }
    }

    let product = Product::new(
        name,
        brand,
        Box::new(macro_elems),
        micronutrients,
        allowed_units,
    );
    Ok((id, product))
}

#[async_trait::async_trait(?Send)]
impl Database for LocalProductDbConcrete {
    async fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, Product> {
        let mut query_template = format!(
            "SELECT {p}.id, {p}.name, {p}.brand",
            p = SqlTablesNames::Products
        );
        // Helper closure to append columns from an enum iterator
        let mut append_columns =
            |table: SqlTablesNames, iter: &mut dyn Iterator<Item = Option<String>>| {
                for col in iter.flatten() {
                    write!(query_template, ", {table}.\"{col}\"").unwrap();
                }
            };

        append_columns(
            SqlTablesNames::MacroElements,
            &mut MacroElementsType::iter().map(|m| {
                if m == MacroElementsType::Calories {
                    None
                } else {
                    Some(m.to_string())
                }
            }),
        );
        append_columns(
            SqlTablesNames::MicroNutrients,
            &mut MicroNutrientsType::iter().map(|m| Some(m.to_string())),
        );
        let mut allowed_columns_iter = AllowedUnitsType::iter()
            .flat_map(|unit| [Some(unit.to_string()), Some(format!("{unit} divider"))].into_iter());
        append_columns(SqlTablesNames::AllowedUnits, &mut allowed_columns_iter);

        write!(
            query_template,
            " FROM {p}
            INNER JOIN {me} ON {p}.id = {me}.id
            INNER JOIN {au} ON {p}.id = {au}.id
            LEFT JOIN {mn} ON {p}.id = {mn}.id",
            p = SqlTablesNames::Products,
            me = SqlTablesNames::MacroElements,
            au = SqlTablesNames::AllowedUnits,
            mn = SqlTablesNames::MicroNutrients
        )
        .unwrap();
        query_template.push_str(
            db_search_criteria_to_sql_query_fragment(criteria)
                .expect("Failed to convert search criteria to SQL query fragment")
                .as_str(),
        );
        query_template.push(';');

        let products = self
            .sqlite_con
            .query_map(&query_template, map_query_row_to_product)
            .unwrap_or_else(|e| panic!("Failed to map query results: {e}"));

        let mut result_map = HashMap::new();
        result_map.extend(products);
        result_map
    }

    async fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: AllowedUnitsType,
        unit_data: UnitData,
    ) -> Result<(), String> {
        let update_query = format!(
            "UPDATE {} SET \"{}\" = {}, \"{}\" = {} WHERE id = '{}';",
            SqlTablesNames::AllowedUnits,
            allowed_unit,
            unit_data.amount,
            allowed_unit.to_string() + " divider",
            unit_data.divider,
            product_id
        );
        self.sqlite_con
            .execute(update_query.as_str())
            .map_err(|e| {
                format!(
                    "Failed to update allowed unit {allowed_unit} for product {product_id}: {e}"
                )
            })?;
        Ok(())
    }
}

// function is long because there are 2 macro definitions inside
#[allow(clippy::too_many_lines)]
#[async_trait::async_trait(?Send)]
impl MutableDatabase for LocalProductDbConcrete {
    async fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let run_query = |table_name: &str,
                         columns_str: &str,
                         values_str: &str|
         -> Result<(), String> {
            self.sqlite_con
                .execute(&format!(
                    "INSERT INTO {table_name} ({columns_str}) VALUES ({values_str});"
                ))
                .map_err(|e| {
                    format!("Failed to insert product '{product_id}' into {table_name} table: {e}")
                })?;
            Ok(())
        };

        run_query(
            &SqlTablesNames::Products.to_string(),
            "id, name, brand",
            format!(
                "'{}', '{}', {}",
                product_id,
                product.name(),
                match product.brand() {
                    Some(brand) => format!("'{brand}'"),
                    None => "NULL".to_string(),
                }
            )
            .as_str(),
        )?;

        macro_rules! map_function_for_add {
            ($it:expr, MacroElements, columns) => {
                $it.filter_map(|x| match x {
                    MacroElementsType::Calories => None,
                    _ => Some(format!("\"{x}\", ")),
                })
            };
            ($it:expr, MicroNutrients, columns) => {
                $it.map(|x| format!("\"{x}\", "))
            };
            ($it:expr, AllowedUnits, columns) => {
                $it.map(|x| format!("\"{x}\", \"{} divider\", ", x))
            };
            ($it:expr, MacroElements, values) => {
                $it.filter_map(|x| match x {
                    MacroElementsType::Calories => None,
                    _ => Some(format!("{}, ", product.macro_elements[x])),
                })
            };
            ($it:expr, MicroNutrients, values) => {
                $it.map(|x| {
                    let val = product.micro_nutrients[x];
                    if val.is_none() {
                        "NULL, ".to_string()
                    } else {
                        format!("{}, ", val.unwrap())
                    }
                })
            };
            ($it:expr, AllowedUnits, values) => {
                $it.map(|x| {
                    let val = product.allowed_units.get(&x);
                    if val.is_none() {
                        "NULL, NULL, ".to_string()
                    } else {
                        format!("{}, {}, ", val.unwrap().amount, val.unwrap().divider)
                    }
                })
            };
        }

        macro_rules! add_product_macro_micro_units {
            ($object:tt, $enum_type:ty, $sql_table_var:expr) => {
                let col_names_iter = <$enum_type>::iter();

                let col_names = map_function_for_add!(col_names_iter, $object, columns)
                    .collect::<String>()
                    .strip_suffix(", ")
                    .unwrap()
                    .to_string();

                let values_iter = <$enum_type>::iter();

                let values = map_function_for_add!(values_iter, $object, values)
                    .collect::<String>()
                    .strip_suffix(", ")
                    .unwrap()
                    .to_string();

                run_query(
                    $sql_table_var.to_string().as_str(),
                    format!("id, {col_names}").as_str(),
                    format!("'{}', {}", product.id(), values).as_str(),
                )?;
            };
        }

        add_product_macro_micro_units!(
            MacroElements,
            MacroElementsType,
            SqlTablesNames::MacroElements
        );
        add_product_macro_micro_units!(
            MicroNutrients,
            MicroNutrientsType,
            SqlTablesNames::MicroNutrients
        );
        add_product_macro_micro_units!(
            AllowedUnits,
            AllowedUnitsType,
            SqlTablesNames::AllowedUnits
        );

        Ok(())
    }

    async fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let run_exec = |sql: String| {
            self.sqlite_con
                .execute(&sql)
                .map_err(|e| format!("Failed to upsert product '{product_id}': {e}"))
        };

        let brand_sql = match product.brand() {
            Some(brand) => format!("'{brand}'"),
            None => "NULL".to_string(),
        };
        run_exec(format!(
            "INSERT INTO {table} (id, name, brand) VALUES ('{id}', '{name}', {brand}) \
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, brand = excluded.brand;",
            table = SqlTablesNames::Products,
            id = product_id,
            name = product.name(),
            brand = brand_sql,
        ))?;

        let macro_cols: Vec<String> = MacroElementsType::iter()
            .filter(|m| *m != MacroElementsType::Calories)
            .map(|m| format!("\"{m}\""))
            .collect();
        let macro_values: Vec<String> = MacroElementsType::iter()
            .filter(|m| *m != MacroElementsType::Calories)
            .map(|m| product.macro_elements[m].to_string())
            .collect();
        let macro_updates = macro_cols
            .iter()
            .map(|c| format!("{c} = excluded.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        run_exec(format!(
            "INSERT INTO {table} (id, {cols}) VALUES ('{id}', {vals}) \
             ON CONFLICT(id) DO UPDATE SET {updates};",
            table = SqlTablesNames::MacroElements,
            cols = macro_cols.join(", "),
            vals = macro_values.join(", "),
            updates = macro_updates,
            id = product_id,
        ))?;

        let micro_cols: Vec<String> = MicroNutrientsType::iter()
            .map(|m| format!("\"{m}\""))
            .collect();
        let micro_values: Vec<String> = MicroNutrientsType::iter()
            .map(|m| match product.micro_nutrients[m] {
                Some(v) => v.to_string(),
                None => "NULL".to_string(),
            })
            .collect();
        let micro_updates = micro_cols
            .iter()
            .map(|c| format!("{c} = excluded.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        run_exec(format!(
            "INSERT INTO {table} (id, {cols}) VALUES ('{id}', {vals}) \
             ON CONFLICT(id) DO UPDATE SET {updates};",
            table = SqlTablesNames::MicroNutrients,
            cols = micro_cols.join(", "),
            vals = micro_values.join(", "),
            updates = micro_updates,
            id = product_id,
        ))?;

        let allowed_cols: Vec<String> = AllowedUnitsType::iter()
            .flat_map(|u| {
                let base = u.to_string();
                vec![format!("\"{base}\""), format!("\"{base} divider\"")]
            })
            .collect();
        let mut allowed_values = Vec::with_capacity(allowed_cols.len());
        for unit in AllowedUnitsType::iter() {
            let entry = product.allowed_units.get(&unit);
            allowed_values.push(
                entry
                    .map(|u| u.amount.to_string())
                    .unwrap_or_else(|| "NULL".to_string()),
            );
            allowed_values.push(
                entry
                    .map(|u| u.divider.to_string())
                    .unwrap_or_else(|| "NULL".to_string()),
            );
        }
        let allowed_updates = allowed_cols
            .iter()
            .map(|c| format!("{c} = excluded.{c}"))
            .collect::<Vec<_>>()
            .join(", ");
        run_exec(format!(
            "INSERT INTO {table} (id, {cols}) VALUES ('{id}', {vals}) \
             ON CONFLICT(id) DO UPDATE SET {updates};",
            table = SqlTablesNames::AllowedUnits,
            cols = allowed_cols.join(", "),
            vals = allowed_values.join(", "),
            updates = allowed_updates,
            id = product_id,
        ))?;

        Ok(())
    }

    async fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
        let main_table_name = SqlTablesNames::Products.to_string();
        self.sqlite_con
            .execute(
                format!(
                    "DELETE FROM {main_table_name} WHERE id = '{product_id}';"
                )
                .as_str(),
            )
            .map_err(|e| {
                format!(
                    "Failed to delete product with ID '{product_id}' from table '{main_table_name}': {e}"
                )
            })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::{
        AllowedUnits, AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients,
        MicroNutrientsType, UnitData,
    };
    use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};
    use approx::assert_relative_eq;
    use futures::executor::block_on;
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Once;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn assert_table_columns(
        connection: &SqliteConnection,
        table: &str,
        expected_columns: &[String],
    ) {
        let count = connection
            .query_first(
                format!(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{table}';"
                )
                .as_str(),
                |row| row.get_i64(0),
            )
            .expect("Failed to check table existence")
            .unwrap_or(0);
        assert!(count > 0, "Expected '{table}' table to exist");

        let columns = connection
            .query_map(
                format!("SELECT name FROM pragma_table_info('{table}');").as_str(),
                |row| row.get_string(0),
            )
            .expect("Failed to read table columns");

        assert_eq!(
            columns, expected_columns,
            "Unexpected columns for table '{table}'"
        );
    }

    #[test]
    fn test_00_local_product_db_new_creates_schema() {
        let db_path = unique_test_db_path();
        let _cleanup_guard = FileCleanup::new(db_path.clone());
        cleanup_existing_files(&db_path).expect("Failed to clean up test database files");
        assert!(
            !db_path.exists(),
            "Expected database file to be absent before initialization"
        );

        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create database directory");
        }

        let path_str = db_path
            .to_str()
            .expect("Database path contains invalid UTF-8");

        {
            let _db = block_on(LocalProductDbConcrete::new(path_str))
                .expect("Expected LocalProductDbConcrete::new to succeed for fresh database");
        }

        assert!(db_path.exists(), "Expected SQLite file to be created");

        let connection = SqliteConnection::open(&db_path.to_string_lossy())
            .expect("Failed to open created database");

        let mut macro_columns = vec!["id".to_string()];
        macro_columns.extend(
            MacroElementsType::iter()
                .filter(|m| *m != MacroElementsType::Calories)
                .map(|m| m.to_string()),
        );

        let mut nutrient_columns = vec!["id".to_string()];
        nutrient_columns.extend(MicroNutrientsType::iter().map(|m| m.to_string()));

        let mut allowed_columns = vec!["id".to_string()];
        for unit in AllowedUnitsType::iter() {
            allowed_columns.push(unit.to_string());
            allowed_columns.push(format!("{unit} divider"));
        }

        let product_columns = vec!["id".to_string(), "name".to_string(), "brand".to_string()];
        assert_table_columns(&connection, "products", &product_columns);
        assert_table_columns(&connection, "macro_elements", &macro_columns);
        assert_table_columns(&connection, "micronutrients", &nutrient_columns);
        assert_table_columns(&connection, "allowed_units", &allowed_columns);

        drop(connection);
    }

    #[test]
    fn test_01_local_product_db_new_preserves_existing_data() {
        let test_db = TestDbGuard::create_empty().expect("Failed to create empty database");

        {
            let mut db = test_db.local_db();
            let mut allowed_units: AllowedUnits = HashMap::new();
            allowed_units.insert(
                AllowedUnitsType::Gram,
                UnitData {
                    amount: 1,
                    divider: 1,
                },
            );
            let product = Product::new(
                "Persisted".to_string(),
                Some("BrandP".to_string()),
                Box::new(MacroElements::new(
                    1.0_f32, 0.5_f32, 2.0_f32, 1.0_f32, 3.0_f32,
                )),
                Box::default(),
                allowed_units,
            );

            let product_id = product.id();
            block_on(db.add_product(product_id.as_str(), product))
                .expect("Expected add_product to succeed for persisted product");
        }

        let db = test_db.local_db();
        let results = block_on(
            db.get_products_matching_criteria(&[DbSearchCriteria::ById("Persisted".to_string())]),
        );
        let key = "Persisted (BrandP)";
        assert!(
            results.contains_key(key),
            "Expected previously inserted product '{key}' to remain after reopening"
        );
    }

    struct TestDbGuard {
        path: PathBuf,
        _cleanup_guard: FileCleanup,
    }

    impl TestDbGuard {
        fn create_empty() -> Result<Self, String> {
            let path = unique_test_db_path();
            cleanup_existing_files(&path)?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let cleanup_guard = FileCleanup::new(path.clone());
            let path_str = path
                .to_str()
                .ok_or_else(|| "Database path contains invalid UTF-8".to_string())?;
            let db = block_on(LocalProductDbConcrete::new(path_str))
                .ok_or_else(|| "LocalProductDbConcrete::new returned None".to_string())?;
            drop(db);
            Ok(Self {
                path,
                _cleanup_guard: cleanup_guard,
            })
        }

        fn create_seeded() -> Result<Self, String> {
            let guard = Self::create_empty()?;
            {
                let mut db = guard.local_db();
                seed_products(&mut db)?;
            }
            Ok(guard)
        }

        fn connection(&self) -> SqliteConnection {
            SqliteConnection::open(
                self.path
                    .to_str()
                    .expect("Database path contains invalid UTF-8"),
            )
            .expect("Failed to reopen test database")
        }

        fn local_db(&self) -> LocalProductDbConcrete {
            block_on(LocalProductDbConcrete::new(
                self.path
                    .to_str()
                    .expect("Database path contains invalid UTF-8"),
            ))
            .expect("Failed to reopen seeded LocalProductDbConcrete")
        }
    }

    impl Drop for TestDbGuard {
        fn drop(&mut self) {
            let _ = cleanup_existing_files(&self.path);
        }
    }

    fn cleanup_existing_files(path: &Path) -> Result<(), String> {
        use std::io::ErrorKind;

        let attempt_remove = |candidate: &Path| -> Result<(), String> {
            match fs::remove_file(candidate) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
                Err(err) => Err(err.to_string()),
            }
        };

        attempt_remove(path)?;
        for suffix in ["-journal", "-wal", "-shm"] {
            let mut os_string = path.as_os_str().to_os_string();
            os_string.push(suffix);
            let candidate: PathBuf = os_string.into();
            attempt_remove(&candidate)?;
        }
        Ok(())
    }

    fn unique_test_db_path() -> PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static INIT_CLEANUP: Once = Once::new();
        INIT_CLEANUP.call_once(|| {
            if let Err(err) = cleanup_previous_test_databases() {
                panic!("Failed to cleanup previous test databases: {err}");
            }
        });

        let suffix = COUNTER.fetch_add(1, Ordering::Relaxed);

        let base_path = Path::new(DATABASE_FILENAME);
        let parent = base_path.parent();
        let stem = base_path.file_stem().map_or_else(
            || "test_db".to_string(),
            |s| s.to_string_lossy().into_owned(),
        );
        let filename = if let Some(ext) = base_path.extension() {
            format!("{}_{}.{}", stem, suffix, ext.to_string_lossy())
        } else {
            format!("{stem}_{suffix}")
        };

        match parent {
            Some(dir) => dir.join(&filename),
            None => PathBuf::from(&filename),
        }
    }

    fn cleanup_previous_test_databases() -> Result<(), String> {
        let base_path = Path::new(DATABASE_FILENAME);
        let parent = base_path
            .parent()
            .ok_or_else(|| "DATABASE_FILENAME has no parent directory".to_string())?;
        let stem = base_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| "DATABASE_FILENAME stem is not valid UTF-8".to_string())?;
        let prefix = format!("{stem}_");
        let extension = base_path
            .extension()
            .and_then(|e| e.to_str())
            .map(std::string::ToString::to_string);

        let entries = fs::read_dir(parent).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if !file_stem.starts_with(&prefix) {
                continue;
            }
            if let Some(ref expected_ext) = extension {
                let actual_ext = path.extension().and_then(|e| e.to_str());
                if actual_ext != Some(expected_ext.as_str()) {
                    continue;
                }
            }
            cleanup_existing_files(&path)?;
        }
        Ok(())
    }

    fn seed_products(db: &mut LocalProductDbConcrete) -> Result<(), String> {
        let mut apple_allowed: AllowedUnits = HashMap::new();
        apple_allowed.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        apple_allowed.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 1,
                divider: 2,
            },
        );
        let mut apple_micro = Box::new(MicroNutrients::default());
        apple_micro[MicroNutrientsType::Fiber] = Some(2.4_f32);
        apple_micro[MicroNutrientsType::Zinc] = Some(0.1_f32);
        apple_micro[MicroNutrientsType::Sodium] = Some(1.0_f32);
        let apple = Product::new(
            "Apple".to_string(),
            Some("BrandA".to_string()),
            Box::new(MacroElements::new(
                0.2_f32, 0.1_f32, 14.0_f32, 10.0_f32, 0.3_f32,
            )),
            apple_micro,
            apple_allowed,
        );
        let apple_id = apple.id();
        block_on(db.add_product(apple_id.as_str(), apple))
            .map_err(|e| format!("Failed to seed product {apple_id}: {e}"))?;

        let mut banana_allowed: AllowedUnits = HashMap::new();
        banana_allowed.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        banana_allowed.insert(
            AllowedUnitsType::Tablespoon,
            UnitData {
                amount: 2,
                divider: 1,
            },
        );
        banana_allowed.insert(
            AllowedUnitsType::Custom,
            UnitData {
                amount: 50,
                divider: 1,
            },
        );
        let mut banana_micro = Box::new(MicroNutrients::default());
        banana_micro[MicroNutrientsType::Fiber] = Some(2.6_f32);
        banana_micro[MicroNutrientsType::Sodium] = Some(1.0_f32);
        let banana = Product::new(
            "Banana".to_string(),
            None,
            Box::new(MacroElements::new(
                0.3_f32, 0.1_f32, 23.0_f32, 12.0_f32, 1.1_f32,
            )),
            banana_micro,
            banana_allowed,
        );
        let banana_id = banana.id();
        block_on(db.add_product(banana_id.as_str(), banana))
            .map_err(|e| format!("Failed to seed product {banana_id}: {e}"))?;

        Ok(())
    }

    struct FileCleanup {
        path: PathBuf,
    }

    impl FileCleanup {
        fn new(path: PathBuf) -> Self {
            Self { path }
        }
    }

    impl Drop for FileCleanup {
        fn drop(&mut self) {
            let _ = cleanup_existing_files(&self.path);
        }
    }

    #[test]
    fn test_02_initialize_and_seed_database() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let conn = test_db.connection();
        let product_count = conn
            .query_first("SELECT COUNT(*) FROM products;", |row| row.get_i64(0))
            .expect("Failed to count products")
            .expect("Missing count row");
        assert_eq!(product_count, 2);
        let macro_columns = conn
            .query_first(
                "SELECT COUNT(*) FROM pragma_table_info('macro_elements');",
                |row| row.get_i64(0),
            )
            .expect("Failed to count macro columns")
            .expect("Missing macro columns row");
        assert_eq!(macro_columns, 6);
    }

    #[test]
    fn test_03_get_products_matching_criteria_returns_expected_product() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let db = test_db.local_db();
        let results = block_on(
            db.get_products_matching_criteria(&[DbSearchCriteria::ById("Apple".to_string())]),
        );
        assert_eq!(results.len(), 1);
        let apple = results
            .get("Apple (BrandA)")
            .expect("Missing Apple product");
        assert_eq!(apple.name(), "Apple");
        assert_eq!(apple.brand(), Some("BrandA"));
        assert_relative_eq!(apple.macro_elements[MacroElementsType::Fat], 0.2_f32);
        assert_eq!(
            apple.micro_nutrients[MicroNutrientsType::Fiber],
            Some(2.4_f32)
        );
        assert_eq!(apple.allowed_units[&AllowedUnitsType::Cup].amount, 1);
        assert_eq!(apple.allowed_units[&AllowedUnitsType::Cup].divider, 2);
    }

    #[test]
    fn test_04_set_product_unit_updates_allowed_units_table() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let result = block_on(db.set_product_unit(
            "Apple (BrandA)",
            AllowedUnitsType::Cup,
            UnitData {
                amount: 3,
                divider: 2,
            },
        ));
        assert!(result.is_ok());
        let conn = test_db.connection();
        let updated = conn
            .query_first(
                "SELECT cup FROM allowed_units WHERE id = 'Apple (BrandA)';",
                |row| row.get_u16_optional(0),
            )
            .expect("Failed to fetch updated unit")
            .expect("Missing cup value");
        assert_eq!(updated, Some(3));
        let updated_div = conn
            .query_first(
                "SELECT \"cup divider\" FROM allowed_units WHERE id = 'Apple (BrandA)';",
                |row| row.get_u16_optional(0),
            )
            .expect("Failed to fetch updated divider")
            .expect("Missing cup divider");
        assert_eq!(updated_div, Some(2));
    }

    #[test]
    fn test_05_add_product_inserts_all_related_rows() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let mut allowed_units: AllowedUnits = HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 2,
                divider: 1,
            },
        );
        let new_product = Product::new(
            "Orange".to_string(),
            Some("CitrusCo".to_string()),
            Box::new(MacroElements::new(
                0.1_f32, 0.05_f32, 11.0_f32, 9.0_f32, 1.0_f32,
            )),
            Box::default(),
            allowed_units,
        );
        let new_id = new_product.id();
        assert!(
            block_on(db.add_product(new_id.as_str(), new_product)).is_ok(),
            "Expected add_product to succeed"
        );
        let conn = test_db.connection();
        let product_count = conn
            .query_first(
                format!("SELECT COUNT(*) FROM products WHERE id = '{new_id}';").as_str(),
                |row| row.get_i64(0),
            )
            .expect("Failed to verify inserted product")
            .expect("Missing product count");
        assert_eq!(product_count, 1);
    }

    #[test]
    fn test_06_update_product_modifies_macro_and_micro_values() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let mut allowed_units: AllowedUnits = HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        allowed_units.insert(
            AllowedUnitsType::Custom,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        let mut micro = Box::new(MicroNutrients::default());
        micro[MicroNutrientsType::Fiber] = Some(3.0_f32);
        micro[MicroNutrientsType::Zinc] = Some(0.2_f32);
        let updated = Product::new(
            "Apple".to_string(),
            Some("BrandA".to_string()),
            Box::new(MacroElements::new(
                0.5_f32, 0.2_f32, 15.0_f32, 11.0_f32, 0.9_f32,
            )),
            micro,
            allowed_units,
        );
        assert!(
            block_on(db.update_product("Apple (BrandA)", updated)).is_ok(),
            "Expected update_product to succeed"
        );
        let conn = test_db.connection();
        let fiber = conn
            .query_first(
                "SELECT Fiber FROM micronutrients WHERE id = 'Apple (BrandA)';",
                |row| row.get_f32_optional(0),
            )
            .expect("Failed to fetch updated fiber")
            .expect("Missing fiber value");
        assert_eq!(fiber, Some(3.0_f32));
    }

    #[test]
    fn test_07_delete_product_removes_all_rows() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        assert!(
            block_on(db.delete_product("Banana")).is_ok(),
            "Expected delete_product to succeed"
        );
        let conn = test_db.connection();
        let count = conn
            .query_first(
                "SELECT COUNT(*) FROM products WHERE id = 'Banana';",
                |row| row.get_i64(0),
            )
            .expect("Failed to check product deletion")
            .expect("Missing count value");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_08_new_returns_connection() {
        let db_path = unique_test_db_path();
        let _cleanup_guard = FileCleanup::new(db_path.clone());
        cleanup_existing_files(&db_path).expect("Failed to remove pre-existing database file");
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).expect("Failed to create database directory");
        }

        let path_str = db_path
            .to_str()
            .expect("Database path contains invalid UTF-8");
        let result = block_on(LocalProductDbConcrete::new(path_str));
        assert!(
            result.is_some(),
            "Expected LocalProductDbConcrete::new to return Some"
        );
        drop(result);
    }

    #[test]
    fn test_09_update_product_inserts_when_missing() {
        let test_db = TestDbGuard::create_empty().expect("Failed to prepare empty database");
        let mut db = test_db.local_db();

        let mut allowed_units: AllowedUnits = HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        allowed_units.insert(
            AllowedUnitsType::Custom,
            UnitData {
                amount: 250,
                divider: 2,
            },
        );

        let mut micro = Box::new(MicroNutrients::default());
        micro[MicroNutrientsType::Fiber] = Some(1.5_f32);

        let product = Product::new(
            "Kiwi".to_string(),
            Some("FreshCo".to_string()),
            Box::new(MacroElements::new(
                0.4_f32, 0.1_f32, 15.0_f32, 9.0_f32, 1.1_f32,
            )),
            micro,
            allowed_units,
        );

        let product_id = product.id();
        assert!(
            block_on(db.update_product(product_id.as_str(), product)).is_ok(),
            "Expected update_product to insert when missing"
        );

        let conn = test_db.connection();
        let name = conn
            .query_first(
                format!("SELECT name FROM products WHERE id = '{product_id}';").as_str(),
                |row| row.get_string(0),
            )
            .expect("Failed to read inserted product")
            .expect("Missing inserted product");
        assert_eq!(name, "Kiwi");

        let fiber = conn
            .query_first(
                format!("SELECT Fiber FROM micronutrients WHERE id = '{product_id}';").as_str(),
                |row| row.get_f32_optional(0),
            )
            .expect("Failed to fetch inserted micronutrient")
            .expect("Missing inserted micronutrient value");
        assert_eq!(fiber, Some(1.5_f32));

        let custom_amount = conn
            .query_first(
                format!("SELECT custom FROM allowed_units WHERE id = '{product_id}';").as_str(),
                |row| row.get_u16_optional(0),
            )
            .expect("Failed to fetch inserted allowed unit")
            .expect("Missing custom unit amount");
        assert_eq!(custom_amount, Some(250));

        let gram_divider = conn
            .query_first(
                format!("SELECT \"gram divider\" FROM allowed_units WHERE id = '{product_id}';")
                    .as_str(),
                |row| row.get_u16_optional(0),
            )
            .expect("Failed to fetch gram divider")
            .expect("Missing gram divider");
        assert_eq!(gram_divider, Some(1));
    }
}
