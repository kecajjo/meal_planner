use core::fmt;
use std::{
    hash::Hash,
    ops::{Add, Index},
};
use strum::IntoEnumIterator;
use strum_macros::{EnumCount, EnumIter};

#[derive(EnumIter, PartialEq, Eq, Hash, Copy, Clone, Debug, EnumCount)]
pub enum MacroElementsType {
    Fat,
    SaturatedFat,
    Carbs,
    Sugar,
    Protein,
    Calories,
}

impl fmt::Display for MacroElementsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            MacroElementsType::Fat => "Fat",
            MacroElementsType::SaturatedFat => "Saturated Fat",
            MacroElementsType::Carbs => "Carbohydrates",
            MacroElementsType::Sugar => "Sugar",
            MacroElementsType::Protein => "Protein",
            MacroElementsType::Calories => "Calories",
        };
        write!(f, "{name}")
    }
}

/// Macro elements per 100g
#[derive(Debug, Clone, PartialEq)]
pub struct MacroElements {
    elements: std::collections::HashMap<MacroElementsType, f32>,
}

impl MacroElements {
    #[must_use]
    pub fn new(fat: f32, saturated_fat: f32, carbs: f32, sugar: f32, protein: f32) -> Self {
        let mut elements = std::collections::HashMap::new();
        for elem in MacroElementsType::iter() {
            let value = match elem {
                MacroElementsType::Fat => fat,
                MacroElementsType::SaturatedFat => saturated_fat,
                MacroElementsType::Carbs => carbs,
                MacroElementsType::Sugar => sugar,
                MacroElementsType::Protein => protein,
                _ => 0.0,
            };
            elements.insert(elem, value);
        }
        let mut me = Self { elements };
        me.recompute_calories();
        me
    }

    fn recompute_calories(&mut self) {
        let fat = *self.elements.get(&MacroElementsType::Fat).unwrap_or(&0.0);
        let carbs = *self.elements.get(&MacroElementsType::Carbs).unwrap_or(&0.0);
        let protein = *self
            .elements
            .get(&MacroElementsType::Protein)
            .unwrap_or(&0.0);
        let calories = (fat * 9.0) + (carbs * 4.0) + (protein * 4.0);
        self.elements.insert(MacroElementsType::Calories, calories);
    }

    pub fn set(&mut self, key: MacroElementsType, value: f32) -> Result<(), String> {
        if key == MacroElementsType::Calories {
            Err("Cannot set calories directly".to_string())
        } else {
            self.elements.insert(key, value);
            Ok(())
        }?;
        self.recompute_calories();
        Ok(())
    }

    #[must_use]
    pub fn add_ref(lhs: &MacroElements, rhs: &MacroElements) -> MacroElements {
        let mut elements = std::collections::HashMap::new();
        for elem in MacroElementsType::iter() {
            let value = lhs[elem] + rhs[elem];
            elements.insert(elem, value);
        }
        let mut result = MacroElements { elements };
        result.recompute_calories();
        result
    }
}

pub struct MacroElementsIter {
    inner: std::collections::hash_map::IntoIter<MacroElementsType, f32>,
}

impl IntoIterator for MacroElements {
    type Item = (MacroElementsType, f32);
    type IntoIter = MacroElementsIter;

    fn into_iter(self) -> Self::IntoIter {
        MacroElementsIter {
            inner: self.elements.clone().into_iter(),
        }
    }
}

impl Iterator for MacroElementsIter {
    type Item = (MacroElementsType, f32);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

impl Index<MacroElementsType> for MacroElements {
    type Output = f32;

    fn index(&self, key: MacroElementsType) -> &Self::Output {
        self.elements.get(&key).expect("Macro element not found")
    }
}

impl<'b> Add<&'b MacroElements> for &MacroElements {
    type Output = MacroElements;

    fn add(self, rhs: &'b MacroElements) -> MacroElements {
        MacroElements::add_ref(self, rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_elements_new_and_calories() {
        let me = MacroElements::new(10.0, 3.0, 20.0, 5.0, 15.0);
        // calories = (10*9) + (20*4) + (15*4) = 90 + 80 + 60 = 230
        assert_eq!(me[MacroElementsType::Fat], 10.0);
        assert_eq!(me[MacroElementsType::SaturatedFat], 3.0);
        assert_eq!(me[MacroElementsType::Carbs], 20.0);
        assert_eq!(me[MacroElementsType::Sugar], 5.0);
        assert_eq!(me[MacroElementsType::Protein], 15.0);
        assert_eq!(me[MacroElementsType::Calories], 230.0);
    }

    #[test]
    fn test_macro_elements_set_and_recompute_calories() {
        let mut me = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        assert!(me.set(MacroElementsType::Fat, 5.0).is_ok());
        assert!(me.set(MacroElementsType::Carbs, 10.0).is_ok());
        assert!(me.set(MacroElementsType::Protein, 8.0).is_ok());
        // calories = (5*9) + (10*4) + (8*4) = 45 + 40 + 32 = 117
        assert_eq!(me[MacroElementsType::Fat], 5.0);
        assert_eq!(me[MacroElementsType::Carbs], 10.0);
        assert_eq!(me[MacroElementsType::Protein], 8.0);
        assert_eq!(me[MacroElementsType::Calories], 117.0);
    }

    #[test]
    fn test_macro_elements_set_invalid_key() {
        let mut me = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        assert!(me.set(MacroElementsType::Calories, 1.0).is_err());
    }

    #[test]
    fn test_macro_elements_index() {
        let me = MacroElements::new(2.0, 1.0, 4.0, 1.5, 3.0);
        assert_eq!(me[MacroElementsType::Fat], 2.0);
        assert_eq!(me[MacroElementsType::SaturatedFat], 1.0);
        assert_eq!(me[MacroElementsType::Carbs], 4.0);
        assert_eq!(me[MacroElementsType::Sugar], 1.5);
        assert_eq!(me[MacroElementsType::Protein], 3.0);
        assert_eq!(me[MacroElementsType::Calories], 46.0);
    }

    #[test]
    fn test_macro_elements_add() {
        let me1 = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        let me2 = MacroElements::new(2.0, 1.0, 4.0, 1.5, 1.0);
        let sum = &me1 + &me2;
        for elem in MacroElementsType::iter() {
            if elem == MacroElementsType::Calories {
                continue;
            }

            assert_eq!(sum[elem], (&me1)[elem] + (&me2)[elem]);
        }
        // calories = (3*9)+(6*4)+(4*4) = 27+24+16 = 67
        assert_eq!(sum[MacroElementsType::Calories], 67.0);
    }

    #[test]
    fn test_macro_elem_type_display() {
        assert_eq!(MacroElementsType::Fat.to_string(), "Fat");
        assert_eq!(MacroElementsType::SaturatedFat.to_string(), "Saturated Fat");
        assert_eq!(MacroElementsType::Carbs.to_string(), "Carbohydrates");
        assert_eq!(MacroElementsType::Sugar.to_string(), "Sugar");
        assert_eq!(MacroElementsType::Protein.to_string(), "Protein");
        assert_eq!(MacroElementsType::Calories.to_string(), "Calories");
    }

    #[test]
    fn test_macro_elements_partial_eq() {
        let me1 = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        let me2 = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        let me3 = MacroElements::new(2.0, 1.0, 4.0, 1.5, 1.0);
        assert_eq!(me1, me2);
        assert_ne!(me1, me3);
    }

    #[test]
    fn test_macro_elements_clone() {
        let me1 = MacroElements::new(2.0, 1.0, 4.0, 1.5, 3.0);
        let me2 = me1.clone();
        assert_eq!(me1, me2);
    }

    #[test]
    fn test_macro_elements_add_ref() {
        let me1 = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        let me2 = MacroElements::new(2.0, 1.0, 4.0, 1.5, 1.0);
        let sum = MacroElements::add_ref(&me1, &me2);
        for elem in MacroElementsType::iter() {
            if elem == MacroElementsType::Calories {
                continue;
            }
            assert_eq!(sum[elem], me1[elem] + me2[elem]);
        }
        // calories = (3*9)+(6*4)+(4*4) = 27+24+16 = 67
        assert_eq!(sum[MacroElementsType::Calories], 67.0);
    }

    #[test]
    fn test_macro_elements_index_panic() {
        let me = MacroElements::new(1.0, 0.5, 2.0, 0.5, 3.0);
        // Remove an element to simulate missing key
        let mut elements = me.clone();
        elements.elements.remove(&MacroElementsType::Sugar);
        let result = std::panic::catch_unwind(|| {
            let _ = elements[MacroElementsType::Sugar];
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_macro_elements_into_iter() {
        let me = MacroElements::new(2.0, 1.0, 4.0, 1.5, 3.0);
        let mut collected: std::collections::HashMap<MacroElementsType, f32> =
            me.clone().into_iter().collect();

        for elem in MacroElementsType::iter() {
            assert_eq!(collected.remove(&elem), Some(me[elem]));
        }
        assert!(collected.is_empty());
    }

    #[test]
    fn test_macro_elements_iter_order_and_values() {
        let me = MacroElements::new(5.0, 2.0, 7.0, 1.0, 3.0);
        let iter = me.clone().into_iter();
        let mut seen = std::collections::HashSet::new();

        for (elem_type, value) in iter {
            assert_eq!(value, me[elem_type]);
            assert!(seen.insert(elem_type), "Duplicate element in iterator");
        }
        assert_eq!(seen.len(), MacroElementsType::iter().count());
    }
}
