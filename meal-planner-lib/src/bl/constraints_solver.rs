use crate::data_types::{
    AllowedUnitsType, NutrientType, Product,
    constraints::{DayMealPlanConstraint, MealConstraint, NutrientConstraint, ProductConstraint},
};

use microlp::{ComparisonOp, OptimizationDirection, Problem, Variable};

#[derive(Debug, Clone, Copy)]
pub enum MinOrMax {
    Min,
    Max,
}

enum ProductEntry {
    Variable(ProductVariable),
    Subcontainer(ProductsContainer),
}

impl ProductEntry {
    fn get_all_product_variables(&self) -> Box<dyn Iterator<Item = &ProductVariable> + '_> {
        match self {
            ProductEntry::Variable(var) => Box::new(std::iter::once(var)),
            ProductEntry::Subcontainer(container) => Box::new(
                container
                    .inner
                    .iter()
                    .flat_map(|entry| entry.get_all_product_variables()),
            ),
        }
    }
}

struct ProductsContainer {
    name: String,
    inner: Vec<ProductEntry>,
}
struct ProductVariable {
    name: String,
    product: Product,
    unit: AllowedUnitsType,
    variable_gram: Variable,
    variable_unit_divided: Variable,
}

pub struct Fraction {
    pub numerator: u16,
    pub denominator: u16,
}

pub enum SolutionEntry {
    Week {
        entries: Vec<SolutionEntry>,
    },
    Day {
        name: String,
        entries: Vec<SolutionEntry>,
    },
    Meal {
        name: String,
        entries: Vec<SolutionEntry>,
    },
    Product {
        product: Product,
        amount_grams: f64,
        unit: AllowedUnitsType,
        amount_unit: Fraction,
    },
}

pub struct Solution {
    pub solution: SolutionEntry,
}

pub struct ConstraintsSolver {
    problem: Problem,
    variables: ProductsContainer,
    nutrient_to_optimize: NutrientType,
}

impl ConstraintsSolver {
    #[must_use]
    pub fn new(min_or_max: MinOrMax, nutrient_to_optimize: NutrientType) -> Self {
        let problem = match min_or_max {
            MinOrMax::Min => Problem::new(OptimizationDirection::Minimize),
            MinOrMax::Max => Problem::new(OptimizationDirection::Maximize),
        };

        Self {
            problem,
            variables: ProductsContainer {
                name: "root".to_string(),
                inner: Vec::new(),
            },
            nutrient_to_optimize,
        }
    }

    pub fn solve_day(
        &mut self,
        day_constraints: &DayMealPlanConstraint,
    ) -> Result<Solution, String> {
        self.create_constraints(day_constraints);
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.problem.solve() {
            Ok(s) => Ok(self.solver_solution_to_output(&s)),
            Err(e) => match e {
                microlp::Error::Infeasible => Err("Constraints are infeasible".to_string()),
                microlp::Error::Unbounded => Err("Problem is unbounded".to_string()),
                _ => Err(format!("Solving error: {e:?}")),
            },
        }
    }

    fn solver_solution_to_output(&self, solution: &microlp::Solution) -> Solution {
        let mut week = Vec::new();
        for day in self.variables.inner.iter().map(|x| {
            if let ProductEntry::Subcontainer(c) = x {
                c
            } else {
                panic!("Expected day container")
            }
        }) {
            let mut day_entries = Vec::new();
            for meal in day.inner.iter().map(|x| {
                if let ProductEntry::Subcontainer(c) = x {
                    c
                } else {
                    panic!("Expected meal container")
                }
            }) {
                let mut meal_entries: Vec<SolutionEntry> = Vec::new();
                for product in meal.inner.iter().map(|x| {
                    if let ProductEntry::Variable(v) = x {
                        v
                    } else {
                        panic!("Expected product variable")
                    }
                }) {
                    let product_solution = SolutionEntry::Product {
                        product: product.product.clone(),
                        amount_grams: *solution.var_value(product.variable_gram),
                        unit: product.unit,
                        amount_unit: Fraction {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            numerator: *solution.var_value(product.variable_unit_divided) as u16,
                            denominator: product
                                .product
                                .allowed_units
                                .get(&product.unit)
                                .unwrap()
                                .divider,
                        },
                    };
                    meal_entries.push(product_solution);
                }
                let meal_solution = SolutionEntry::Meal {
                    name: meal.name.clone(),
                    entries: meal_entries,
                };
                day_entries.push(meal_solution);
            }
            let day_solution = SolutionEntry::Day {
                name: day.name.clone(),
                entries: day_entries,
            };
            week.push(day_solution);
        }
        Solution {
            solution: SolutionEntry::Week { entries: week },
        }
    }

    fn create_constraints(&mut self, day_constraints: &DayMealPlanConstraint) {
        let mut day_vec = Vec::new();
        self.create_day_constraints(day_constraints, &mut day_vec);
        self.variables
            .inner
            .push(ProductEntry::Subcontainer(ProductsContainer {
                name: "Day1".to_string(),
                inner: day_vec,
            }));
    }

    fn create_day_constraints(
        &mut self,
        day_constraints: &DayMealPlanConstraint,
        product_entries: &mut Vec<ProductEntry>,
    ) {
        // TODO: maybe in the future there will be day-level product constraints
        // 1st constraint all products (at they are variables not constraints)

        // constraint all meals
        for (meal_name, meal) in &day_constraints.meals {
            let mut meal_container = ProductsContainer {
                name: meal_name.clone(),
                inner: Vec::new(),
            };

            self.add_meal_constraints(meal, &mut meal_container.inner);
            product_entries.push(ProductEntry::Subcontainer(meal_container));
        }

        // then nutrients constraints
        for nutrient_constr in &day_constraints.nutrients {
            self.add_nutrient_constraints(nutrient_constr, product_entries);
        }
    }

    fn add_meal_constraints(
        &mut self,
        meal: &MealConstraint,
        product_entries: &mut Vec<ProductEntry>,
    ) {
        // 1st products as they are variables
        for product_constraint in &meal.products {
            let product_variable =
                self.add_product_constraints(product_constraint.food(), product_constraint);
            product_entries.push(ProductEntry::Variable(product_variable));
        }

        // then nutrients constraints
        for nutrient_constr in &meal.nutrients {
            self.add_nutrient_constraints(nutrient_constr, product_entries);
        }
    }

    // for now return product constraint in grams, but might change to return in allowed unit with divider
    fn add_product_constraints(
        &mut self,
        product: &Product,
        product_constraint: &ProductConstraint,
    ) -> ProductVariable {
        // create base product variable
        let nutrient_amount = f64::from(
            product
                .get_nutrient_amount(self.nutrient_to_optimize)
                .unwrap_or(0.0),
        );

        // not int var as int constraint will be given on allowed_units level
        let product_gram_variable = self.problem.add_var(
            // nutrient amount per 1g of product
            nutrient_amount * 0.01,
            (
                f64::from(product_constraint.low_bound().unwrap_or(0)),
                f64::from(product_constraint.up_bound().unwrap_or(u16::MAX)),
            ),
        );

        // add variables for the product units including dividers
        let unit_var = self.problem.add_integer_var(0.0, (0, i32::from(u16::MAX)));
        let unit_data = product
            .allowed_units
            .get(&product_constraint.unit())
            .unwrap();

        self.problem.add_constraint(
            [
                (
                    unit_var,
                    f64::from(unit_data.divider) * f64::from(unit_data.amount),
                ),
                (product_gram_variable, -1.0),
            ],
            ComparisonOp::Eq,
            0.0,
        );

        ProductVariable {
            name: product.id(),
            product: product.clone(),
            unit: product_constraint.unit(),
            variable_gram: product_gram_variable,
            variable_unit_divided: unit_var,
        }
    }

    // no need to keep references to nutrients as they are stored in the problem
    // Information about their values can be calculated based on products and their quantities
    fn add_nutrient_constraints(
        &mut self,
        nutrient_constr: &NutrientConstraint,
        products: &[ProductEntry],
    ) {
        let mut product_macros = Vec::new();
        for p in products
            .iter()
            .flat_map(|entry| entry.get_all_product_variables())
        {
            product_macros.push((
                p.variable_gram,
                f64::from(
                    p.product
                        .get_nutrient_amount(nutrient_constr.element())
                        .unwrap_or(0.0),
                ) * 0.01,
            ));
        }
        self.problem.add_constraint(
            &product_macros,
            ComparisonOp::Ge,
            f64::from(nutrient_constr.min().unwrap_or(0.0)),
        );

        if let Some(max_val) = nutrient_constr.max() {
            self.problem
                .add_constraint(&product_macros, ComparisonOp::Le, f64::from(max_val));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::collections::HashMap;

    use crate::data_types::{
        AllowedUnitsType, MacroElements, MacroElementsType, MicroNutrients, MicroNutrientsType,
        NutrientType, Product, UnitData, constraints::*,
    };

    fn build_product(
        name: &str,
        protein_per_100g: f32,
        gram_amount: u16,
        gram_divider: u16,
        fiber: Option<f32>,
    ) -> Product {
        let macro_elements = Box::new(MacroElements::new(5.0, 1.0, 10.0, 2.0, protein_per_100g));
        let mut micro_nutrients = Box::new(MicroNutrients::default());
        if let Some(fiber_value) = fiber {
            micro_nutrients[MicroNutrientsType::Fiber] = Some(fiber_value);
        }

        let mut allowed_units = HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: gram_amount,
                divider: gram_divider,
            },
        );

        Product::new(
            name.to_string(),
            None,
            macro_elements,
            micro_nutrients,
            allowed_units,
        )
    }

    fn make_day_constraint(
        meal_name: &str,
        meal: MealConstraint,
        day_nutrients: Vec<NutrientConstraint>,
    ) -> DayMealPlanConstraint {
        let mut meals = HashMap::new();
        meals.insert(meal_name.to_string(), meal);
        DayMealPlanConstraint {
            meals,
            nutrients: day_nutrients,
        }
    }

    #[test]
    fn test_solver_handles_feasible_day() {
        let mut solver = ConstraintsSolver::new(
            MinOrMax::Min,
            NutrientType::Macro(MacroElementsType::Protein),
        );

        let product = build_product("ProteinPowder", 40.0, 1, 1, Some(5.0));
        let product_constraint = ProductConstraint::new(
            Box::new(product.clone()),
            Some(0),
            Some(200),
            AllowedUnitsType::Gram,
        )
        .expect("product constraint should be valid");

        let meal = MealConstraint {
            products: vec![product_constraint],
            nutrients: vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(20.0), Some(100.0))
                    .unwrap(),
                NutrientConstraint::new(MacroElementsType::Fat, Some(0.0), None).unwrap(),
            ],
        };

        let day_constraint = make_day_constraint(
            "Breakfast",
            meal,
            vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(15.0), Some(120.0))
                    .unwrap(),
                NutrientConstraint::new(MicroNutrientsType::Alcohol, None, Some(0.0)).unwrap(),
            ],
        );

        let solution = solver
            .solve_day(&day_constraint)
            .expect("solution should exist");

        match solution.solution {
            SolutionEntry::Week {
                entries: week_entries,
            } => {
                assert_eq!(week_entries.len(), 1);
                match &week_entries[0] {
                    SolutionEntry::Day {
                        name,
                        entries: day_entries,
                    } => {
                        assert_eq!(name, "Day1");
                        assert_eq!(day_entries.len(), 1);
                        match &day_entries[0] {
                            SolutionEntry::Meal {
                                name: meal_name,
                                entries: meal_entries,
                            } => {
                                assert_eq!(meal_name, "Breakfast");
                                assert_eq!(meal_entries.len(), 1);
                                match &meal_entries[0] {
                                    SolutionEntry::Product {
                                        product: solved_product,
                                        amount_grams,
                                        unit,
                                        amount_unit,
                                    } => {
                                        assert_eq!(solved_product.name(), product.name());
                                        assert_eq!(*unit, AllowedUnitsType::Gram);
                                        assert_relative_eq!(*amount_grams, 50.0, epsilon = 1e-4);
                                        assert_eq!(amount_unit.numerator, 50);
                                        assert_eq!(amount_unit.denominator, 1);
                                    }
                                    _ => panic!("Expected product entry"),
                                }
                            }
                            _ => panic!("Expected meal entry"),
                        }
                    }
                    _ => panic!("Expected day entry"),
                }
            }
            _ => panic!("Expected week entry"),
        }
    }

    #[test]
    fn test_solver_reports_infeasible_constraints() {
        let mut solver = ConstraintsSolver::new(
            MinOrMax::Min,
            NutrientType::Macro(MacroElementsType::Protein),
        );

        let product = build_product("ProteinPowder", 40.0, 1, 1, None);
        let product_constraint =
            ProductConstraint::new(Box::new(product), Some(0), Some(60), AllowedUnitsType::Gram)
                .expect("product constraint should be valid");

        let meal = MealConstraint {
            products: vec![product_constraint],
            nutrients: vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(200.0), Some(220.0))
                    .unwrap(),
            ],
        };

        let day_constraint = make_day_constraint("Lunch", meal, Vec::new());

        let result = solver.solve_day(&day_constraint);
        assert!(matches!(result, Err(msg) if msg == "Constraints are infeasible"));
    }

    #[test]
    fn test_solver_reports_unbounded_problem() {
        let mut solver = ConstraintsSolver::new(
            MinOrMax::Max,
            NutrientType::Macro(MacroElementsType::Protein),
        );
        solver.problem.add_var(1.0, (0.0, f64::INFINITY));

        let product = build_product("Unbounded", 10.0, 1, 1, None);
        let product_constraint =
            ProductConstraint::new(Box::new(product), None, None, AllowedUnitsType::Gram)
                .expect("product constraint should be valid");

        let meal = MealConstraint {
            products: vec![product_constraint],
            nutrients: Vec::new(),
        };

        let day_constraint = make_day_constraint("Dinner", meal, Vec::new());

        let result = solver.solve_day(&day_constraint);
        assert!(matches!(result, Err(msg) if msg == "Problem is unbounded"));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_solver_handles_multiple_meals_and_day_constraints() {
        let mut solver = ConstraintsSolver::new(
            MinOrMax::Min,
            NutrientType::Macro(MacroElementsType::Protein),
        );

        let eggs_fiber = 8.0;
        let beans_fiber = 5.0;
        let spinach_fiber = 20.0;

        let eggs = build_product("Eggs", 30.0, 50, 1, Some(eggs_fiber));
        let beans = build_product("Beans", 18.0, 100, 1, Some(beans_fiber));
        let spinach = build_product("Spinach", 4.0, 25, 1, Some(spinach_fiber));

        let breakfast = MealConstraint {
            products: vec![
                ProductConstraint::new(
                    Box::new(eggs.clone()),
                    Some(50),
                    Some(300),
                    AllowedUnitsType::Gram,
                )
                .expect("eggs constraint should be valid"),
            ],
            nutrients: vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(30.0), Some(80.0))
                    .unwrap(),
            ],
        };

        let dinner = MealConstraint {
            products: vec![
                ProductConstraint::new(
                    Box::new(beans.clone()),
                    Some(100),
                    Some(300),
                    AllowedUnitsType::Gram,
                )
                .expect("beans constraint should be valid"),
                ProductConstraint::new(
                    Box::new(spinach.clone()),
                    Some(0),
                    Some(100),
                    AllowedUnitsType::Gram,
                )
                .expect("spinach constraint should be valid"),
            ],
            nutrients: vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(35.0), Some(90.0))
                    .unwrap(),
                NutrientConstraint::new(MicroNutrientsType::Fiber, Some(10.0), Some(20.0)).unwrap(),
            ],
        };

        let mut meals = HashMap::new();
        meals.insert("Breakfast".to_string(), breakfast);
        meals.insert("Dinner".to_string(), dinner);

        let day_constraint = DayMealPlanConstraint {
            meals,
            nutrients: vec![
                NutrientConstraint::new(MacroElementsType::Protein, Some(65.0), Some(90.0))
                    .unwrap(),
                NutrientConstraint::new(MicroNutrientsType::Fiber, Some(25.0), Some(35.0)).unwrap(),
            ],
        };

        let solution = solver
            .solve_day(&day_constraint)
            .expect("solution should exist");

        let SolutionEntry::Week {
            entries: week_entries,
        } = solution.solution
        else {
            panic!("Expected week entry")
        };
        assert_eq!(week_entries.len(), 1);

        let day_entry = week_entries.into_iter().next().expect("expected day entry");
        let day_entries = match day_entry {
            SolutionEntry::Day { name, entries } => {
                assert_eq!(name, "Day1");
                entries
            }
            _ => panic!("Expected day entry"),
        };

        let mut meals_map: HashMap<_, _> = HashMap::new();
        for meal_entry in day_entries {
            match meal_entry {
                SolutionEntry::Meal { name, entries } => {
                    meals_map.insert(name, entries);
                }
                _ => panic!("Expected meal entry"),
            }
        }

        let breakfast_entries = meals_map
            .remove("Breakfast")
            .expect("expected breakfast meal");
        assert_eq!(breakfast_entries.len(), 1);
        let (eggs_amount, eggs_units) = match &breakfast_entries[0] {
            SolutionEntry::Product {
                product,
                amount_grams,
                unit,
                amount_unit,
            } => {
                assert_eq!(product.name(), "Eggs");
                assert_eq!(*unit, AllowedUnitsType::Gram);
                assert_eq!(amount_unit.denominator, 1);
                assert_relative_eq!(*amount_grams, 100.0, epsilon = 1e-6);
                (*amount_grams, amount_unit.numerator)
            }
            _ => panic!("Expected product entry in breakfast"),
        };
        assert_eq!(eggs_units, 2);

        let dinner_entries = meals_map.remove("Dinner").expect("expected dinner meal");
        assert_eq!(dinner_entries.len(), 2);

        let mut beans_amount = None;
        let mut beans_units = None;
        let mut spinach_amount = None;
        let mut spinach_units = None;

        for entry in &dinner_entries {
            match entry {
                SolutionEntry::Product {
                    product,
                    amount_grams,
                    unit,
                    amount_unit,
                } => match product.name() {
                    "Beans" => {
                        assert_eq!(*unit, AllowedUnitsType::Gram);
                        assert_eq!(amount_unit.denominator, 1);
                        assert_relative_eq!(*amount_grams, 200.0, epsilon = 1e-6);
                        beans_amount = Some(*amount_grams);
                        beans_units = Some(amount_unit.numerator);
                    }
                    "Spinach" => {
                        assert_eq!(*unit, AllowedUnitsType::Gram);
                        assert_eq!(amount_unit.denominator, 1);
                        assert_relative_eq!(*amount_grams, 50.0, epsilon = 1e-6);
                        spinach_amount = Some(*amount_grams);
                        spinach_units = Some(amount_unit.numerator);
                    }
                    other => panic!("Unexpected product {other}"),
                },
                _ => panic!("Expected product entry in dinner"),
            }
        }

        let beans_amount = beans_amount.expect("beans amount not found");
        let beans_units = beans_units.expect("beans units not found");
        let spinach_amount = spinach_amount.expect("spinach amount not found");
        let spinach_units = spinach_units.expect("spinach units not found");

        assert_eq!(beans_units, 2);
        assert_eq!(spinach_units, 2);

        let total_fiber = (f64::from(eggs_fiber) / 100.0) * eggs_amount
            + (f64::from(beans_fiber) / 100.0) * beans_amount
            + (f64::from(spinach_fiber) / 100.0) * spinach_amount;
        assert!(total_fiber >= 25.0);
        assert!(total_fiber <= 35.0);

        let total_protein = (30.0 / 100.0) * eggs_amount
            + (18.0 / 100.0) * beans_amount
            + (4.0 / 100.0) * spinach_amount;
        assert!(total_protein >= 65.0);
        assert!(total_protein <= 90.0);
    }
}
