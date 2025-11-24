use super::NutrientConstraint;
use super::ProductConstraint;

pub struct MealConstraint {
    pub products: Vec<ProductConstraint>,
    pub nutrients: Vec<NutrientConstraint>,
}

#[cfg(test)]
mod tests {

    use super::super::*;
    use super::*;
    use crate::data_types::{CommonUnits, MacroElemType, MacroElements, MicroNutrients, Product};

    fn initialize_meal_with_products() -> MealConstraint {
        let mut macro_elements_vec = vec![
            Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0)),
            Box::new(MacroElements::new(2.0, 3.0, 4.0, 5.0, 6.0)),
            Box::new(MacroElements::new(3.0, 4.0, 5.0, 6.0, 7.0)),
            Box::new(MacroElements::new(3.0, 4.0, 5.0, 6.0, 7.0)),
        ];
        let mut micro_nutrients_vec = vec![
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
            Box::new(MicroNutrients::default()),
        ];
        let mut grams1 = std::collections::HashMap::new();
        grams1.insert(CommonUnits::Cup, 200);
        grams1.insert(CommonUnits::Piece, 150);
        let mut grams2 = std::collections::HashMap::new();
        grams2.insert(CommonUnits::Tablespoon, 10);
        let mut grams3 = std::collections::HashMap::new();
        grams3.insert(CommonUnits::Piece, 100);
        let mut grams4 = std::collections::HashMap::new();
        grams4.insert(CommonUnits::Piece, 50);
        let prod1 = Box::new(Product::new(
            "Apple".to_string(),
            None,
            macro_elements_vec.pop().unwrap(),
            micro_nutrients_vec.pop().unwrap(),
            grams1,
        ));
        let prod2 = Box::new(Product::new(
            "Banana".to_string(),
            None,
            macro_elements_vec.pop().unwrap(),
            micro_nutrients_vec.pop().unwrap(),
            grams2,
        ));
        let prod3 = Box::new(Product::new(
            "Carrot".to_string(),
            None,
            macro_elements_vec.pop().unwrap(),
            micro_nutrients_vec.pop().unwrap(),
            grams3,
        ));
        let prod4 = Box::new(Product::new(
            "Carrot".to_string(),
            None,
            macro_elements_vec.pop().unwrap(),
            micro_nutrients_vec.pop().unwrap(),
            grams4,
        ));

        let pc1 = ProductConstraint::new(prod1, Some(1), Some(2), CommonUnits::Cup);
        let pc2 = ProductConstraint::new(prod2, Some(2), Some(3), CommonUnits::Tablespoon);
        let pc3 = ProductConstraint::new(prod3, Some(3), Some(4), CommonUnits::Custom);
        let pc4 = ProductConstraint::new(prod4, Some(3), Some(4), CommonUnits::Piece);
        let mut products = Vec::new();
        products.push(pc1.unwrap());
        products.push(pc2.unwrap());
        assert!(pc3.is_none());
        products.push(pc4.unwrap());

        let mut nutrients = Vec::new();
        nutrients
            .push(NutrientConstraint::new(MacroElemType::Protein, Some(10.0), Some(20.0)).unwrap());
        nutrients.push(NutrientConstraint::new(MacroElemType::Fat, Some(5.0), Some(15.0)).unwrap());
        nutrients
            .push(NutrientConstraint::new(MacroElemType::Carbs, Some(30.0), Some(60.0)).unwrap());

        MealConstraint {
            products: products,
            nutrients: nutrients,
        }
    }

    #[test]
    fn test_meal_add_remove_middle_product() {
        let mut meal = initialize_meal_with_products();
        // Remove from middle (second product)
        let removed = meal.products.remove(1);
        assert!(removed.food().name() == "Banana");
        assert!(meal.products[1].food().name() == "Carrot");
    }

    #[test]
    fn test_meal_add_remove_middle_nutrient() {
        let mut meal = initialize_meal_with_products();
        // Remove from middle (second nutrient)
        let removed = meal.nutrients.remove(1);
        assert_eq!(removed.element(), NutrientType::Macro(MacroElemType::Fat));
        assert_eq!(
            meal.nutrients[1].element(),
            NutrientType::Macro(MacroElemType::Carbs)
        );
    }

    #[test]
    fn test_meal_modify_middle_nutrient() {
        let mut meal = initialize_meal_with_products();
        // Modify middle nutrient (second nutrient)
        if let Some(nutrient) = meal.nutrients.get_mut(1) {
            nutrient.update(
                NutrientConstraint::new(nutrient.element(), Some(7.0), Some(17.0)).unwrap(),
            );
        }
        let modified = &meal.nutrients[1];
        assert_eq!(modified.min(), Some(7.0));
        assert_eq!(modified.max(), Some(17.0));
    }

    #[test]
    fn test_meal_modify_middle_product() {
        let mut meal = initialize_meal_with_products();
        // Modify middle product (second product)
        if let Some(product_constraint) = meal.products.get_mut(1) {
            product_constraint.update(
                ProductConstraint::new(
                    Box::new(product_constraint.food().clone()),
                    Some(4),
                    Some(5),
                    product_constraint.unit(),
                )
                .unwrap(),
            );
        }
        let modified = &meal.products[1];
        assert_eq!(modified.low_bound(), Some(4));
        assert_eq!(modified.up_bound(), Some(5));
    }
}
