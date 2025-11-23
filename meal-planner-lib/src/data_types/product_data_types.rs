use std::{
    hash::Hash,
    ops::{Add, Index, IndexMut},
};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum MacroElemType {
    Fat,
    SaturatedFat,
    Carbs,
    Sugar,
    Protein,
    Calories,
}

// Macro elements per 100g
#[derive(Debug, Clone, PartialEq)]
pub struct MacroElements {
    elements: std::collections::HashMap<MacroElemType, f32>,
}

impl MacroElements {
    pub fn new(fat: f32, saturated_fat: f32, carbs: f32, sugar: f32, protein: f32) -> Self {
        let mut elements = std::collections::HashMap::new();
        for elem in MacroElemType::iter() {
            let value = match elem {
                MacroElemType::Fat => fat,
                MacroElemType::SaturatedFat => saturated_fat,
                MacroElemType::Carbs => carbs,
                MacroElemType::Sugar => sugar,
                MacroElemType::Protein => protein,
                _ => 0.0,
            };
            elements.insert(elem, value);
        }
        let mut me = Self { elements };
        me.recompute_calories();
        me
    }

    fn recompute_calories(&mut self) {
        let fat = *self.elements.get(&MacroElemType::Fat).unwrap_or(&0.0);
        let carbs = *self.elements.get(&MacroElemType::Carbs).unwrap_or(&0.0);
        let protein = *self.elements.get(&MacroElemType::Protein).unwrap_or(&0.0);
        let calories = (fat * 9.0) + (carbs * 4.0) + (protein * 4.0);
        self.elements.insert(MacroElemType::Calories, calories);
    }

    pub fn set(&mut self, key: MacroElemType, value: f32) {
        match key {
            MacroElemType::Calories => panic!("Cannot set calories directly"),
            _ => {
                self.elements.insert(key, value);
            }
        }
        self.recompute_calories();
    }
}

impl Index<MacroElemType> for MacroElements {
    type Output = f32;

    fn index(&self, key: MacroElemType) -> &Self::Output {
        self.elements.get(&key).expect("Macro element not found")
    }
}

impl<'a, 'b> Add<&'b MacroElements> for &'a MacroElements {
    type Output = MacroElements;

    fn add(self, rhs: &'b MacroElements) -> MacroElements {
        MacroElements::add_ref(self, rhs)
    }
}

impl MacroElements {
    pub fn add_ref(lhs: &MacroElements, rhs: &MacroElements) -> MacroElements {
        let mut elements = std::collections::HashMap::new();
        for elem in MacroElemType::iter() {
            let value = lhs[elem] + rhs[elem];
            elements.insert(elem, value);
        }
        let mut result = MacroElements { elements };
        result.recompute_calories();
        result
    }
}

#[derive(EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug)]
pub enum MicroNutrientsType {
    Fiber,
    Zinc,
    Sodium,
    Alcohol,
}

// Micro nutrients per 100g
#[derive(Debug, Clone, PartialEq)]
pub struct MicroNutrients {
    elements: std::collections::HashMap<MicroNutrientsType, Option<f32>>,
}

impl Default for MicroNutrients {
    fn default() -> Self {
        let elements = std::collections::HashMap::new();
        Self { elements }
    }
}

impl Index<MicroNutrientsType> for MicroNutrients {
    type Output = Option<f32>;

    fn index(&self, key: MicroNutrientsType) -> &Self::Output {
        self.elements.get(&key).unwrap_or(&None)
    }
}

impl IndexMut<MicroNutrientsType> for MicroNutrients {
    fn index_mut(&mut self, key: MicroNutrientsType) -> &mut Self::Output {
        self.elements.entry(key).or_insert(None)
    }
}

impl<'a, 'b> Add<&'b MicroNutrients> for &'a MicroNutrients {
    type Output = MicroNutrients;

    fn add(self, rhs: &'b MicroNutrients) -> MicroNutrients {
        let mut elements = std::collections::HashMap::new();
        for elem in MicroNutrientsType::iter() {
            let value = match (self.elements.get(&elem), rhs.elements.get(&elem)) {
                (Some(Some(v1)), Some(Some(v2))) => Some(v1 + v2),
                (Some(Some(v)), _) | (_, Some(Some(v))) => Some(*v),
                _ => None,
            };
            elements.insert(elem, value);
        }
        MicroNutrients { elements }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum CommonUnits {
    Piece,
    Cup,
    Tablespoon,
    Teaspoon,
    Box,
    Custom,
}

const DEFAULT_ALLOWED_UNITS: (CommonUnits, u16) = (CommonUnits::Piece, 1);
pub type AllowedUnits = std::collections::HashMap<CommonUnits, u16>;

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    name: String,
    brand: Option<String>,
    pub macro_elements: Box<MacroElements>,
    pub micro_nutrients: Box<MicroNutrients>,
    pub allowed_units: AllowedUnits,
}

impl Product {
    pub fn new(
        name: String,
        brand: Option<String>,
        macro_elements: Box<MacroElements>,
        micro_nutrients: Box<MicroNutrients>,
        mut allowed_units: AllowedUnits,
    ) -> Self {
        if allowed_units.is_empty() {
            allowed_units.insert(DEFAULT_ALLOWED_UNITS.0, DEFAULT_ALLOWED_UNITS.1);
        }
        Self {
            name,
            brand,
            macro_elements,
            micro_nutrients,
            allowed_units: allowed_units,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn brand(&self) -> Option<&str> {
        self.brand.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_new_and_accessors() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let product = Product::new(
            "TestName".to_string(),
            Some("TestBrand".to_string()),
            macro_elements,
            micro_nutrients,
            {
                let mut allowed_units = std::collections::HashMap::new();
                allowed_units.insert(CommonUnits::Piece, 123);
                allowed_units
            },
        );
        assert_eq!(product.name(), "TestName");
        assert_eq!(product.brand(), Some("TestBrand"));
        assert_eq!(product.macro_elements[MacroElemType::Fat], 1.0);
        assert_eq!(product.micro_nutrients[MicroNutrientsType::Fiber], None);
        let mut expected_allowed_units = std::collections::HashMap::new();
        expected_allowed_units.insert(CommonUnits::Piece, 123);
        assert_eq!(product.allowed_units, expected_allowed_units);
    }

    #[test]
    fn test_macro_elements_new_and_calories() {
        let me = MacroElements::new(10.0, 3.0, 20.0, 5.0, 15.0);
        // calories = (10*9) + (20*4) + (15*4) = 90 + 80 + 60 = 230
        assert_eq!(me[MacroElemType::Fat], 10.0);
        assert_eq!(me[MacroElemType::SaturatedFat], 3.0);
        assert_eq!(me[MacroElemType::Carbs], 20.0);
        assert_eq!(me[MacroElemType::Sugar], 5.0);
        assert_eq!(me[MacroElemType::Protein], 15.0);
        assert_eq!(me[MacroElemType::Calories], 230.0);
    }

    #[test]
    fn test_macro_elements_set_and_recompute_calories() {
        let mut me = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        me.set(MacroElemType::Fat, 5.0);
        me.set(MacroElemType::Carbs, 10.0);
        me.set(MacroElemType::Protein, 8.0);
        // calories = (5*9) + (10*4) + (8*4) = 45 + 40 + 32 = 117
        assert_eq!(me[MacroElemType::Fat], 5.0);
        assert_eq!(me[MacroElemType::Carbs], 10.0);
        assert_eq!(me[MacroElemType::Protein], 8.0);
        assert_eq!(me[MacroElemType::Calories], 117.0);
    }

    #[test]
    #[should_panic(expected = "Cannot set calories directly")]
    fn test_macro_elements_set_invalid_key() {
        let mut me = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        me.set(MacroElemType::Calories, 1.0);
    }

    #[test]
    fn test_macro_elements_index() {
        let me = MacroElements::new(2.0, 1.0, 4.0, 1.5, 3.0);
        assert_eq!(me[MacroElemType::Fat], 2.0);
        assert_eq!(me[MacroElemType::SaturatedFat], 1.0);
        assert_eq!(me[MacroElemType::Carbs], 4.0);
        assert_eq!(me[MacroElemType::Sugar], 1.5);
        assert_eq!(me[MacroElemType::Protein], 3.0);
        assert_eq!(me[MacroElemType::Calories], 46.0);
    }

    #[test]
    fn test_micro_nutrients_default() {
        let mn = MicroNutrients::default();
        assert_eq!(mn[MicroNutrientsType::Fiber], None);
        assert_eq!(mn[MicroNutrientsType::Zinc], None);
        assert_eq!(mn[MicroNutrientsType::Sodium], None);
        assert_eq!(mn[MicroNutrientsType::Alcohol], None);
    }

    #[test]
    fn test_product_creation() {
        let macro_elements = Box::new(MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0));
        let mut micro_nutrients = Box::new(MicroNutrients::default());
        micro_nutrients[MicroNutrientsType::Fiber] = Some(2.5);
        let product = Product {
            name: "Test Product".to_string(),
            brand: Some("Test Brand".to_string()),
            macro_elements,
            micro_nutrients,
            allowed_units: {
                let mut allowed_units = std::collections::HashMap::new();
                allowed_units.insert(CommonUnits::Piece, 100);
                allowed_units
            },
        };
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.brand.as_deref(), Some("Test Brand"));
        assert_eq!(product.macro_elements[MacroElemType::Fat], 1.0);
        assert_eq!(
            product.micro_nutrients[MicroNutrientsType::Fiber],
            Some(2.5)
        );
        assert_eq!(product.micro_nutrients[MicroNutrientsType::Zinc], None);
        let mut expected_allowed_units = std::collections::HashMap::new();
        expected_allowed_units.insert(CommonUnits::Piece, 100);
        assert_eq!(product.allowed_units, expected_allowed_units);
    }

    #[test]
    fn test_macro_elements_add() {
        let me1 = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        let me2 = MacroElements::new(2.0, 1.0, 4.0, 1.5, 1.0);
        let sum = &me1 + &me2;
        for elem in MacroElemType::iter() {
            if elem == MacroElemType::Calories {
                continue;
            }

            assert_eq!(sum[elem], (&me1)[elem] + (&me2)[elem]);
        }
        // calories = (3*9)+(6*4)+(4*4) = 27+24+16 = 67
        assert_eq!(sum[MacroElemType::Calories], 67.0);
    }

    #[test]
    fn test_micro_nutrients_add() {
        let mut mn1 = MicroNutrients::default();
        mn1[MicroNutrientsType::Fiber] = Some(1.0);
        mn1[MicroNutrientsType::Zinc] = Some(0.5);
        let mut mn2 = MicroNutrients::default();
        mn2[MicroNutrientsType::Fiber] = Some(2.0);
        mn2[MicroNutrientsType::Alcohol] = Some(0.1);
        let sum = &mn1 + &mn2;
        assert_eq!(sum[MicroNutrientsType::Fiber], Some(3.0));
        assert_eq!(sum[MicroNutrientsType::Zinc], Some(0.5));
        assert_eq!(sum[MicroNutrientsType::Alcohol], Some(0.1));
        assert_eq!(sum[MicroNutrientsType::Sodium], None);
    }

    #[test]
    fn test_micro_nutrients_index_mut() {
        let mut mn = MicroNutrients::default();
        assert_eq!(mn[MicroNutrientsType::Zinc], None);
        mn[MicroNutrientsType::Zinc] = Some(1.2);
        assert_eq!(mn[MicroNutrientsType::Zinc], Some(1.2));
        mn[MicroNutrientsType::Zinc] = None;
        assert_eq!(mn[MicroNutrientsType::Zinc], None);
    }
}
