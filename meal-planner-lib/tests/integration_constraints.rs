#![cfg(feature = "test-utils")]

use std::collections::HashMap;

use approx::assert_relative_eq;
use meal_planner_lib::bl::constraints_solver::{ConstraintsSolver, MinOrMax, SolutionEntry};
use meal_planner_lib::data_types::{
    AllowedUnitsType, MacroElementsType, MicroNutrientsType, NutrientType,
    constraints::{DayMealPlanConstraint, MealConstraint, NutrientConstraint, ProductConstraint},
};
use meal_planner_lib::database_access::{DataBaseTypes, DbSearchCriteria, DbWrapper, get_db};

fn mock_db() -> Box<dyn DbWrapper> {
    get_db(DataBaseTypes::Mock).expect("mock database should be available")
}

fn extract_single_product<'a>(
    solution: &'a SolutionEntry,
    meal_name: &str,
) -> (
    &'a meal_planner_lib::data_types::Product,
    f64,
    &'a meal_planner_lib::bl::constraints_solver::Fraction,
) {
    let week_entries = match solution {
        SolutionEntry::Week { entries } => entries,
        _ => panic!("Expected week entry"),
    };
    assert_eq!(week_entries.len(), 1, "Expected single day in plan");

    let day_entry = match &week_entries[0] {
        SolutionEntry::Day { name, entries } => {
            assert_eq!(name, "Day1");
            entries
        }
        _ => panic!("Expected day entry"),
    };
    assert_eq!(day_entry.len(), 1, "Expected single meal for the day");

    let meal_entry = match &day_entry[0] {
        SolutionEntry::Meal { name, entries } => {
            assert_eq!(name, meal_name);
            entries
        }
        _ => panic!("Expected meal entry"),
    };
    assert_eq!(meal_entry.len(), 1, "Expected single product for the meal");

    match &meal_entry[0] {
        SolutionEntry::Product {
            product,
            amount_grams,
            unit,
            amount_unit,
        } => {
            assert_eq!(*unit, AllowedUnitsType::Gram);
            (product, *amount_grams, amount_unit)
        }
        _ => panic!("Expected product entry"),
    }
}

#[test]
fn test_user_can_build_day_plan_from_mock_db() {
    let db = mock_db();
    let criteria = [DbSearchCriteria::ById("Apple".to_string())];
    let results = db.get_products_matching_criteria(&criteria);
    assert_eq!(results.len(), 1);
    let apple = results
        .get("Apple (BrandedApple)")
        .expect("apple should exist")
        .clone();

    let apple_constraint = ProductConstraint::new(
        Box::new(apple.clone()),
        Some(0),
        Some(300),
        AllowedUnitsType::Gram,
    )
    .expect("apple should support gram unit");

    let breakfast = MealConstraint {
        products: vec![apple_constraint],
        nutrients: vec![
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(5.0), Some(6.0))
                .expect("valid fiber constraint"),
        ],
    };

    let mut meals = HashMap::new();
    meals.insert("Breakfast".to_string(), breakfast);

    let day_constraints = DayMealPlanConstraint {
        meals,
        nutrients: vec![
            NutrientConstraint::new(MacroElementsType::Calories, Some(50.0), Some(70.0))
                .expect("valid calorie constraint"),
        ],
    };

    let mut solver = ConstraintsSolver::new(
        MinOrMax::Min,
        NutrientType::Macro(MacroElementsType::Calories),
    );
    let solution = solver
        .solve_day(&day_constraints)
        .expect("solution should be feasible");

    let (product, grams, fraction) = extract_single_product(&solution.solution, "Breakfast");
    assert_eq!(product.name(), "Apple");
    assert_relative_eq!(grams, 200.0, epsilon = 1e-6);
    assert_eq!(fraction.denominator, 1);
    assert_eq!(fraction.numerator, 200);

    let fiber_per_100 = f64::from(
        product
            .get_nutrient_amount(NutrientType::Micro(MicroNutrientsType::Fiber))
            .expect("fiber data should exist"),
    );
    let total_fiber = (fiber_per_100 / 100.0) * grams;
    assert!(total_fiber >= 5.0 - 1e-6 && total_fiber <= 6.0 + 1e-6);

    let calories_per_100 = f64::from(
        product
            .get_nutrient_amount(NutrientType::Macro(MacroElementsType::Calories))
            .expect("calorie data should exist"),
    );
    let total_calories = (calories_per_100 / 100.0) * grams;
    assert!(total_calories >= 50.0 - 1e-6 && total_calories <= 70.0 + 1e-6);
}

#[test]
fn test_user_receives_infeasible_error_from_mock_db() {
    let db = mock_db();
    let results =
        db.get_products_matching_criteria(&[DbSearchCriteria::ById("Banana".to_string())]);
    assert_eq!(results.len(), 1);
    let banana = results
        .get("Banana (BrandedBanana)")
        .expect("banana should exist")
        .clone();

    let banana_constraint = ProductConstraint::new(
        Box::new(banana.clone()),
        Some(0),
        Some(150),
        AllowedUnitsType::Gram,
    )
    .expect("banana should support gram unit");

    let snack = MealConstraint {
        products: vec![banana_constraint],
        nutrients: vec![
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(50.0), Some(60.0))
                .expect("valid fiber constraint"),
        ],
    };

    let mut meals = HashMap::new();
    meals.insert("Snack".to_string(), snack);

    let day_constraints = DayMealPlanConstraint {
        meals,
        nutrients: Vec::new(),
    };

    let mut solver = ConstraintsSolver::new(
        MinOrMax::Min,
        NutrientType::Macro(MacroElementsType::Calories),
    );
    let result = solver.solve_day(&day_constraints);
    assert!(matches!(result, Err(message) if message == "Constraints are infeasible"));
}

#[test]
fn test_user_retrieves_multi_meal_solution_from_mock_db() {
    let db = mock_db();

    let apple = db
        .get_product_by_id("Apple (BrandedApple)")
        .expect("apple should exist");
    let banana = db
        .get_product_by_id("Banana (BrandedBanana)")
        .expect("banana should exist");

    let apple_constraint = ProductConstraint::new(
        Box::new(apple.clone()),
        Some(0),
        Some(300),
        AllowedUnitsType::Gram,
    )
    .expect("apple should support gram unit");
    let banana_constraint = ProductConstraint::new(
        Box::new(banana.clone()),
        Some(0),
        Some(200),
        AllowedUnitsType::Gram,
    )
    .expect("banana should support gram unit");

    let breakfast = MealConstraint {
        products: vec![apple_constraint],
        nutrients: vec![
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(3.0), Some(3.2))
                .expect("valid breakfast fiber constraint"),
        ],
    };

    let dinner = MealConstraint {
        products: vec![banana_constraint],
        nutrients: vec![
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(4.0), Some(4.2))
                .expect("valid dinner fiber constraint"),
        ],
    };

    let mut meals = HashMap::new();
    meals.insert("Breakfast".to_string(), breakfast);
    meals.insert("Dinner".to_string(), dinner);

    let day_constraints = DayMealPlanConstraint {
        meals,
        nutrients: vec![
            NutrientConstraint::new(MicroNutrientsType::Fiber, Some(6.8), Some(7.2))
                .expect("valid day fiber constraint"),
            NutrientConstraint::new(MacroElementsType::Protein, Some(4.5), Some(5.5))
                .expect("valid day protein constraint"),
            NutrientConstraint::new(MacroElementsType::Calories, Some(55.0), Some(65.0))
                .expect("valid day calorie constraint"),
        ],
    };

    let mut solver = ConstraintsSolver::new(
        MinOrMax::Min,
        NutrientType::Macro(MacroElementsType::Calories),
    );
    let solution = solver
        .solve_day(&day_constraints)
        .expect("solution should be feasible");

    let week_entries = match &solution.solution {
        SolutionEntry::Week { entries } => entries,
        _ => panic!("Expected week entry"),
    };
    assert_eq!(week_entries.len(), 1, "Expected single day");

    let day_entries = match &week_entries[0] {
        SolutionEntry::Day { name, entries } => {
            assert_eq!(name, "Day1");
            entries
        }
        _ => panic!("Expected day entry"),
    };

    let mut meals_map: HashMap<&str, &Vec<SolutionEntry>> = HashMap::new();
    for meal_entry in day_entries {
        match meal_entry {
            SolutionEntry::Meal { name, entries } => {
                meals_map.insert(name.as_str(), entries);
            }
            _ => panic!("Expected meal entry"),
        }
    }

    let breakfast_entries = meals_map.get("Breakfast").expect("expected breakfast meal");
    assert_eq!(breakfast_entries.len(), 1);
    let (breakfast_product, breakfast_grams) = match &breakfast_entries[0] {
        SolutionEntry::Product {
            product,
            amount_grams,
            unit,
            amount_unit,
        } => {
            assert_eq!(product.name(), "Apple");
            assert_eq!(*unit, AllowedUnitsType::Gram);
            assert_eq!(amount_unit.denominator, 1);
            assert!((*amount_grams - 120.0).abs() <= 1.0);
            assert!((i32::from(amount_unit.numerator) - 120).abs() <= 1);
            (product, *amount_grams)
        }
        _ => panic!("Expected product entry for breakfast"),
    };

    let dinner_entries = meals_map.get("Dinner").expect("expected dinner meal");
    assert_eq!(dinner_entries.len(), 1);
    let (dinner_product, dinner_grams) = match &dinner_entries[0] {
        SolutionEntry::Product {
            product,
            amount_grams,
            unit,
            amount_unit,
        } => {
            assert_eq!(product.name(), "Banana");
            assert_eq!(*unit, AllowedUnitsType::Gram);
            assert_eq!(amount_unit.denominator, 1);
            assert!((*amount_grams - 80.0).abs() <= 1.0);
            assert!((i32::from(amount_unit.numerator) - 80).abs() <= 1);
            (product, *amount_grams)
        }
        _ => panic!("Expected product entry for dinner"),
    };

    let apple_fiber = f64::from(
        breakfast_product
            .get_nutrient_amount(NutrientType::Micro(MicroNutrientsType::Fiber))
            .expect("apple fiber data"),
    );
    let banana_fiber = f64::from(
        dinner_product
            .get_nutrient_amount(NutrientType::Micro(MicroNutrientsType::Fiber))
            .expect("banana fiber data"),
    );
    let total_fiber =
        (apple_fiber / 100.0) * breakfast_grams + (banana_fiber / 100.0) * dinner_grams;
    assert!(total_fiber >= 6.8 - 1e-6 && total_fiber <= 7.2 + 1e-6);

    let apple_protein = f64::from(
        breakfast_product
            .get_nutrient_amount(NutrientType::Macro(MacroElementsType::Protein))
            .expect("apple protein data"),
    );
    let banana_protein = f64::from(
        dinner_product
            .get_nutrient_amount(NutrientType::Macro(MacroElementsType::Protein))
            .expect("banana protein data"),
    );
    let total_protein =
        (apple_protein / 100.0) * breakfast_grams + (banana_protein / 100.0) * dinner_grams;
    assert!(total_protein >= 4.5 - 1e-6 && total_protein <= 5.5 + 1e-6);

    let apple_calories = f64::from(
        breakfast_product
            .get_nutrient_amount(NutrientType::Macro(MacroElementsType::Calories))
            .expect("apple calorie data"),
    );
    let banana_calories = f64::from(
        dinner_product
            .get_nutrient_amount(NutrientType::Macro(MacroElementsType::Calories))
            .expect("banana calorie data"),
    );
    let total_calories =
        (apple_calories / 100.0) * breakfast_grams + (banana_calories / 100.0) * dinner_grams;
    assert!(total_calories >= 55.0 - 1e-6 && total_calories <= 65.0 + 1e-6);
}
