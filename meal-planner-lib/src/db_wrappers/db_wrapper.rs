use core::panic;
use std::collections::HashMap;

use crate::data_types::Product;

use super::local_db;
#[cfg(test)]
use super::mock_db;

pub enum DataBaseTypes {
    #[cfg(test)]
    MockDb,
    OpenFoodFactsDb,
    LocalDb,
}

pub enum DbSearchCriteria {
    ByName(String),
    ByBarcode(String),
}

pub fn get_db(db_type: DataBaseTypes) -> Option<Box<dyn DbWrapper>> {
    match db_type {
        #[cfg(test)]
        DataBaseTypes::MockDb => Some(Box::new(mock_db::MockProductDb::new())),
        DataBaseTypes::LocalDb => Some(Box::new(local_db::LocalProductDb::new()?)),
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

    fn get_product_id(&self, product: &Product) -> String {
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
        allowed_unit: crate::data_types::CommonUnits,
        quantity: u16,
    ) -> Result<(), String>;

    fn update_product_units(&mut self, product: &crate::data_types::Product) -> Result<(), String> {
        for (unit, qty) in &product.allowed_units {
            self.set_product_unit(&self.get_product_id(product), *unit, *qty)?;
        }
        Ok(())
    }

    fn clone_product_units(
        &mut self,
        source_product: &crate::data_types::Product,
        target_product_id: &str,
    ) -> Result<(), String> {
        let mut dest_prod = self
            .get_product_by_id(target_product_id)
            .ok_or_else(|| format!("Product with ID '{}' not found.", target_product_id))?;
        dest_prod.allowed_units = source_product.allowed_units.clone();
        self.update_product_units(&dest_prod)?;
        Ok(())
    }

    fn get_product_by_id(&self, product_id: &str) -> Option<crate::data_types::Product> {
        let mut results = self
            .get_products_matching_criteria(&[DbSearchCriteria::ByName(product_id.to_string())]);
        results.remove(product_id)
    }
}

pub trait MutableDbWrapper: DbWrapper {
    fn add_product(&mut self, product: crate::data_types::Product) -> Result<(), String>;
    fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String>;
    fn get_mut_product(&mut self, name: &str) -> Option<&mut crate::data_types::Product>;
}

#[cfg(test)]
mod dbwrapper_trait_default_impl_tests {
    use super::*;
    use crate::data_types::{CommonUnits, MacroElements, MicroNutrients, Product};
    use std::collections::HashMap;

    struct DummyDb {
        pub products: HashMap<String, Product>,
        pub set_calls: std::cell::RefCell<Vec<(String, CommonUnits, u16)>>,
    }

    impl DbWrapper for DummyDb {
        fn get_products_matching_criteria(
            &self,
            criteria: &[DbSearchCriteria],
        ) -> HashMap<String, Product> {
            // Only ByName supported for this dummy
            let mut map = HashMap::new();
            for crit in criteria {
                if let DbSearchCriteria::ByName(name) = crit {
                    if let Some(prod) = self.products.get(name) {
                        map.insert(name.clone(), prod.clone());
                    }
                }
            }
            map
        }

        fn set_product_unit(
            &mut self,
            product_id: &str,
            allowed_unit: CommonUnits,
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
            Box::new(MicroNutrients::default()),
            {
                let mut map = HashMap::new();
                map.insert(CommonUnits::Piece, 1);
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
        assert_eq!(db.get_product_id(&prod1), "Apple (BrandA)");
        assert_eq!(db.get_product_id(&prod2), "Banana");
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
        let result = db.update_product_units(&prod);
        assert!(result.is_ok());
        let calls = db.set_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "Apple (BrandA)");
        assert_eq!(calls[0].1, CommonUnits::Piece);
        assert_eq!(calls[0].2, 1);
    }

    #[test]
    fn test_clone_product_units_default_impl_success() {
        let mut products = HashMap::new();
        let mut source = make_product("Apple", Some("BrandA"));
        source.allowed_units.insert(CommonUnits::Box, 5);
        let target = make_product("Banana", Some("BrandB"));
        products.insert("Apple (BrandA)".to_string(), source.clone());
        products.insert("Banana (BrandB)".to_string(), target.clone());
        let mut db = DummyDb {
            products,
            set_calls: std::cell::RefCell::new(vec![]),
        };
        let result = db.clone_product_units(&source, "Banana (BrandB)");
        assert!(result.is_ok());
        // Should have called set_product_unit for each allowed_unit in source
        let calls = db.set_calls.borrow();
        assert!(calls.iter().any(|(id, unit, qty)| id == "Banana (BrandB)"
            && *unit == CommonUnits::Piece
            && *qty == 1));
        assert!(calls.iter().any(|(id, unit, qty)| id == "Banana (BrandB)"
            && *unit == CommonUnits::Box
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
        let result = db.clone_product_units(&source, "NonExistent");
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
