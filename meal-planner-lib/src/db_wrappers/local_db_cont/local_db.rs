use core::fmt;
use std::collections::{HashMap, HashSet};
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::EnumIter;

use crate::data_types::{
    AllowedUnits, AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients,
    MicroNutrientsType, Product,
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
                AllowedUnitsType::iter()
                    .map(|x: AllowedUnitsType| x.to_string())
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
        let macro_elem_to_text = |x: MacroElementsType| {
            if MacroElementsType::Calories == x {
                "".to_string()
            } else {
                format!("{} FLOAT NOT NULL,\n", x)
            }
        };
        let macro_elements_fields: String =
            MacroElementsType::iter().map(macro_elem_to_text).collect();
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

        let allowed_units_to_text = |x: AllowedUnitsType| match x {
            AllowedUnitsType::Piece => format!("{} INTEGER NOT NULL DEFAULT 1,\n", x),
            _ => format!("{} INTEGER,\n", x),
        };
        let allowed_units_fields: String = AllowedUnitsType::iter()
            .map(allowed_units_to_text)
            .collect();
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
    Ok((
        row.get_unwrap(SelectColumnIndexToProductData::Id as usize),
        product,
    ))
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
                for col in iter {
                    if let Some(col) = col {
                        query_template.push_str(&format!(", {}.{}", table, col));
                    }
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
        product_id: &str,
        allowed_unit: AllowedUnitsType,
        quantity: u16,
    ) -> Result<(), String> {
        let update_query = format!(
            "UPDATE {} SET {} = {} WHERE id = {};",
            SqlTablesNames::AllowedUnits,
            allowed_unit,
            quantity,
            product_id
        );
        self.sqlite_con
            .execute(update_query.as_str(), [])
            .map_err(|e| {
                format!(
                    "Failed to update allowed unit {} for product {}: {}",
                    allowed_unit, product_id, e
                )
            })?;
        Ok(())
    }
}

impl MutableDbWrapper for LocalProductDb {
    fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        let query_template = "INSERT INTO ?1 (?2) VALUES (?3);";
        let run_query =
            |table_name: &str, columns_str: &str, values_str: &str| -> Result<(), String> {
                self.sqlite_con
                    .execute(query_template, [table_name, columns_str, values_str])
                    .map_err(|e| {
                        format!(
                            "Failed to insert product '{}' into {} table: {}",
                            product_id, table_name, e
                        )
                    })?;
                Ok(())
            };

        run_query(
            &SqlTablesNames::Products.to_string(),
            "id, name, brand",
            format!(
                "{}, {}, {}",
                product_id,
                product.name(),
                product.brand().unwrap_or("NULL")
            )
            .as_str(),
        )?;

        macro_rules! map_function_for_add {
            ($it:expr, MacroElements, columns) => {
                $it.filter_map(|x| match x {
                    MacroElementsType::Calories => None,
                    _ => Some(format!("{}, ", x)),
                })
            };
            ($it:expr, MicroNutrients, columns) => {
                $it.map(|x| format!("{}, ", x))
            };
            ($it:expr, AllowedUnits, columns) => {
                $it.map(|x| format!("{}, ", x))
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
                    format!("id, {}", col_names).as_str(),
                    format!("{}, {}", self.get_product_default_id(&product), values).as_str(),
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
        let query_template = format!("UPDATE ?1 SET ?2 = ?3 where id = {}", product_id);
        let run_query = |table: &str, col: &str, val: &str| {
            self.sqlite_con
                .execute(&query_template, [table, col, val])
                .expect(&format!(
                    "Failed to update {col} for {id}",
                    col = col,
                    id = product_id
                ));
        };

        for (col, val) in vec![
            ("name", product.name()),
            ("brand", product.brand().unwrap_or("NULL")),
        ] {
            run_query(&SqlTablesNames::Products.to_string(), col, val);
        }

        for (col, val) in product.macro_elements.into_iter().filter(|x| {
            if MacroElementsType::Calories == x.0 {
                return false;
            } else {
                return true;
            }
        }) {
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
            let value = if product.allowed_units.get(&col).is_some() {
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
        for table in SqlTablesNames::iter() {
            self.sqlite_con
                .execute(
                    format!("DELETE FROM {} WHERE id = {};", table, product_id).as_str(),
                    [],
                )
                .map_err(|e| {
                    format!(
                        "Failed to delete product with ID '{}' from table '{}': {}",
                        product_id, table, e
                    )
                })?;
        }
        Ok(())
    }
}
