use core::panic;
use std::collections::HashMap;

use crate::data_types::Product;

use super::local_db;
#[cfg(test)]
use super::mock_db;

pub enum DataBaseTypes {
    #[cfg(test)]
    Mock,
    OpenFoodFacts,
    Local,
}

pub enum DbSearchCriteria {
    ById(String),
    // ByBarcode(String),
}

pub fn get_db(db_type: DataBaseTypes) -> Option<Box<dyn DbWrapper>> {
    match db_type {
        #[cfg(test)]
        DataBaseTypes::Mock => Some(Box::new(mock_db::MockProductDb::new())),
        DataBaseTypes::Local => Some(Box::new(local_db::LocalProductDb::new(
            local_db::DATABASE_FILENAME,
        )?)),
        _ => panic!("Database type not supported in this build."),
        // DataBaseTypes::OpenFoodFactsDb => {
        //     Box::new(open_food_facts_wrapper::OpenFoodFactsDbWrapper::new())
        // }
    }
}

pub trait DbWrapper {
    fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, crate::data_types::Product>;

    fn get_product_default_id(&self, product: &Product) -> String {
        let brand = product.brand().unwrap_or_default();
        if brand.is_empty() {
            product.name().to_string()
        } else {
            format!("{} ({})", product.name(), brand)
        }
    }

    fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: crate::data_types::AllowedUnitsType,
        quantity: u16,
    ) -> Result<(), String>;

    fn update_product_units(
        &mut self,
        product_id: &str,
        allowed_units: &crate::data_types::AllowedUnits,
    ) -> Result<(), String> {
        for (unit, qty) in allowed_units {
            self.set_product_unit(product_id, *unit, *qty)?;
        }
        Ok(())
    }

    fn clone_product_units(
        &mut self,
        source_units: &crate::data_types::AllowedUnits,
        target_product_id: &str,
    ) -> Result<(), String> {
        let mut dest_prod = self
            .get_product_by_id(target_product_id)
            .ok_or_else(|| format!("Product with ID '{}' not found.", target_product_id))?;
        dest_prod.allowed_units = source_units.clone();
        self.update_product_units(target_product_id, &dest_prod.allowed_units)?;
        Ok(())
    }

    fn get_product_by_id(&self, product_id: &str) -> Option<crate::data_types::Product> {
        let mut results =
            self.get_products_matching_criteria(&[DbSearchCriteria::ById(product_id.to_string())]);
        results.remove(product_id)
    }
}

pub trait MutableDbWrapper: DbWrapper {
    fn add_product(
        &mut self,
        product_id: &str,
        product: crate::data_types::Product,
    ) -> Result<(), String>;
    fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String>;
    fn delete_product(&mut self, product_id: &str) -> Result<(), String>;
}

#[cfg(test)]
mod dbwrapper_trait_default_impl_tests {
    use super::*;
    use crate::data_types::{AllowedUnitsType, MacroElements, Product};
    use std::collections::HashMap;

    struct DummyDb {
        pub products: HashMap<String, Product>,
        pub set_calls: std::cell::RefCell<Vec<(String, AllowedUnitsType, u16)>>,
    }

    impl DbWrapper for DummyDb {
        fn get_products_matching_criteria(
            &self,
            criteria: &[DbSearchCriteria],
        ) -> HashMap<String, Product> {
            // Only ById supported for this dummy
            let mut map = HashMap::new();
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

        fn set_product_unit(
            &mut self,
            product_id: &str,
            allowed_unit: AllowedUnitsType,
            quantity: u16,
        ) -> Result<(), String> {
            self.set_calls
                .borrow_mut()
                .push((product_id.to_string(), allowed_unit, quantity));
            if self.products.contains_key(product_id) {
                Ok(())
            } else {
                Err(format!("Product with ID '{}' not found.", product_id))
            }
        }
    }

    fn make_product(name: &str, brand: Option<&str>) -> Product {
        Product::new(
            name.to_string(),
            brand.map(|b| b.to_string()),
            Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0)),
            Box::default(),
            {
                let mut map = HashMap::new();
                map.insert(AllowedUnitsType::Piece, 1);
                map
            },
        )
    }

    #[test]
    fn test_get_product_id_default_impl() {
        let db = DummyDb {
            products: HashMap::new(),
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let prod1 = make_product("Apple", Some("BrandA"));
        let prod2 = make_product("Banana", None);
        assert_eq!(db.get_product_default_id(&prod1), "Apple (BrandA)");
        assert_eq!(db.get_product_default_id(&prod2), "Banana");
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
        let result = db.update_product_units(
            db.get_product_default_id(&prod).as_str(),
            &prod.allowed_units,
        );
        assert!(result.is_ok());
        let calls = db.set_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Apple (BrandA)");
        assert_eq!(calls[0].1, AllowedUnitsType::Piece);
        assert_eq!(calls[0].2, 1);
    }

    #[test]
    fn test_clone_product_units_default_impl_success() {
        let mut products = HashMap::new();
        let mut source = make_product("Apple", Some("BrandA"));
        source.allowed_units.insert(AllowedUnitsType::Box, 5);
        let target = make_product("Banana", Some("BrandB"));
        products.insert("Apple (BrandA)".to_string(), source.clone());
        products.insert("Banana (BrandB)".to_string(), target.clone());
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = db.clone_product_units(&source.allowed_units, "Banana (BrandB)");
        assert!(result.is_ok());
        // Should have called set_product_unit for each allowed_unit in source
        let calls = db.set_calls.borrow();
        assert!(calls.iter().any(|(id, unit, qty)| id == "Banana (BrandB)"
            && *unit == AllowedUnitsType::Piece
            && *qty == 1));
        assert!(calls.iter().any(|(id, unit, qty)| id == "Banana (BrandB)"
            && *unit == AllowedUnitsType::Box
            && *qty == 5));
    }

    #[test]
    fn test_clone_product_units_default_impl_error() {
        let products = HashMap::new();
        let source = make_product("Apple", Some("BrandA"));
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = db.clone_product_units(&source.allowed_units, "NonExistent");
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
        let found = db.get_product_by_id("Apple (BrandA)");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "Apple");
        let not_found = db.get_product_by_id("NonExistent");
        assert!(not_found.is_none());
    }
}
