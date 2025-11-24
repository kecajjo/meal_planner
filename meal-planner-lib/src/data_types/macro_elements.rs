use std::{
    hash::Hash,
    ops::{Add, Index},
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
