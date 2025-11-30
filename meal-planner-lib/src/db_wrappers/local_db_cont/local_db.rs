use core::fmt;
use core::fmt::Write;
use std::collections::{HashMap, HashSet};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::EnumIter;

use crate::data_types::{
    AllowedUnits, AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients,
    MicroNutrientsType, Product,
};
use crate::db_wrappers::{DbSearchCriteria, DbWrapper, MutableDbWrapper};
use const_format::concatcp;

// cant make paths relative to this file
const DATABASE_PATH: &str = "src/db_wrappers/local_db_cont/";
#[cfg(not(test))]
pub(crate) const DATABASE_FILENAME: &str = concatcp!(DATABASE_PATH, "local_db.sqlite");
#[cfg(test)]
pub(crate) const DATABASE_FILENAME: &str = concatcp!(DATABASE_PATH, "test_local_db.sqlite");

pub(crate) struct LocalProductDb {
    sqlite_con: rusqlite::Connection,
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
impl LocalProductDb {
    pub fn new(database_file: &str) -> Option<Self> {
        let con = rusqlite::Connection::open(database_file).ok()?;
        if con.execute("PRAGMA foreign_keys = ON;", []).is_err() {
            return None;
        }
        Self::init_db_if_new_created(&con);
        Some(LocalProductDb { sqlite_con: con })
    }

    fn init_db_if_new_created(sqlite_con: &rusqlite::Connection) {
        let mut stmt = sqlite_con
            .prepare(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}';",
                SqlTablesNames::Products
            ))
            .expect("Failed to prepare statement");
        let mut rows = stmt.query([]).expect("Failed to execute query");
        if rows.next().expect("Failed to fetch row").is_none() {
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

        let mut stmt = self
            .sqlite_con
            .prepare(format!("SELECT name FROM pragma_table_info('{table_name}')").as_str())
            .unwrap_or_else(|_| panic!("Getting columns names of the {table_name} table failed"));

        let db_column_iter = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("Failed to map column names");

        let mut missing_columns = Vec::new();
        for col_result in db_column_iter {
            let col_name = col_result.expect("Failed to get column name");
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
                .execute(alter_table_query.as_str(), [])
                .map_err(|e| {
                    format!("Failed to add column '{col}' to table '{table_name}': {e}")
                })?;
        }

        Ok(())
    }

    fn create_tables(sqlite_con: &rusqlite::Connection) {
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
                [],
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
            AllowedUnitsType::Piece => format!("\"{x}\" INTEGER NOT NULL DEFAULT 1,\n"),
            _ => format!("\"{x}\" INTEGER,\n"),
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
        sqlite_con: &rusqlite::Connection,
        table_name: &str,
        data: &str,
    ) -> Result<(), String> {
        sqlite_con
            .execute(
                &format!(
                    "CREATE TABLE {} (
                        id TEXT NOT NULL PRIMARY KEY,
                        {}
                        FOREIGN KEY(id) REFERENCES {}(id) ON DELETE CASCADE
                    )",
                    table_name,
                    data,
                    SqlTablesNames::Products
                ),
                [],
            )
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

fn map_query_row_to_product(row: &rusqlite::Row) -> (String, Product) {
    enum SelectColumnIndexToProductData {
        Id = 0,
        Name = 1,
        Brand = 2,
        MacroElementsStart = 3,
        // -1 because Calories are not to be inside DB
        MicroNutrientsStart = 3 + MacroElementsType::COUNT as isize - 1,
        // -1 because Calories are not to be inside DB
        AllowedUnitsStart =
            3 + MacroElementsType::COUNT as isize + MicroNutrientsType::COUNT as isize - 1,
    }

    let name = row.get_unwrap(SelectColumnIndexToProductData::Name as usize);
    let brand = row.get_unwrap(SelectColumnIndexToProductData::Brand as usize);

    let mut iter_macroelems = MacroElementsType::iter().map(|m| {
        row.get_unwrap((SelectColumnIndexToProductData::MacroElementsStart as usize) + (m as usize))
    });
    // the last item is Calories, which is computed, so we skip it
    let macro_elems = MacroElements::new(
        iter_macroelems.next().unwrap(),
        iter_macroelems.next().unwrap(),
        iter_macroelems.next().unwrap(),
        iter_macroelems.next().unwrap(),
        iter_macroelems.next().unwrap(),
    );

    let iter_micronutrients = MicroNutrientsType::iter().map(|m| {
        row.get_unwrap(
            (SelectColumnIndexToProductData::MicroNutrientsStart as usize) + (m as usize),
        )
    });
    let mut micronutrients = MicroNutrients::default();
    for (m_type, value) in MicroNutrientsType::iter().zip(iter_micronutrients) {
        micronutrients[m_type] = value;
    }

    let iter_allowed_units = AllowedUnitsType::iter().map(|u| {
        row.get_unwrap((SelectColumnIndexToProductData::AllowedUnitsStart as usize) + (u as usize))
    });
    let mut allowed_units: AllowedUnits = HashMap::new();
    for (unit, quantity) in AllowedUnitsType::iter().zip(iter_allowed_units) {
        if let Some(qty) = quantity {
            allowed_units.insert(unit, qty);
        }
    }

    let product = Product::new(
        name,
        brand,
        Box::new(macro_elems),
        Box::new(micronutrients),
        allowed_units,
    );
    (
        row.get_unwrap(SelectColumnIndexToProductData::Id as usize),
        product,
    )
}

impl DbWrapper for LocalProductDb {
    fn get_products_matching_criteria(
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
        append_columns(
            SqlTablesNames::AllowedUnits,
            &mut AllowedUnitsType::iter().map(|m| Some(m.to_string())),
        );

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

        let mut stmt = self
            .sqlite_con
            .prepare(&query_template)
            .expect("Failed to prepare statement");

        let mut result_map = HashMap::new();
        let product_iter = stmt
            .query_map([], |row| Ok(map_query_row_to_product(row)))
            .expect("Failed to map query results")
            .map(|res| res.expect("Failed to map row to product"));

        result_map.extend(product_iter);
        result_map
    }

    fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: AllowedUnitsType,
        quantity: u16,
    ) -> Result<(), String> {
        let update_query = format!(
            "UPDATE {} SET \"{}\" = {} WHERE id = '{}';",
            SqlTablesNames::AllowedUnits,
            allowed_unit,
            quantity,
            product_id
        );
        self.sqlite_con
            .execute(update_query.as_str(), [])
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
impl MutableDbWrapper for LocalProductDb {
    fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let run_query = |table_name: &str,
                         columns_str: &str,
                         values_str: &str|
         -> Result<(), String> {
            self.sqlite_con
                .execute(
                    &format!("INSERT INTO {table_name} ({columns_str}) VALUES ({values_str});"),
                    [],
                )
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
                $it.map(|x| format!("\"{x}\", "))
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
                        "NULL, ".to_string()
                    } else {
                        format!("{}, ", val.unwrap())
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

    fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let run_query = |table: &str, col: &str, val: &str| {
            self.sqlite_con
                .execute(
                    &format!("UPDATE {table} SET \"{col}\" = {val} where id = '{product_id}';"),
                    [],
                )
                .unwrap_or_else(|_| panic!("Failed to update {col} for {product_id}"));
        };

        for (col, val) in [
            ("name", format!("'{}'", product.name())),
            (
                "brand",
                match product.brand() {
                    Some(brand) => format!("'{brand}'"),
                    None => "NULL".to_string(),
                },
            ),
        ] {
            run_query(&SqlTablesNames::Products.to_string(), col, &val);
        }

        for (col, val) in product
            .macro_elements
            .into_iter()
            .filter(|x| MacroElementsType::Calories != x.0)
        {
            run_query(
                &SqlTablesNames::MacroElements.to_string(),
                &col.to_string(),
                &val.to_string(),
            );
        }

        for col in MicroNutrientsType::iter() {
            let value = if product.micro_nutrients[col].is_some() {
                product.micro_nutrients[col].unwrap().to_string()
            } else {
                "NULL".to_string()
            };
            run_query(
                &SqlTablesNames::MicroNutrients.to_string(),
                &col.to_string(),
                &value,
            );
        }

        for col in AllowedUnitsType::iter() {
            let value = if product.allowed_units.contains_key(&col) {
                product.allowed_units.get(&col).unwrap().to_string()
            } else {
                "NULL".to_string()
            };
            run_query(
                &SqlTablesNames::AllowedUnits.to_string(),
                &col.to_string(),
                &value,
            );
        }

        Ok(())
    }

    fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
        let main_table_name = SqlTablesNames::Products.to_string();
        self.sqlite_con
            .execute(
                format!(
                    "DELETE FROM {main_table_name} WHERE id = '{product_id}';"
                )
                .as_str(),
                [],
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
        MicroNutrientsType,
    };
    use crate::db_wrappers::{DbSearchCriteria, DbWrapper, MutableDbWrapper};
    use rusqlite::{Connection, params};
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Once;
    use std::sync::atomic::{AtomicUsize, Ordering};
    #[allow(unused_imports)]
    use strum::IntoEnumIterator;

    fn assert_table_columns(connection: &Connection, table: &str, expected_columns: &[String]) {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1;",
                [table],
                |row| row.get(0),
            )
            .expect("Failed to check table existence");
        assert!(count > 0, "Expected '{table}' table to exist");

        let mut stmt = connection
            .prepare(&format!("SELECT name FROM pragma_table_info('{table}');"))
            .expect("Failed to prepare pragma_table_info statement");
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("Failed to read table columns")
            .map(|res| res.expect("Failed to extract column name"))
            .collect();

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
            let _db = LocalProductDb::new(path_str)
                .expect("Expected LocalProductDb::new to succeed for fresh database");
        }

        assert!(db_path.exists(), "Expected SQLite file to be created");

        let connection = Connection::open(&db_path).expect("Failed to open created database");

        let mut macro_columns = vec!["id".to_string()];
        macro_columns.extend(
            MacroElementsType::iter()
                .filter(|m| *m != MacroElementsType::Calories)
                .map(|m| m.to_string()),
        );

        let mut nutrient_columns = vec!["id".to_string()];
        nutrient_columns.extend(MicroNutrientsType::iter().map(|m| m.to_string()));

        let mut allowed_columns = vec!["id".to_string()];
        allowed_columns.extend(AllowedUnitsType::iter().map(|u| u.to_string()));

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
            allowed_units.insert(AllowedUnitsType::Piece, 1);
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
            db.add_product(product_id.as_str(), product)
                .expect("Expected add_product to succeed for persisted product");
        }

        let db = test_db.local_db();
        let results =
            db.get_products_matching_criteria(&[DbSearchCriteria::ById("Persisted".to_string())]);
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
            let db = LocalProductDb::new(path_str)
                .ok_or_else(|| "LocalProductDb::new returned None".to_string())?;
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

        fn connection(&self) -> Connection {
            Connection::open(&self.path).expect("Failed to reopen test database")
        }

        fn local_db(&self) -> LocalProductDb {
            LocalProductDb::new(
                self.path
                    .to_str()
                    .expect("Database path contains invalid UTF-8"),
            )
            .expect("Failed to reopen seeded LocalProductDb")
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

    fn seed_products(db: &mut LocalProductDb) -> Result<(), String> {
        let mut apple_allowed: AllowedUnits = HashMap::new();
        apple_allowed.insert(AllowedUnitsType::Piece, 1);
        apple_allowed.insert(AllowedUnitsType::Cup, 1);
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
        db.add_product(apple_id.as_str(), apple)
            .map_err(|e| format!("Failed to seed product {apple_id}: {e}"))?;

        let mut banana_allowed: AllowedUnits = HashMap::new();
        banana_allowed.insert(AllowedUnitsType::Piece, 1);
        banana_allowed.insert(AllowedUnitsType::Tablespoon, 2);
        banana_allowed.insert(AllowedUnitsType::Custom, 50);
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
        db.add_product(banana_id.as_str(), banana)
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
        let product_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM products;", [], |row| row.get(0))
            .expect("Failed to count products");
        assert_eq!(product_count, 2);
        let macro_columns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('macro_elements');",
                [],
                |row| row.get(0),
            )
            .expect("Failed to count macro columns");
        assert_eq!(macro_columns, 6);
    }

    #[test]
    fn test_03_get_products_matching_criteria_returns_expected_product() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let db = test_db.local_db();
        let results =
            db.get_products_matching_criteria(&[DbSearchCriteria::ById("Apple".to_string())]);
        assert_eq!(results.len(), 1);
        let apple = results
            .get("Apple (BrandA)")
            .expect("Missing Apple product");
        assert_eq!(apple.name(), "Apple");
        assert_eq!(apple.brand(), Some("BrandA"));
        assert_eq!(
            apple.macro_elements[MacroElementsType::Fat],
            0.2_f32,
            "Unexpected fat value"
        );
        assert_eq!(
            apple.micro_nutrients[MicroNutrientsType::Fiber],
            Some(2.4_f32)
        );
        assert_eq!(apple.allowed_units[&AllowedUnitsType::Piece], 1);
    }

    #[test]
    fn test_04_set_product_unit_updates_allowed_units_table() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let result = db.set_product_unit("Apple (BrandA)", AllowedUnitsType::Cup, 3);
        assert!(result.is_ok());
        let conn = test_db.connection();
        let updated: Option<u16> = conn
            .query_row(
                "SELECT cup FROM allowed_units WHERE id = ?1;",
                params!["Apple (BrandA)"],
                |row| row.get(0),
            )
            .expect("Failed to fetch updated unit");
        assert_eq!(updated, Some(3));
    }

    #[test]
    fn test_05_add_product_inserts_all_related_rows() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let mut allowed_units: AllowedUnits = HashMap::new();
        allowed_units.insert(AllowedUnitsType::Piece, 1);
        allowed_units.insert(AllowedUnitsType::Cup, 2);
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
            db.add_product(new_id.as_str(), new_product.clone()).is_ok(),
            "Expected add_product to succeed"
        );
        let conn = test_db.connection();
        let product_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id = ?1;",
                params![new_id],
                |row| row.get(0),
            )
            .expect("Failed to verify inserted product");
        assert_eq!(product_count, 1);
    }

    #[test]
    fn test_06_update_product_modifies_macro_and_micro_values() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        let mut allowed_units: AllowedUnits = HashMap::new();
        allowed_units.insert(AllowedUnitsType::Piece, 1);
        allowed_units.insert(AllowedUnitsType::Custom, 250);
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
            db.update_product("Apple (BrandA)", updated).is_ok(),
            "Expected update_product to succeed"
        );
        let conn = test_db.connection();
        let fiber: Option<f32> = conn
            .query_row(
                "SELECT Fiber FROM micronutrients WHERE id = ?1;",
                params!["Apple (BrandA)"],
                |row| row.get(0),
            )
            .expect("Failed to fetch updated fiber");
        assert_eq!(fiber, Some(3.0_f32));
    }

    #[test]
    fn test_07_delete_product_removes_all_rows() {
        let test_db = TestDbGuard::create_seeded().expect("Failed to prepare seeded database");
        let mut db = test_db.local_db();
        assert!(
            db.delete_product("Banana").is_ok(),
            "Expected delete_product to succeed"
        );
        let conn = test_db.connection();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE id = ?1;",
                params!["Banana"],
                |row| row.get(0),
            )
            .expect("Failed to check product deletion");
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
        let result = LocalProductDb::new(path_str);
        assert!(
            result.is_some(),
            "Expected LocalProductDb::new to return Some"
        );
        drop(result);
    }
}
