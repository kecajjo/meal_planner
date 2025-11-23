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
        let mut grams_per_unit = vec![
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Piece, 150);
                map.insert(crate::data_types::CommonUnits::Box, 50);
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Cup, 250);
                map.insert(crate::data_types::CommonUnits::Teaspoon, 5);
                map.insert(crate::data_types::CommonUnits::Tablespoon, 5);
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Cup, 250);
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Teaspoon, 1);
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Box, 50);
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(crate::data_types::CommonUnits::Piece, 1);
                map
            },
        ];

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
                grams_per_unit.pop().unwrap(),
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

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::data_types::{MacroElements, MicroNutrients, Product};

    #[test]
    fn test_new_and_sample_products() {
        let db = MockProductDb::new();
        // There should be 6 products as per create_sample_products
        assert_eq!(db.products.len(), 6);
        assert!(db.products.contains_key("Apple (BrandedApple)"));
        assert!(db.products.contains_key("Beer"));
        assert!(db.products.contains_key("Whiskey (BrandedWhiskey)"));
        assert!(db.products.contains_key("Salt"));
        assert!(db.products.contains_key("MixedNutrients (BrandedMixed)"));
        assert!(db.products.contains_key("Banana (BrandedBanana)"));
    }

    #[test]
    fn test_add_product() {
        let mut db = MockProductDb::new();
        let product = Product::new(
            "Orange".to_string(),
            Some("BrandedOrange".to_string()),
            Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0)),
            Box::new(MicroNutrients::default()),
            {
                let mut map = std::collections::HashMap::new();
                map.insert(crate::data_types::CommonUnits::Piece, 1);
                map
            },
        );
        db.add_product(product.clone());
        let key = db.get_product_id(&product);
        assert!(db.products.contains_key(&key));
        // Adding again should not duplicate
        db.add_product(product);
        assert_eq!(db.products.len(), 7);
    }

    #[test]
    fn test_edit_product() {
        let mut db = MockProductDb::new();
        let key = "Apple (BrandedApple)";
        let mut product = db.products[key].clone();
        let new_macros = Box::new(MacroElements::new(9.0, 8.0, 7.0, 6.0, 5.0));
        product.macro_elements = new_macros.clone();
        db.edit_product(product.clone());
        let updated = &db.products[key].macro_elements;
        use crate::data_types::MacroElemType;
        assert_eq!(updated[MacroElemType::Fat], new_macros[MacroElemType::Fat]);
        assert_eq!(
            updated[MacroElemType::SaturatedFat],
            new_macros[MacroElemType::SaturatedFat]
        );
        assert_eq!(
            updated[MacroElemType::Carbs],
            new_macros[MacroElemType::Carbs]
        );
        assert_eq!(
            updated[MacroElemType::Sugar],
            new_macros[MacroElemType::Sugar]
        );
        assert_eq!(
            updated[MacroElemType::Protein],
            new_macros[MacroElemType::Protein]
        );
    }

    #[test]
    fn test_get_mut_product() {
        let mut db = MockProductDb::new();
        let key = "Apple (BrandedApple)";
        {
            let prod = db.get_mut_product(key);
            assert!(prod.is_some());
            let prod = prod.unwrap();
            prod.grams_per_unit = {
                let mut map = std::collections::HashMap::new();
                map.insert(crate::data_types::CommonUnits::Cup, 200);
                map.insert(crate::data_types::CommonUnits::Piece, 150);
                map
            };
        }
        assert_eq!(db.products[key].grams_per_unit, {
            let mut map = std::collections::HashMap::new();
            map.insert(crate::data_types::CommonUnits::Cup, 200);
            map.insert(crate::data_types::CommonUnits::Piece, 150);
            map
        });
        // Non-existent
        assert!(db.get_mut_product("NonExistent ()").is_none());
    }

    #[test]
    fn test_get_products_matching_criteria_by_name() {
        let db = MockProductDb::new();
        use crate::db_wrappers::DbSearchCriteria;
        let crit = vec![DbSearchCriteria::ByName("App".to_string())];
        let results = db.get_products_matching_criteria(&crit);
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("Apple (BrandedApple)"));
    }

    #[test]
    #[should_panic(expected = "Mock DB does not support search by barcode")]
    fn test_get_products_matching_criteria_by_barcode_panics() {
        let db = MockProductDb::new();
        use crate::db_wrappers::DbSearchCriteria;
        let crit = vec![DbSearchCriteria::ByBarcode("12345".to_string())];
        let _ = db.get_products_matching_criteria(&crit);
    }
}
