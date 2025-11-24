use std::hash::Hash;

use super::{macro_elements::*, micro_nutrients::*};

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
}
