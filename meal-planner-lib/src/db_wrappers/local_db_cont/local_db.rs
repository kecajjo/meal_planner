use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::result;
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
    }

    // this function should be run only after upgrading version of this library (i.e., when new micro nutrient or unit is added)
    fn _update_table_columns(&self, table_name: SqlTablesNames) -> Result<(), String> {
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
    }

    fn create_tables(&self) {
        self.sqlite_con
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
            .expect(format!("Failed to create '{}' table", SqlTablesNames::Products).as_str());

        let table_schema = format!(
            "CREATE TABLE ?1 (
                id TEXT NOT NULL PRIMARY KEY,
                ?2
                FOREIGN KEY(id) REFERENCES {}(id)
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

        let allowed_units_to_text = |x: CommonUnits| match x {
            CommonUnits::Piece => format!("{} INTEGER NOT NULL DEFAULT 1,\n", x),
            _ => format!("{} INTEGER,\n", x),
        };
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
            DbSearchCriteria::ByName(name) => {
                query_fragment.push_str(&format!(
                    "{}.name LIKE '{}%'",
                    SqlTablesNames::Products,
                    name
                ));
            }
        }
    }
    Ok(query_fragment)
}

fn map_query_row_to_product(row: &rusqlite::Row) -> Result<(String, Product), rusqlite::Error> {
    // Implementation to map a database row to a Product struct
    unimplemented!()
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
        let mut append_columns = |table: SqlTablesNames, iter: &mut dyn Iterator<Item = String>| {
            for col in iter {
                query_template.push_str(&format!(", {}.{}", table, col));
            }
        };

        append_columns(
            SqlTablesNames::MacroElements,
            &mut MacroElemType::iter().map(|m| m.to_string()),
        );
        append_columns(
            SqlTablesNames::MicroNutrients,
            &mut MicroNutrientsType::iter().map(|m| m.to_string()),
        );
        append_columns(
            SqlTablesNames::AllowedUnits,
            &mut CommonUnits::iter().map(|m| m.to_string()),
        );

        query_template.push_str(&format!(
            " FROM {p}
            INNER JOIN {me} ON {p}.id = {me}.id
            INNER JOIN {au} ON {p}.id = {au}.id
            LEFT JOIN {mn} ON {p}.id = {mn}.id",
            p = SqlTablesNames::Products,
            me = SqlTablesNames::MacroElements,
            au = SqlTablesNames::AllowedUnits,
            mn = SqlTablesNames::MicroNutrients
        ));

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
            .query_map([], |row| map_query_row_to_product(row))
            .expect("Failed to map query results")
            .map(|res| res.expect("Failed to map row to product"));

        result_map.extend(product_iter);
        result_map
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
