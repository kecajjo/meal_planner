use std::collections::HashMap;

use crate::data_types::{
    CommonUnits, MacroElemType, MacroElements, MicroNutrients, MicroNutrientsType, Product,
};
use crate::db_wrappers::{DbSearchCriteria, DbWrapper, MutableDbWrapper};

#[cfg(not(test))]
const DATABASE_PATH: &str = "local_db.yaml";
#[cfg(test)]
const DATABASE_PATH: &str = "test_local_db.yaml";

pub(crate) struct LocalProductDb {
    products: HashMap<String, Product>,
}

impl LocalProductDb {
    // TODO: add locks to prevent data races. locks are inside fs:: module
    pub fn new() -> Self {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(DATABASE_PATH);
        LocalProductDb {
            products: HashMap::new(),
        }
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
