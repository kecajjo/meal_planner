use core::fmt;
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use strum_macros::{EnumCount, EnumIter};

use super::{
    macro_elements::MacroElements, macro_elements::MacroElementsType,
    micro_nutrients::MicroNutrients, micro_nutrients::MicroNutrientsType,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NutrientType {
    Macro(MacroElementsType),
    Micro(MicroNutrientsType),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, EnumIter, EnumCount, Serialize, Deserialize)]
pub enum AllowedUnitsType {
    Gram,
    Piece,
    Cup,
    Tablespoon,
    Teaspoon,
    Box,
    Custom,
}

impl fmt::Display for AllowedUnitsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit_str = match self {
            AllowedUnitsType::Gram => "gram",
            AllowedUnitsType::Piece => "piece",
            AllowedUnitsType::Cup => "cup",
            AllowedUnitsType::Tablespoon => "tablespoon",
            AllowedUnitsType::Teaspoon => "teaspoon",
            AllowedUnitsType::Box => "box",
            AllowedUnitsType::Custom => "custom",
        };
        write!(f, "{unit_str}")
    }
}

const DEFAULT_ALLOWED_UNITS: (AllowedUnitsType, UnitData) = (
    AllowedUnitsType::Gram,
    UnitData {
        amount: 1,
        divider: 1,
    },
);

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash, Serialize, Deserialize)]
pub struct UnitData {
    pub amount: u16,
    pub divider: u16,
}

pub type AllowedUnits = std::collections::HashMap<AllowedUnitsType, UnitData>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    name: String,
    brand: Option<String>,
    pub macro_elements: Box<MacroElements>,
    pub micro_nutrients: Box<MicroNutrients>,
    pub allowed_units: AllowedUnits,
}

impl Product {
    #[must_use]
    pub fn new(
        name: String,
        brand: Option<String>,
        macro_elements: Box<MacroElements>,
        micro_nutrients: Box<MicroNutrients>,
        mut allowed_units: AllowedUnits,
    ) -> Self {
        allowed_units.insert(DEFAULT_ALLOWED_UNITS.0, DEFAULT_ALLOWED_UNITS.1);
        Self {
            name,
            brand,
            macro_elements,
            micro_nutrients,
            allowed_units,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn brand(&self) -> Option<&str> {
        self.brand.as_deref()
    }

    #[must_use]
    pub fn id(&self) -> String {
        match &self.brand {
            Some(brand) if !brand.is_empty() => format!("{} ({})", self.name(), brand),
            _ => self.name.clone(),
        }
    }

    #[must_use]
    pub fn get_nutrient_amount(&self, nutrient: NutrientType) -> Option<f32> {
        match nutrient {
            NutrientType::Macro(macro_type) => Some(self.macro_elements[macro_type]),
            NutrientType::Micro(micro_type) => self.micro_nutrients[micro_type],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::{MacroElementsType, MicroNutrientsType};
    use approx::assert_relative_eq;

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
                allowed_units.insert(
                    AllowedUnitsType::Gram,
                    UnitData {
                        amount: 123,
                        divider: 1,
                    },
                );
                allowed_units
            },
        );
        assert_eq!(product.name(), "TestName");
        assert_eq!(product.brand(), Some("TestBrand"));
        assert_relative_eq!(product.macro_elements[MacroElementsType::Fat], 1.0);
        assert_eq!(product.micro_nutrients[MicroNutrientsType::Fiber], None);
        let mut expected_allowed_units = std::collections::HashMap::new();
        expected_allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 123,
                divider: 1,
            },
        );
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
                allowed_units.insert(
                    AllowedUnitsType::Gram,
                    UnitData {
                        amount: 100,
                        divider: 1,
                    },
                );
                allowed_units
            },
        };
        assert_eq!(product.name, "Test Product");
        assert_eq!(product.brand.as_deref(), Some("Test Brand"));
        assert_relative_eq!(product.macro_elements[MacroElementsType::Fat], 1.0);
        assert_eq!(
            product.micro_nutrients[MicroNutrientsType::Fiber],
            Some(2.5)
        );
        assert_eq!(product.micro_nutrients[MicroNutrientsType::Zinc], None);
        let mut expected_allowed_units = std::collections::HashMap::new();
        expected_allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 100,
                divider: 1,
            },
        );
        assert_eq!(product.allowed_units, expected_allowed_units);
    }

    #[test]
    fn test_product_default_allowed_units() {
        let macro_elements = Box::new(MacroElements::new(0.0, 0.0, 0.0, 0.0, 0.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let product = Product::new(
            "NoUnits".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            std::collections::HashMap::new(),
        );
        let mut expected_allowed_units = std::collections::HashMap::new();
        expected_allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        assert_eq!(product.allowed_units, expected_allowed_units);
        assert_eq!(product.brand(), None);
    }

    #[test]
    fn test_common_units_display() {
        assert_eq!(AllowedUnitsType::Gram.to_string(), "gram");
        assert_eq!(AllowedUnitsType::Cup.to_string(), "cup");
        assert_eq!(AllowedUnitsType::Tablespoon.to_string(), "tablespoon");
        assert_eq!(AllowedUnitsType::Teaspoon.to_string(), "teaspoon");
        assert_eq!(AllowedUnitsType::Box.to_string(), "box");
        assert_eq!(AllowedUnitsType::Custom.to_string(), "custom");
    }

    #[test]
    fn test_product_id_with_brand() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let product = Product::new(
            "Apple".to_string(),
            Some("FreshFarms".to_string()),
            macro_elements,
            micro_nutrients,
            std::collections::HashMap::new(),
        );
        assert_eq!(product.id(), "Apple (FreshFarms)");
    }

    #[test]
    fn test_product_id_without_brand() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let product = Product::new(
            "Banana".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            std::collections::HashMap::new(),
        );
        assert_eq!(product.id(), "Banana");
    }

    #[test]
    fn test_allowed_units_empty_inserts_default() {
        let macro_elements = Box::new(MacroElements::new(0.0, 0.0, 0.0, 0.0, 0.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let product = Product::new(
            "DefaultUnit".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            std::collections::HashMap::new(),
        );
        assert_eq!(
            product.allowed_units.get(&AllowedUnitsType::Gram),
            Some(&UnitData {
                amount: 1,
                divider: 1
            })
        );
        assert_eq!(product.allowed_units.len(), 1);
    }

    #[test]
    fn test_allowed_units_multiple_units() {
        let macro_elements = Box::new(MacroElements::new(0.0, 0.0, 0.0, 0.0, 0.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 2,
                divider: 1,
            },
        );
        allowed_units.insert(
            AllowedUnitsType::Box,
            UnitData {
                amount: 5,
                divider: 1,
            },
        );
        let product = Product::new(
            "MultiUnit".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units.clone(),
        );
        assert_eq!(product.allowed_units, allowed_units);
    }
}
