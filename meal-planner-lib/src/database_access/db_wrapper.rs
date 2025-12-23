use core::panic;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::data_types::{Product, UnitData};
use async_trait::async_trait;

use super::local_db;
#[cfg(any(test, feature = "test-utils"))]
use super::mock_db;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataBaseTypes {
    #[cfg(any(test, feature = "test-utils"))]
    Mock,
    OpenFoodFacts,
    Local(String),
}

pub const LOCAL_DB_DEFAULT_FILE: &str = "local_db.sqlite3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DbSearchCriteria {
    ById(String),
    // ByBarcode(String),
}

/// Returns a database instance for the given type.
///
/// # Panics
/// Panics if the database type is not supported in this build.
#[must_use]
pub async fn get_db(db_type: DataBaseTypes) -> Option<Box<dyn Database>> {
    match db_type {
        #[cfg(any(test, feature = "test-utils"))]
        DataBaseTypes::Mock => Some(Box::new(mock_db::MockProductDb::new())),
        DataBaseTypes::Local(db_path) => {
            Some(Box::new(local_db::LocalProductDb::new(&db_path).await?))
        }
        _ => panic!("Database type not supported in this build."),
        // DataBaseTypes::OpenFoodFactsDb => {
        //     Box::new(open_food_facts_wrapper::OpenFoodFactsDbWrapper::new())
        // }
    }
}

/// Returns a mutable database instance for the given type.
///
/// # Panics
/// Panics if the database type is not mutable.
#[must_use]
pub async fn get_mutable_db(db_type: DataBaseTypes) -> Option<Box<dyn MutableDatabase>> {
    match db_type {
        #[cfg(any(test, feature = "test-utils"))]
        DataBaseTypes::Mock => Some(Box::new(mock_db::MockProductDb::new())),
        DataBaseTypes::Local(db_path) => {
            Some(Box::new(local_db::LocalProductDb::new(&db_path).await?))
        }
        _ => panic!("Database type not mutable."),
    }
}

#[must_use]
pub fn get_mutable_db_types() -> Vec<DataBaseTypes> {
    let types = vec![
        #[cfg(any(test, feature = "test-utils"))]
        DataBaseTypes::Mock,
        DataBaseTypes::Local("local_db.sqlite".to_string()),
    ];
    types
}

#[async_trait(?Send)]
pub trait Database {
    async fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> BTreeMap<String, crate::data_types::Product>;

    async fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: crate::data_types::AllowedUnitsType,
        unit_data: UnitData,
    ) -> Result<(), String>;

    async fn update_product_units(
        &mut self,
        product_id: &str,
        allowed_units: &crate::data_types::AllowedUnits,
    ) -> Result<(), String> {
        for (unit, qty) in allowed_units {
            self.set_product_unit(product_id, *unit, *qty).await?;
        }
        Ok(())
    }

    async fn clone_product_units(
        &mut self,
        source_units: &crate::data_types::AllowedUnits,
        target_product_id: &str,
    ) -> Result<(), String> {
        let mut dest_prod = self
            .get_product_by_id(target_product_id)
            .await
            .ok_or_else(|| format!("Product with ID '{target_product_id}' not found."))?;
        dest_prod.allowed_units = source_units.clone();
        self.update_product_units(target_product_id, &dest_prod.allowed_units)
            .await?;
        Ok(())
    }

    async fn get_product_by_id(&self, product_id: &str) -> Option<crate::data_types::Product> {
        let mut results = self
            .get_products_matching_criteria(&[DbSearchCriteria::ById(product_id.to_string())])
            .await;
        results.remove(product_id)
    }
}

#[async_trait(?Send)]
pub trait MutableDatabase: Database {
    async fn add_product(
        &mut self,
        product_id: &str,
        product: crate::data_types::Product,
    ) -> Result<(), String>;
    async fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String>;
    async fn delete_product(&mut self, product_id: &str) -> Result<(), String>;
}

#[cfg(test)]
mod dbwrapper_trait_default_impl_tests {
    use super::*;
    use crate::data_types::{AllowedUnitsType, MacroElements, Product};
    use async_trait::async_trait;
    use futures::executor::block_on;
    use std::collections::HashMap;

    struct DummyDb {
        pub products: HashMap<String, Product>,
        pub set_calls: std::cell::RefCell<Vec<(String, AllowedUnitsType, u16, u16)>>,
    }

    #[async_trait(?Send)]
    impl Database for DummyDb {
        async fn get_products_matching_criteria(
            &self,
            criteria: &[DbSearchCriteria],
        ) -> BTreeMap<String, Product> {
            // Only ById supported for this dummy
            let mut map = BTreeMap::new();
            for crit in criteria {
                // TODO: wont be necessary when there are more different criteria
                #[allow(irrefutable_let_patterns)]
                if let DbSearchCriteria::ById(name) = crit
                    && let Some(prod) = self.products.get(name)
                {
                    map.insert(name.clone(), prod.clone());
                }
            }
            map
        }

        async fn set_product_unit(
            &mut self,
            product_id: &str,
            allowed_unit: AllowedUnitsType,
            unit_data: UnitData,
        ) -> Result<(), String> {
            self.set_calls.borrow_mut().push((
                product_id.to_string(),
                allowed_unit,
                unit_data.amount,
                unit_data.divider,
            ));
            if self.products.contains_key(product_id) {
                Ok(())
            } else {
                Err(format!("Product with ID '{product_id}' not found."))
            }
        }
    }

    fn make_product(name: &str, brand: Option<&str>) -> Product {
        Product::new(
            name.to_string(),
            brand.map(std::string::ToString::to_string),
            Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0)),
            Box::default(),
            {
                let mut map = HashMap::new();
                map.insert(
                    AllowedUnitsType::Gram,
                    UnitData {
                        amount: 1,
                        divider: 1,
                    },
                );
                map
            },
        )
    }

    #[test]
    fn test_update_product_units_default_impl() {
        let mut products = HashMap::new();
        let prod = make_product("Apple", Some("BrandA"));
        products.insert("Apple (BrandA)".to_string(), prod.clone());
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = block_on(db.update_product_units(&prod.id(), &prod.allowed_units));
        assert!(result.is_ok());
        let calls = db.set_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Apple (BrandA)");
        assert_eq!(calls[0].1, AllowedUnitsType::Gram);
        assert_eq!(calls[0].2, 1);
    }

    #[test]
    fn test_clone_product_units_default_impl_success() {
        let mut products = HashMap::new();
        let mut source = make_product("Apple", Some("BrandA"));
        source.allowed_units.insert(
            AllowedUnitsType::Box,
            UnitData {
                amount: 5,
                divider: 1,
            },
        );
        let target = make_product("Banana", Some("BrandB"));
        products.insert("Apple (BrandA)".to_string(), source.clone());
        products.insert("Banana (BrandB)".to_string(), target.clone());
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = block_on(db.clone_product_units(&source.allowed_units, "Banana (BrandB)"));
        assert!(result.is_ok());
        // Should have called set_product_unit for each allowed_unit in source
        let calls = db.set_calls.borrow();
        assert!(
            calls
                .iter()
                .any(|(id, unit, amount, divider)| id == "Banana (BrandB)"
                    && *unit == AllowedUnitsType::Gram
                    && *amount == 1
                    && *divider == 1)
        );
        assert!(
            calls
                .iter()
                .any(|(id, unit, amount, divider)| id == "Banana (BrandB)"
                    && *unit == AllowedUnitsType::Box
                    && *amount == 5
                    && *divider == 1)
        );
    }

    #[test]
    fn test_clone_product_units_default_impl_error() {
        let products = HashMap::new();
        let source = make_product("Apple", Some("BrandA"));
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = block_on(db.clone_product_units(&source.allowed_units, "NonExistent"));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Product with ID 'NonExistent' not found."
        );
    }

    #[test]
    fn test_get_product_by_id_default_impl() {
        let mut products = HashMap::new();
        let prod = make_product("Apple", Some("BrandA"));
        products.insert("Apple (BrandA)".to_string(), prod.clone());
        let db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let found = block_on(db.get_product_by_id("Apple (BrandA)"));
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Apple");
        let not_found = block_on(db.get_product_by_id("NonExistent"));
        assert!(not_found.is_none());
    }
}
