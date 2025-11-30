use std::collections::HashMap;

use super::db_wrapper::{DbSearchCriteria, DbWrapper, MutableDbWrapper};
use crate::data_types::{MacroElements, MicroNutrients, MicroNutrientsType, Product, UnitData};

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

    #[allow(clippy::too_many_lines)]
    fn create_sample_products(&mut self) {
        let macro_elements = [
            Box::new(MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0)),
            Box::new(MacroElements::new(0.5, 1.0, 0.5, 2.0, 1.0)),
            Box::new(MacroElements::new(2.0, 1.5, 1.0, 0.5, 0.0)),
            Box::new(MacroElements::new(1.5, 2.0, 0.5, 1.0, 2.5)),
            Box::new(MacroElements::new(0.0, 0.5, 1.5, 2.0, 1.0)),
            Box::new(MacroElements::new(2.5, 1.0, 0.0, 0.5, 1.5)),
        ];
        let mut micro_nutrients: [Box<MicroNutrients>; 6] = [
            Box::default(),
            Box::default(),
            Box::default(),
            Box::default(),
            Box::default(),
            Box::default(),
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
        let mut allowed_units = vec![
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Gram,
                    UnitData {
                        amount: 1,
                        divider: 1,
                    },
                );
                map.insert(
                    crate::data_types::AllowedUnitsType::Box,
                    UnitData {
                        amount: 100,
                        divider: 1,
                    },
                );
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Cup,
                    UnitData {
                        amount: 250,
                        divider: 1,
                    },
                );
                map.insert(
                    crate::data_types::AllowedUnitsType::Teaspoon,
                    UnitData {
                        amount: 5,
                        divider: 1,
                    },
                );
                map.insert(
                    crate::data_types::AllowedUnitsType::Tablespoon,
                    UnitData {
                        amount: 5,
                        divider: 1,
                    },
                );
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Cup,
                    UnitData {
                        amount: 250,
                        divider: 1,
                    },
                );
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Teaspoon,
                    UnitData {
                        amount: 1,
                        divider: 1,
                    },
                );
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Box,
                    UnitData {
                        amount: 50,
                        divider: 1,
                    },
                );
                map
            },
            {
                let mut map = HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Gram,
                    UnitData {
                        amount: 1,
                        divider: 1,
                    },
                );
                map
            },
        ];

        let names = [
            "Apple",
            "Beer",
            "Whiskey",
            "Salt",
            "MixedNutrients",
            "Banana",
        ];
        let brands = [
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
                allowed_units.pop().unwrap(),
            ));
        }
    }

    fn add_or_modify_product(&mut self, product: Product) {
        self.products.insert(product.id(), product);
    }
}

impl MutableDbWrapper for MockProductDb {
    fn add_product(
        &mut self,
        product_id: &str,
        product: crate::data_types::Product,
    ) -> Result<(), String> {
        if self.products.contains_key(product_id) {
            return Err(format!(
                "Product with ID '{}' already exists.",
                product.id()
            ));
        }
        self.add_or_modify_product(product);
        Ok(())
    }

    fn update_product(&mut self, product_id: &str, product: Product) -> Result<(), String> {
        if !self.products.contains_key(product_id) {
            return Err(format!("Product with ID '{product_id}' not found."));
        }
        self.add_or_modify_product(product);
        Ok(())
    }

    fn delete_product(&mut self, product_id: &str) -> Result<(), String> {
        if self.products.remove(product_id).is_some() {
            Ok(())
        } else {
            Err(format!("Product with ID '{product_id}' not found."))
        }
    }
}

impl DbWrapper for MockProductDb {
    fn get_products_matching_criteria(
        &self,
        criteria: &[DbSearchCriteria],
    ) -> HashMap<String, crate::data_types::Product> {
        let is_prod_matching_crit = |product: &Product, criterion: &DbSearchCriteria| -> bool {
            match criterion {
                DbSearchCriteria::ById(name_crit) => product.id().starts_with(name_crit),
            }
        };

        let mut results = HashMap::new();

        for (name, product) in &self.products {
            if criteria
                .iter()
                .all(|crit| is_prod_matching_crit(product, crit))
            {
                results.insert(name.clone(), product.clone());
            }
        }

        results
    }

    fn set_product_unit(
        &mut self,
        product_id: &str,
        allowed_unit: crate::data_types::AllowedUnitsType,
        unit_data: UnitData,
    ) -> Result<(), String> {
        let product = self
            .products
            .get_mut(product_id)
            .ok_or_else(|| format!("Product with ID '{product_id}' not found."))?;
        product.allowed_units.insert(allowed_unit, unit_data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::data_types::{MacroElements, Product};
    use crate::db_wrappers::DbSearchCriteria;
    use approx::assert_relative_eq;

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
            Box::default(),
            {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    crate::data_types::AllowedUnitsType::Gram,
                    UnitData {
                        amount: 1,
                        divider: 1,
                    },
                );
                map
            },
        );
        assert!(
            db.add_product(product.id().as_str(), product.clone())
                .is_ok()
        );
        let key = product.id();
        assert!(db.products.contains_key(&key));
        // Adding again should not duplicate
        assert!(db.add_product(product.id().as_str(), product).is_err());
        assert_eq!(db.products.len(), 7);
    }

    #[test]
    fn test_update_product() {
        use crate::data_types::MacroElementsType;
        let mut db = MockProductDb::new();
        let key = "Apple (BrandedApple)";
        let mut product = db.products[key].clone();
        let new_macros = Box::new(MacroElements::new(9.0, 8.0, 7.0, 6.0, 5.0));
        product.macro_elements = new_macros.clone();
        assert!(
            db.update_product(product.id().as_str(), product.clone())
                .is_ok()
        );
        let updated = &db.products[key].macro_elements;
        assert_relative_eq!(
            updated[MacroElementsType::Fat],
            new_macros[MacroElementsType::Fat]
        );
        assert_relative_eq!(
            updated[MacroElementsType::SaturatedFat],
            new_macros[MacroElementsType::SaturatedFat]
        );
        assert_relative_eq!(
            updated[MacroElementsType::Carbs],
            new_macros[MacroElementsType::Carbs]
        );
        assert_relative_eq!(
            updated[MacroElementsType::Sugar],
            new_macros[MacroElementsType::Sugar]
        );
        assert_relative_eq!(
            updated[MacroElementsType::Protein],
            new_macros[MacroElementsType::Protein]
        );
    }

    #[test]
    fn test_get_products_matching_criteria_by_name() {
        let db = MockProductDb::new();
        let crit = vec![DbSearchCriteria::ById("App".to_string())];
        let results = db.get_products_matching_criteria(&crit);
        assert_eq!(results.len(), 1);
        assert!(results.contains_key("Apple (BrandedApple)"));
    }

    #[test]
    fn test_set_product_unit_success() {
        let mut db = MockProductDb::new();
        let product_id = "Apple (BrandedApple)";
        let unit = crate::data_types::AllowedUnitsType::Cup;
        let unit_data = UnitData {
            amount: 123,
            divider: 2,
        };
        let result = db.set_product_unit(product_id, unit, unit_data);
        assert!(result.is_ok());
        let product = db.products.get(product_id).unwrap();
        assert_eq!(product.allowed_units.get(&unit), Some(&unit_data));
    }

    #[test]
    fn test_set_product_unit_error() {
        let mut db = MockProductDb::new();
        let product_id = "NonExistentProduct";
        let unit = crate::data_types::AllowedUnitsType::Cup;
        let unit_data = UnitData {
            amount: 123,
            divider: 1,
        };
        let result = db.set_product_unit(product_id, unit, unit_data);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            format!("Product with ID '{product_id}' not found.")
        );
    }
}
