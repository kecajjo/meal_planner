use crate::data_types::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NutrientType {
    Macro(MacroElementsType),
    Micro(MicroNutrientsType),
}

// Constraint on a nutritional element (macro or micro)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NutrientConstraint {
    element: NutrientType,
    min: Option<f32>,
    max: Option<f32>,
}

impl From<MacroElementsType> for NutrientType {
    fn from(value: MacroElementsType) -> Self {
        NutrientType::Macro(value)
    }
}

impl From<MicroNutrientsType> for NutrientType {
    fn from(value: MicroNutrientsType) -> Self {
        NutrientType::Micro(value)
    }
}

impl NutrientConstraint {
    pub fn new<E>(element: E, min: Option<f32>, max: Option<f32>) -> Option<Self>
    where
        E: Into<NutrientType>,
    {
        if let Some(min_val) = min {
            if min_val < 0.0 {
                return None;
            }
        }
        if let Some(max_val) = max {
            if max_val < 0.0 {
                return None;
            }
        }

        if let (Some(min_val), Some(max_val)) = (min, max) {
            if min_val > max_val {
                return None;
            }
        }

        Some(Self {
            element: element.into(),
            min,
            max,
        })
    }

    pub fn element(&self) -> NutrientType {
        self.element
    }

    pub fn min(&self) -> Option<f32> {
        self.min
    }

    pub fn max(&self) -> Option<f32> {
        self.max
    }

    pub fn update(&mut self, other: NutrientConstraint) {
        *self = other;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nutrient_constraint_constructor_macro() {
        let constraint =
            NutrientConstraint::new(MacroElementsType::Protein, Some(10.0), Some(50.0)).unwrap();
        match constraint.element() {
            NutrientType::Macro(MacroElementsType::Protein) => {}
            _ => panic!("Expected MacroElemType::Protein"),
        }
        assert_eq!(constraint.min(), Some(10.0));
        assert_eq!(constraint.max(), Some(50.0));
    }

    #[test]
    fn test_nutrient_constraint_constructor_micro() {
        let constraint =
            NutrientConstraint::new(MicroNutrientsType::Zinc, None, Some(18.0)).unwrap();
        match constraint.element() {
            NutrientType::Micro(MicroNutrientsType::Zinc) => {}
            _ => panic!("Expected MicroNutrientsType::Salt"),
        }
        assert_eq!(constraint.min(), None);
        assert_eq!(constraint.max(), Some(18.0));
    }

    #[test]
    fn test_nutrient_constraint_no_bounds() {
        let constraint = NutrientConstraint::new(MacroElementsType::Fat, None, None).unwrap();
        assert_eq!(constraint.min(), None);
        assert_eq!(constraint.max(), None);
    }

    #[test]
    fn test_nutrient_constraint_element_accessor() {
        let constraint =
            NutrientConstraint::new(MicroNutrientsType::Alcohol, Some(0.0), None).unwrap();
        assert_eq!(
            constraint.element(),
            NutrientType::Micro(MicroNutrientsType::Alcohol)
        );
    }
    #[test]
    fn test_nutrient_constraint_negative_min() {
        let constraint = NutrientConstraint::new(MacroElementsType::Carbs, Some(-1.0), Some(10.0));
        assert!(
            constraint.is_none(),
            "Constraint should not allow negative min"
        );
    }

    #[test]
    fn test_nutrient_constraint_negative_max() {
        let constraint = NutrientConstraint::new(MicroNutrientsType::Zinc, Some(0.0), Some(-5.0));
        assert!(
            constraint.is_none(),
            "Constraint should not allow negative max"
        );
    }

    #[test]
    fn test_nutrient_constraint_min_greater_than_max() {
        let constraint = NutrientConstraint::new(MacroElementsType::Fat, Some(20.0), Some(10.0));
        assert!(
            constraint.is_none(),
            "Constraint should not allow min > max"
        );
    }

    #[test]
    fn test_nutrient_constraint_min_equal_max() {
        let constraint = NutrientConstraint::new(MacroElementsType::Fat, Some(10.0), Some(10.0));
        assert!(constraint.is_some(), "Constraint should allow min == max");
        let constraint = constraint.unwrap();
        assert_eq!(constraint.min(), Some(10.0));
        assert_eq!(constraint.max(), Some(10.0));
    }

    #[test]
    fn test_nutrient_constraint_zero_min_and_max() {
        let constraint = NutrientConstraint::new(MicroNutrientsType::Fiber, Some(0.0), Some(0.0));
        assert!(
            constraint.is_some(),
            "Constraint should allow zero min and max"
        );
        let constraint = constraint.unwrap();
        assert_eq!(constraint.min(), Some(0.0));
        assert_eq!(constraint.max(), Some(0.0));
    }

    #[test]
    fn test_nutrient_constraint_update_macro() {
        let mut constraint =
            NutrientConstraint::new(MacroElementsType::Protein, Some(10.0), Some(50.0)).unwrap();
        let new_constraint =
            NutrientConstraint::new(MacroElementsType::Fat, Some(5.0), Some(20.0)).unwrap();
        constraint.update(new_constraint);
        assert_eq!(
            constraint.element(),
            NutrientType::Macro(MacroElementsType::Fat)
        );
        assert_eq!(constraint.min(), Some(5.0));
        assert_eq!(constraint.max(), Some(20.0));
    }

    #[test]
    fn test_nutrient_constraint_update_micro() {
        let mut constraint =
            NutrientConstraint::new(MicroNutrientsType::Zinc, Some(2.0), Some(10.0)).unwrap();
        let new_constraint =
            NutrientConstraint::new(MicroNutrientsType::Alcohol, None, Some(5.0)).unwrap();
        constraint.update(new_constraint);
        assert_eq!(
            constraint.element(),
            NutrientType::Micro(MicroNutrientsType::Alcohol)
        );
        assert_eq!(constraint.min(), None);
        assert_eq!(constraint.max(), Some(5.0));
    }

    #[test]
    fn test_nutrient_constraint_update_to_no_bounds() {
        let mut constraint =
            NutrientConstraint::new(MacroElementsType::Carbs, Some(1.0), Some(10.0)).unwrap();
        let new_constraint = NutrientConstraint::new(MacroElementsType::Carbs, None, None).unwrap();
        constraint.update(new_constraint);
        assert_eq!(constraint.min(), None);
        assert_eq!(constraint.max(), None);
    }

    #[test]
    fn test_nutrient_constraint_update_to_same() {
        let mut constraint =
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(3.0), Some(8.0)).unwrap();
        let new_constraint =
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(3.0), Some(8.0)).unwrap();
        constraint.update(new_constraint);
        assert_eq!(
            constraint.element(),
            NutrientType::Micro(MicroNutrientsType::Fiber)
        );
        assert_eq!(constraint.min(), Some(3.0));
        assert_eq!(constraint.max(), Some(8.0));
    }

    #[test]
    fn test_nutrient_constraint_update_macro_to_micro() {
        let mut constraint =
            NutrientConstraint::new(MacroElementsType::Fat, Some(2.0), Some(6.0)).unwrap();
        let new_constraint =
            NutrientConstraint::new(MicroNutrientsType::Zinc, Some(1.0), Some(3.0)).unwrap();
        constraint.update(new_constraint);
        assert_eq!(
            constraint.element(),
            NutrientType::Micro(MicroNutrientsType::Zinc)
        );
        assert_eq!(constraint.min(), Some(1.0));
        assert_eq!(constraint.max(), Some(3.0));
    }
}
