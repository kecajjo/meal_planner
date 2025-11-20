use std::collections::HashMap;

use crate::data_types::Product;

mod db_mock;
mod open_food_facts_wrapper;

enum DataBaseTypes {
    MockDb,
    OpenFoodFactsDb,
}

enum DbSearchCriteria {
    ByName(String),
    ByBarcode(String),
}

trait DbWrapper {
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
}

trait MutableDbWrapper: DbWrapper {
    fn add_product(&mut self, product: crate::data_types::Product);
    fn edit_product(&mut self, product: Product);
    fn get_mut_product(&mut self, name: &str) -> Option<&mut crate::data_types::Product>;
}
