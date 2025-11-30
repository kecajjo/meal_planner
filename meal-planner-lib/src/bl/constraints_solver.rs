use crate::data_types::{
    AllowedUnitsType, NutrientType, Product,
    constraints::{DayMealPlanConstraint, MealConstraint, NutrientConstraint, ProductConstraint},
};
use microlp::{ComparisonOp, OptimizationDirection, Problem, Variable};

#[derive(Debug, Clone, Copy)]
pub(super) enum MinOrMax {
    Min,
    Max,
}

enum ProductEntry {
    Variable(ProductVariable),
    Subcontainer(ProductsContainer),
}

impl ProductEntry {
    fn get_all_products(&self) -> Box<dyn Iterator<Item = &ProductVariable> + '_> {
        match self {
            ProductEntry::Variable(var) => Box::new(std::iter::once(var)),
            ProductEntry::Subcontainer(container) => Box::new(
                container
                    .inner
                    .iter()
                    .flat_map(|entry| entry.get_all_products()),
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

pub(super) struct ConstraintsSolver {
    problem: Problem,
    variables: ProductsContainer,
    nutrient_to_optimize: NutrientType,
}

impl ConstraintsSolver {
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

    pub fn create_constraints(&mut self, day_constraints: &DayMealPlanConstraint) {
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
        // crate base product variable
        let nutrient_amount = f64::from(product
            .get_nutrient_amount(self.nutrient_to_optimize)
            .unwrap_or(0.0));

        // not int var as int contraint will be given on allowd_units level
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
                (unit_var, f64::from(unit_data.divider) * f64::from(unit_data.amount)),
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
        for p in products.iter().flat_map(|entry| entry.get_all_products()) {
            product_macros.push((
                p.variable_gram,
                f64::from(p.product
                    .get_nutrient_amount(nutrient_constr.element())
                    .unwrap_or(0.0))
                    * 0.01,
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
