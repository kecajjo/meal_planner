use core::fmt;
use std::{
    hash::Hash,
    ops::{Add, Index, IndexMut},
};
use strum::IntoEnumIterator;
use strum_macros::{EnumCount, EnumIter};

#[derive(EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug, EnumCount)]
pub enum MicroNutrientsType {
    Fiber,
    Zinc,
    Sodium,
    Alcohol,
}

impl fmt::Display for MicroNutrientsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            MicroNutrientsType::Fiber => "Fiber",
            MicroNutrientsType::Zinc => "Zinc",
            MicroNutrientsType::Sodium => "Sodium",
            MicroNutrientsType::Alcohol => "Alcohol",
        };
        write!(f, "{}", name)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_micro_nutrients_default() {
        let mn = MicroNutrients::default();
        assert_eq!(mn[MicroNutrientsType::Fiber], None);
        assert_eq!(mn[MicroNutrientsType::Zinc], None);
        assert_eq!(mn[MicroNutrientsType::Sodium], None);
        assert_eq!(mn[MicroNutrientsType::Alcohol], None);
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

    #[test]
    fn test_micro_nutrients_type_display() {
        assert_eq!(MicroNutrientsType::Fiber.to_string(), "Fiber");
        assert_eq!(MicroNutrientsType::Zinc.to_string(), "Zinc");
        assert_eq!(MicroNutrientsType::Sodium.to_string(), "Sodium");
        assert_eq!(MicroNutrientsType::Alcohol.to_string(), "Alcohol");
    }

    #[test]
    fn test_micro_nutrients_partial_eq() {
        let mut mn1 = MicroNutrients::default();
        let mut mn2 = MicroNutrients::default();
        assert_eq!(mn1, mn2);

        mn1[MicroNutrientsType::Fiber] = Some(1.0);
        assert_ne!(mn1, mn2);

        mn2[MicroNutrientsType::Fiber] = Some(1.0);
        assert_eq!(mn1, mn2);
    }

    #[test]
    fn test_micro_nutrients_clone() {
        let mut mn1 = MicroNutrients::default();
        mn1[MicroNutrientsType::Sodium] = Some(0.7);
        let mn2 = mn1.clone();
        assert_eq!(mn1, mn2);
        assert_eq!(mn2[MicroNutrientsType::Sodium], Some(0.7));
    }

    #[test]
    fn test_micro_nutrients_add_none_values() {
        let mut mn1 = MicroNutrients::default();
        mn1[MicroNutrientsType::Fiber] = None;
        let mut mn2 = MicroNutrients::default();
        mn2[MicroNutrientsType::Fiber] = None;
        let sum = &mn1 + &mn2;
        assert_eq!(sum[MicroNutrientsType::Fiber], None);
    }

    #[test]
    fn test_micro_nutrients_add_mixed_some_none() {
        let mut mn1 = MicroNutrients::default();
        mn1[MicroNutrientsType::Alcohol] = Some(2.0);
        let mn2 = MicroNutrients::default();
        let sum = &mn1 + &mn2;
        assert_eq!(sum[MicroNutrientsType::Alcohol], Some(2.0));
    }
}
