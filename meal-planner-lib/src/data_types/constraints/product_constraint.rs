use crate::data_types::{AllowedUnitsType, Product};

pub type AllowedUnitDividers = std::collections::HashMap<AllowedUnitsType, u16>;

// Constraint on a product (food item)
#[derive(Debug)]
pub struct ProductConstraint {
    food: Box<Product>,
    low_bound: Option<u16>,
    up_bound: Option<u16>,
    unit: AllowedUnitsType,
}

impl ProductConstraint {
    #[must_use]
    pub fn new(
        food: Box<Product>,
        low_bound: Option<u16>,
        up_bound: Option<u16>,
        unit: AllowedUnitsType,
    ) -> Option<Self> {
        if !food.allowed_units.contains_key(&unit) {
            return None;
        }
        if let (Some(lb), Some(ub)) = (low_bound, up_bound)
            && lb > ub
        {
            return None;
        }
        Some(Self {
            food,
            low_bound,
            up_bound,
            unit,
        })
    }

    #[must_use]
    pub fn food(&self) -> &Product {
        &self.food
    }
    #[must_use]
    pub fn low_bound(&self) -> Option<u16> {
        self.low_bound
    }
    #[must_use]
    pub fn up_bound(&self) -> Option<u16> {
        self.up_bound
    }
    #[must_use]
    pub fn unit(&self) -> AllowedUnitsType {
        self.unit
    }
    pub fn update(&mut self, other: ProductConstraint) {
        *self = other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::{MacroElements, MicroNutrients, UnitData};

    #[test]
    fn test_product_constraint_valid_creation() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        ));
        let constraint = ProductConstraint::new(product, Some(1), Some(5), AllowedUnitsType::Cup);
        assert!(constraint.is_some());
    }

    #[test]
    fn test_product_constraint_invalid_unit() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let allowed_units = std::collections::HashMap::new(); // No units defined
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        ));
        let constraint = ProductConstraint::new(product, Some(1), Some(5), AllowedUnitsType::Cup);
        assert!(constraint.is_none());
    }

    #[test]
    fn test_product_constraint_invalid_bounds() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        ));
        let constraint = ProductConstraint::new(product, Some(10), Some(5), AllowedUnitsType::Cup);
        assert!(constraint.is_none());
    }

    #[test]
    fn test_product_constraint_update() {
        let macro_elements1 = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients1 = Box::new(MicroNutrients::default());
        let mut allowed_units1 = std::collections::HashMap::new();
        allowed_units1.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        allowed_units1.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        let product1 = Box::new(Product::new(
            "Product 1".to_string(),
            None,
            macro_elements1,
            micro_nutrients1,
            allowed_units1,
        ));
        let mut constraint1 =
            ProductConstraint::new(product1, Some(1), Some(5), AllowedUnitsType::Cup).unwrap();

        let macro_elements2 = Box::new(MacroElements::new(10.0, 20.0, 30.0, 40.0, 50.0));
        let micro_nutrients2 = Box::new(MicroNutrients::default());
        let mut allowed_units2 = std::collections::HashMap::new();
        allowed_units2.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        let product2 = Box::new(Product::new(
            "Product 2".to_string(),
            None,
            macro_elements2,
            micro_nutrients2,
            allowed_units2,
        ));
        let constraint2 =
            ProductConstraint::new(product2, Some(2), Some(10), AllowedUnitsType::Gram).unwrap();

        constraint1.update(constraint2);

        assert_eq!(constraint1.food().name(), "Product 2");
        assert_eq!(constraint1.low_bound(), Some(2));
        assert_eq!(constraint1.up_bound(), Some(10));
        assert_eq!(constraint1.unit(), AllowedUnitsType::Gram);
    }

    #[test]
    fn test_product_constraint_no_bounds() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        ));
        let constraint = ProductConstraint::new(product, None, None, AllowedUnitsType::Cup);
        assert!(constraint.is_some());
        let constraint = constraint.unwrap();
        assert_eq!(constraint.low_bound(), None);
        assert_eq!(constraint.up_bound(), None);
    }

    #[test]
    fn test_product_constraint_equal_bounds() {
        let macro_elements = Box::new(MacroElements::new(1.0, 2.0, 3.0, 4.0, 5.0));
        let micro_nutrients = Box::new(MicroNutrients::default());
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Cup,
            UnitData {
                amount: 250,
                divider: 1,
            },
        );
        let product = Box::new(Product::new(
            "Test Product".to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        ));
        let constraint = ProductConstraint::new(product, Some(3), Some(3), AllowedUnitsType::Cup);
        assert!(constraint.is_some());
        let constraint = constraint.unwrap();
        assert_eq!(constraint.low_bound(), Some(3));
        assert_eq!(constraint.up_bound(), Some(3));
    }
}
