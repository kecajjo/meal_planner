use std::collections::HashMap;

use crate::data_types::{AllowedUnitsType, Product, UnitData};
use crate::database_access::{Database, DbSearchCriteria, MutableDatabase};

#[cfg(not(target_arch = "wasm32"))]
use crate::database_access::local_db_cont::local_db_generic;
#[cfg(target_arch = "wasm32")]
use crate::database_access::local_db_cont::local_db_wasm;

#[cfg(not(target_arch = "wasm32"))]
use local_db_generic::LocalProductDbConcrete;

#[cfg(target_arch = "wasm32")]
use local_db_wasm::LocalProductDbConcrete;

pub struct LocalProductDb {
    inner: LocalProductDbConcrete,
}
#[allow(warnings)]
impl LocalProductDb {
    /// Creates a new local database instance backed by `SQLite`.
    pub fn new(database_file: &str) -> Option<Self> {
        LocalProductDbConcrete::new(database_file).map(|inner| Self { inner })
    }
}

impl Database for LocalProductDb {
    fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, Product> {
        self.inner.get_products_matching_criteria(criteria)
    }

    fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: AllowedUnitsType,
        unit_data: UnitData,
    ) -> Result<(), String> {
        self.inner
            .set_product_unit(product_id, allowed_unit, unit_data)
    }
}

impl MutableDatabase for LocalProductDb {
    fn add_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        self.inner.add_product(product_id, product)
    }

    fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        self.inner.update_product(product_id, product)
    }

    fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
        self.inner.delete_product(product_id)
    }
}
