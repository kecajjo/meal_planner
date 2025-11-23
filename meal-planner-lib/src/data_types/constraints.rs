use crate::data_types::CommonUnits;

pub use super::product_data_types::{MacroElemType, MicroNutrientsType, Product};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NutrientType {
    Macro(MacroElemType),
    Micro(MicroNutrientsType),
}

// Constraint on a nutritional element (macro or micro)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutrientConstraint {
    element: NutrientType,
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl From<MacroElemType> for NutrientType {
    fn from(value: MacroElemType) -> Self {
        NutrientType::Macro(value)
    }
}

impl From<MicroNutrientsType> for NutrientType {
    fn from(value: MicroNutrientsType) -> Self {
        NutrientType::Micro(value)
    }
}

impl NutrientConstraint {
    pub fn new<E>(element: E, min: Option<f32>, max: Option<f32>) -> Self
    where
        E: Into<NutrientType>,
    {
        Self {
            element: element.into(),
            min,
            max,
        }
    }

    pub fn element(&self) -> NutrientType {
        self.element
    }
}

// Constraint on a product (food item)
#[derive(Debug)]
pub struct ProductConstraint {
    food: Box<Product>,
    low_bound: Option<u16>,
    up_bound: Option<u16>,
    unit: CommonUnits,
}

impl ProductConstraint {
    pub fn new(
        food: Box<Product>,
        low_bound: Option<u16>,
        up_bound: Option<u16>,
        unit: CommonUnits,
    ) -> Option<Self> {
        if !food.grams_per_unit.contains_key(&unit) {
            return None;
        }
        if let (Some(lb), Some(ub)) = (low_bound, up_bound) {
            if lb > ub {
                return None;
            }
        }
        Some(Self {
            food,
            low_bound,
            up_bound,
            unit,
        })
    }

    pub fn food(&self) -> &Product {
        &self.food
    }
    pub fn low_bound(&self) -> Option<u16> {
        self.low_bound
    }
    pub fn up_bound(&self) -> Option<u16> {
        self.up_bound
    }
    pub fn unit(&self) -> CommonUnits {
        self.unit
    }
}

pub struct MealConstraint {
    pub products: Vec<ProductConstraint>,
    pub nutrients: Vec<NutrientConstraint>,
}

pub struct DayMealPlanConstraint {
    pub meals: HashMap<String, MealConstraint>,
    pub nutrients: Vec<NutrientConstraint>,
}

#[cfg(test)]
mod tests {
    use super::super::product_data_types::{MacroElements, MicroNutrients};
    use super::*;
    // MacroElements and MicroNutrients are now available from super

    #[test]
    fn test_nutrient_constraint_constructor_macro() {
        let constraint = NutrientConstraint::new(MacroElemType::Protein, Some(10.0), Some(50.0));
        match constraint.element {
            NutrientType::Macro(MacroElemType::Protein) => {}
            _ => panic!("Expected MacroElemType::Protein"),
        }
        assert_eq!(constraint.min, Some(10.0));
        assert_eq!(constraint.max, Some(50.0));
    }

    #[test]
    fn test_nutrient_constraint_constructor_micro() {
        let constraint = NutrientConstraint::new(MicroNutrientsType::Zinc, None, Some(18.0));
        match constraint.element {
            NutrientType::Micro(MicroNutrientsType::Zinc) => {}
            _ => panic!("Expected MicroNutrientsType::Salt"),
        }
        assert_eq!(constraint.min, None);
        assert_eq!(constraint.max, Some(18.0));
    }

    #[test]
    fn test_nutrient_constraint_no_bounds() {
        let constraint = NutrientConstraint::new(MacroElemType::Fat, None, None);
        assert_eq!(constraint.min, None);
        assert_eq!(constraint.max, None);
    }

    #[test]
    fn test_nutrient_constraint_element_accessor() {
        let constraint = NutrientConstraint::new(MicroNutrientsType::Alcohol, Some(0.0), None);
        assert_eq!(
            constraint.element(),
            NutrientType::Micro(MicroNutrientsType::Alcohol)
        );
    }

    // --- Product Constraint tests ---
    #[test]
    fn test_product_constraint_valid_creation() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut grams_per_unit = std::collections::HashMap::new();
        grams_per_unit.insert(CommonUnits::Cup, 250);
        grams_per_unit.insert(CommonUnits::Piece, 1);
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            grams_per_unit,
        ));
        let constraint = ProductConstraint::new(product, Some(1), Some(5), CommonUnits::Cup);
        assert!(constraint.is_some());
    }

    #[test]
    fn test_product_constraint_invalid_unit() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let grams_per_unit = std::collections::HashMap::new(); // No units defined
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            grams_per_unit,
        ));
        let constraint = ProductConstraint::new(product, Some(1), Some(5), CommonUnits::Cup);
        assert!(constraint.is_none());
    }

    #[test]
    fn test_product_constraint_invalid_bounds() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut grams_per_unit = std::collections::HashMap::new();
        grams_per_unit.insert(CommonUnits::Cup, 250);
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            grams_per_unit,
        ));
        let constraint = ProductConstraint::new(product, Some(10), Some(5), CommonUnits::Cup);
        assert!(constraint.is_none());
    }

    // --- Meal tests ---
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
        nutrients.push(NutrientConstraint::new(
            MacroElemType::Protein,
            Some(10.0),
            Some(20.0),
        ));
        nutrients.push(NutrientConstraint::new(
            MacroElemType::Fat,
            Some(5.0),
            Some(15.0),
        ));
        nutrients.push(NutrientConstraint::new(
            MacroElemType::Carbs,
            Some(30.0),
            Some(60.0),
        ));

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
        assert!(removed.food.name() == "Banana");
        assert!(meal.products[1].food.name() == "Carrot");
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
            nutrient.min = Some(7.0);
            nutrient.max = Some(17.0);
        }
        let modified = &meal.nutrients[1];
        assert_eq!(modified.min, Some(7.0));
        assert_eq!(modified.max, Some(17.0));
    }

    #[test]
    fn test_meal_modify_middle_product() {
        let mut meal = initialize_meal_with_products();
        // Modify middle product (second product)
        if let Some(product_constraint) = meal.products.get_mut(1) {
            product_constraint.low_bound = Some(4);
            product_constraint.up_bound = Some(5);
        }
        let modified = &meal.products[1];
        assert_eq!(modified.low_bound, Some(4));
        assert_eq!(modified.up_bound, Some(5));
    }

    // --- DayMealPlan tests ---
    fn init_day_plan() -> DayMealPlanConstraint {
        let breakfast = MealConstraint {
            products: Vec::new(),
            nutrients: Vec::new(),
        };
        let lunch = MealConstraint {
            products: Vec::new(),
            nutrients: Vec::new(),
        };
        let dinner = MealConstraint {
            products: Vec::new(),
            nutrients: Vec::new(),
        };
        let mut meals = HashMap::new();
        meals.insert("breakfast".to_string(), breakfast);
        meals.insert("lunch".to_string(), lunch);
        meals.insert("dinner".to_string(), dinner);
        DayMealPlanConstraint {
            meals,
            nutrients: Vec::new(),
        }
    }

    #[test]
    fn test_day_meal_plan_add_remove_middle_meal() {
        let mut plan = init_day_plan();
        // Remove from middle (lunch)
        let keys: Vec<_> = plan.meals.keys().cloned().collect();
        let removed = plan.meals.remove(&keys[1]);
        assert!(removed.is_some());
        // Insert back in the middle
        plan.meals.insert(
            keys[1].clone(),
            MealConstraint {
                products: Vec::new(),
                nutrients: Vec::new(),
            },
        );
        assert!(plan.meals.contains_key(&keys[1]));
    }
}
