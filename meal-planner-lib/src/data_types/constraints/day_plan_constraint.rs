use std::collections::HashMap;

use super::MealConstraint;
use super::NutrientConstraint;

pub struct DayMealPlanConstraint {
    pub meals: HashMap<String, MealConstraint>,
    pub nutrients: Vec<NutrientConstraint>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
