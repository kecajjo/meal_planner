use core::fmt;
use std::collections::{HashMap, HashSet};
use strum::IntoEnumIterator;

use crate::data_types::{
    CommonUnits, MacroElemType, MacroElements, MicroNutrients, MicroNutrientsType, Product,
};
use crate::db_wrappers::{DbSearchCriteria, DbWrapper, MutableDbWrapper};
use const_format::concatcp;
use rusqlite;

// cant make paths relative to this file
const DATABASE_PATH: &str = "src/db_wrappers/local_db_cont/";
#[cfg(not(test))]
const DATABASE_FILENAME: &str = concatcp!(DATABASE_PATH, "local_db.sqlite");
#[cfg(test)]
const DATABASE_FILENAME: &str = concatcp!(DATABASE_PATH, "test_local_db.sqlite");

pub(crate) struct LocalProductDb {
    sqlite_con: rusqlite::Connection,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
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
        write!(f, "{}", table_name)
    }
}

// TODO panicking to be replaced with proper error handling
impl LocalProductDb {
    pub fn new() -> Option<Self> {
        let con =
            rusqlite::Connection::open(DATABASE_PATH).expect("Failed to open SQLite database");
        Some(LocalProductDb { sqlite_con: con })
    }

    fn init_db_if_new_created(&self) {
        let mut stmt = self
            .sqlite_con
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='products';")
            .expect("Failed to prepare statement");
        let mut rows = stmt.query([]).expect("Failed to execute query");
        if rows.next().expect("Failed to fetch row").is_none() {
            self.create_tables();
        }

        // make sure MicroNutrients and AllowedUnits are up to date
    }

    fn update_table_columns(&self, table_name: SqlTablesNames) -> Result<(), String> {
        let (all_columns, col_type) = match table_name {
            t @ (SqlTablesNames::Products | SqlTablesNames::MacroElements) => {
                return Err(format!("{} table should have all necessary columns", t));
            }
            SqlTablesNames::MicroNutrients => (
                MicroNutrientsType::iter()
                    .map(|x| x.to_string())
                    .collect::<HashSet<String>>(),
                "FLOAT",
            ),
            SqlTablesNames::AllowedUnits => (
                CommonUnits::iter()
                    .map(|x: CommonUnits| x.to_string())
                    .collect::<HashSet<String>>(),
                "INTEGER",
            ),
        };

        let mut stmt = self
            .sqlite_con
            .prepare(format!("SELECT name FROM pragma_table_info('{}')", table_name).as_str())
            .expect(format!("Getting columns names of the {} table failed", table_name).as_str());

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
            let alter_table_query = format!(
                "ALTER TABLE {} ADD COLUMN {} {};",
                table_name, col, col_type
            );
            self.sqlite_con
                .execute(alter_table_query.as_str(), [])
                .map_err(|e| {
                    format!(
                        "Failed to add column '{}' to table '{}': {}",
                        col, table_name, e
                    )
                })?;
        }

        Ok(())

        // Now `columns` contains the names of all columns in the MicroNutrients table
    }

    fn create_tables(&self) {
        self.sqlite_con
            .execute(
                format!(
                    "CREATE TABLE {} (
                    id TEXT PRIMARY KEY,
                    name CHAR NOT NULL,
                    brand CHAR
                )",
                    SqlTablesNames::Products
                )
                .as_str(),
                [],
            )
            .expect(format!("Failed to create '{}' table", SqlTablesNames::Products).as_str());

        let table_schema = format!(
            "CREATE TABLE ?1 (
                id TEXT PRIMARY KEY,
                ?2
                FOREIGN KEY(product_id) REFERENCES {}(id)
            )",
            SqlTablesNames::Products
        );
        let macro_elem_to_text = |x: MacroElemType| format!("{} FLOAT NOT NULL,\n", x);
        let macro_elements_fields: String = MacroElemType::iter().map(macro_elem_to_text).collect();
        self.create_table_for_table_name(
            SqlTablesNames::MacroElements.to_string().as_str(),
            table_schema.as_str(),
            macro_elements_fields.as_str(),
        )
        .expect(format!("Failed to create '{}' table", SqlTablesNames::MacroElements).as_str());

        let micronutrients_to_text = |x: MicroNutrientsType| format!("{} FLOAT,\n", x);
        let micronutrients_fields: String = MicroNutrientsType::iter()
            .map(micronutrients_to_text)
            .collect();
        self.create_table_for_table_name(
            SqlTablesNames::MicroNutrients.to_string().as_str(),
            table_schema.as_str(),
            micronutrients_fields.as_str(),
        )
        .expect(
            format!(
                "Failed to create '{}' table",
                SqlTablesNames::MicroNutrients
            )
            .as_str(),
        );

        let allowed_units_to_text = |x: CommonUnits| format!("{} INTEGER,\n", x);
        let allowed_units_fields: String = CommonUnits::iter().map(allowed_units_to_text).collect();
        self.create_table_for_table_name(
            SqlTablesNames::AllowedUnits.to_string().as_str(),
            table_schema.as_str(),
            allowed_units_fields.as_str(),
        )
        .expect(format!("Failed to create '{}' table", SqlTablesNames::AllowedUnits).as_str());
    }

    fn create_table_for_table_name(
        &self,
        table_name: &str,
        table_schema: &str,
        data: &str,
    ) -> Result<(), String> {
        self.sqlite_con
            .execute(table_schema, [table_name, data])
            .map_err(|e| format!("Failed to create '{}' table: {}", table_name, e))?;
        Ok(())
    }
}

impl DbWrapper for LocalProductDb {
    fn get_products_matching_criteria(
        &self,
        _criteria: &[DbSearchCriteria],
    ) -> HashMap<String, Product> {
        unimplemented!()
    }

    fn set_product_unit(
        &mut self,
        _product_id: &str,
        _allowed_unit: CommonUnits,
        _quantity: u16,
    ) -> Result<(), String> {
        unimplemented!()
    }

    // The following methods have default implementations in the trait, so we do not need to implement them unless overriding.
}

impl MutableDbWrapper for LocalProductDb {
    fn add_product(&mut self, _product: Product) -> Result<(), String> {
        unimplemented!()
    }

    fn update_product(&mut self, _product_id: &str, _product: Product) -> Result<(), String> {
        unimplemented!()
    }

    fn get_mut_product(&mut self, _name: &str) -> Option<&mut Product> {
        unimplemented!()
    }
}
