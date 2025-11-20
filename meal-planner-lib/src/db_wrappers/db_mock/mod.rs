use core::panic;
use std::collections::HashMap;

use super::{DbSearchCriteria, DbWrapper, MutableDbWrapper};
use crate::data_types::{MacroElements, MicroNutrients, MicroNutrientsType, Product};

pub struct MockProductDb {
    pub products: HashMap<String, Product>,
}

impl MockProductDb {
    pub fn new() -> Self {
        let mut me = Self {
            products: HashMap::new(),
        };
        me.create_sample_products();
        me
    }

    fn create_sample_products(&mut self) {
        let macro_elements = vec![
            Box::new(MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0)),
            Box::new(MacroElements::new(0.5, 1.0, 0.5, 2.0, 1.0)),
            Box::new(MacroElements::new(2.0, 1.5, 1.0, 0.5, 0.0)),
            Box::new(MacroElements::new(1.5, 2.0, 0.5, 1.0, 2.5)),
            Box::new(MacroElements::new(0.0, 0.5, 1.5, 2.0, 1.0)),
            Box::new(MacroElements::new(2.5, 1.0, 0.0, 0.5, 1.5)),
        ];
        let mut micro_nutrients = vec![
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
        ];
        micro_nutrients[0][MicroNutrientsType::Fiber] = Some(2.5);
        micro_nutrients[0][MicroNutrientsType::Zinc] = Some(3.5);
        micro_nutrients[1][MicroNutrientsType::Alcohol] = Some(15.0);
        micro_nutrients[2][MicroNutrientsType::Alcohol] = Some(100.0);
        micro_nutrients[3][MicroNutrientsType::Sodium] = Some(3.0);
        micro_nutrients[4][MicroNutrientsType::Zinc] = Some(6.0);
        micro_nutrients[4][MicroNutrientsType::Fiber] = Some(6.0);
        micro_nutrients[4][MicroNutrientsType::Sodium] = Some(8.0);
        micro_nutrients[4][MicroNutrientsType::Alcohol] = Some(7.0);
        micro_nutrients[5][MicroNutrientsType::Fiber] = Some(5.0);
        let names = vec![
            "Apple",
            "Beer",
            "Whiskey",
            "Salt",
            "MixedNutrients",
            "Banana",
        ];
        let brands = vec![
            Some("BrandedApple".to_string()),
            None,
            Some("BrandedWhiskey".to_string()),
            None,
            Some("BrandedMixed".to_string()),
            Some("BrandedBanana".to_string()),
        ];
        for i in 0..6 {
            self.add_or_modify_product(Product::new(
                names[i].to_string(),
                brands[i].clone(),
                macro_elements[i].clone(),
                micro_nutrients[i].clone(),
                100,
            ));
        }
    }

    fn add_or_modify_product(&mut self, product: Product) {
        self.products.insert(self.get_product_id(&product), product);
    }
}

impl MutableDbWrapper for MockProductDb {
    fn add_product(&mut self, product: crate::data_types::Product) {
        if !self.products.contains_key(&self.get_product_id(&product)) {
            self.add_or_modify_product(product);
        }
    }

    fn edit_product(&mut self, product: Product) {
        if self.products.contains_key(&self.get_product_id(&product)) {
            self.add_or_modify_product(product);
        }
    }

    fn get_mut_product(&mut self, name: &str) -> Option<&mut crate::data_types::Product> {
        self.products.get_mut(name)
    }
}

impl DbWrapper for MockProductDb {
    fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, crate::data_types::Product> {
        let is_prod_matching_crit = |product: &Product, criterion: &DbSearchCriteria| -> bool {
            match criterion {
                DbSearchCriteria::ByName(name_crit) => {
                    self.get_product_id(product).starts_with(name_crit)
                }
                DbSearchCriteria::ByBarcode(barcode_crit) => {
                    panic!(
                        "Mock DB does not support search by barcode. Tried to search for {}",
                        barcode_crit
                    );
                }
            }
        };

        let mut results = HashMap::new();

        for (name, product) in self.products.iter() {
            if criteria
                .iter()
                .all(|crit| is_prod_matching_crit(product, crit))
            {
                results.insert(name.clone(), product.clone());
            }
        }

        results
    }
}
