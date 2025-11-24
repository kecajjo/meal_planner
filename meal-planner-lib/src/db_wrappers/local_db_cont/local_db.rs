use std::collections::HashMap;

use crate::data_types::{
    CommonUnits, MacroElemType, MacroElements, MicroNutrients, MicroNutrientsType, Product,
};

#[cfg(not(test))]
const DATABASE_PATH: &str = "local_db.yaml";
#[cfg(test)]
const DATABASE_PATH: &str = "test_local_db.yaml";

struct LocalProductDb {
    products: HashMap<String, Product>,
}

impl LocalProductDb {
    pub fn new() -> Self {
        LocalProductDb {
            products: HashMap::new(),
        }
    }
}
