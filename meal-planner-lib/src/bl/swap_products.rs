use super::constraints_solver::Fraction;
use crate::data_types::{AllowedUnitsType, NutrientType, Product};

struct ProductSwapper {}

impl ProductSwapper {
    pub fn get_grams_of_swapped_product(
        input: &Product,
        amount: f32,
        output: &Product,
        nutrient_equivalent: NutrientType,
    ) -> Result<(u16, u16), String> {
        let (low, high) = Self::get_amount_of_swapped_product(
            input,
            amount,
            output,
            nutrient_equivalent,
            AllowedUnitsType::Gram,
        )?;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok((
            (f32::from(low.numerator) / f32::from(low.denominator)).floor() as u16,
            (f32::from(high.numerator) / f32::from(high.denominator)).ceil() as u16,
        ))
    }

    pub fn get_amount_of_swapped_product(
        input: &Product,
        amount: f32,
        output: &Product,
        nutrient_equivalent: NutrientType,
        allowed_units: AllowedUnitsType,
    ) -> Result<(Fraction, Fraction), String> {
        let Some(nutrient_amount) = input.get_nutrient_amount(nutrient_equivalent) else {
            return Err(format!(
                "Input product '{}' does not have nutrient '{:?}'",
                input.id(),
                nutrient_equivalent
            ));
        };
        let Some(nutrient_amount_output) = output.get_nutrient_amount(nutrient_equivalent) else {
            return Err(format!(
                "Output product '{}' does not have nutrient '{:?}'",
                output.id(),
                nutrient_equivalent
            ));
        };
        let Some(unit_size) = input.allowed_units.get(&allowed_units) else {
            return Err(format!(
                "Input product '{}' does not have allowed unit '{:?}'",
                input.id(),
                allowed_units
            ));
        };
        let calculated_amout = amount * nutrient_amount / nutrient_amount_output;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let low_calculated_amount = Fraction {
            numerator: (calculated_amout * f32::from(unit_size.divider)
                / f32::from(unit_size.amount).floor()) as u16,
            denominator: unit_size.divider,
        };
        let high_calculated_amount = Fraction {
            numerator: low_calculated_amount.numerator + 1,
            denominator: unit_size.divider,
        };
        Ok((low_calculated_amount, high_calculated_amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_types::{
        MacroElements, MacroElementsType, MicroNutrients, MicroNutrientsType, UnitData,
    };

    fn make_product(
        name: &str,
        fat_per_100g: f32,
        extra_units: &[(AllowedUnitsType, UnitData)],
    ) -> Product {
        let mut allowed_units = std::collections::HashMap::new();
        allowed_units.insert(
            AllowedUnitsType::Gram,
            UnitData {
                amount: 1,
                divider: 1,
            },
        );
        for (unit, data) in extra_units {
            allowed_units.insert(*unit, *data);
        }
        Product::new(
            name.to_string(),
            None,
            Box::new(MacroElements::new(fat_per_100g, 0.0, 0.0, 0.0, 0.0)),
            Box::new(MicroNutrients::default()),
            allowed_units,
        )
    }

    #[test]
    fn converts_between_products_in_grams() {
        let olive = make_product("Olive", 91.0, &[]);
        let milk = make_product("Milk", 3.0, &[]);
        let (low, high) = ProductSwapper::get_grams_of_swapped_product(
            &olive,
            55.0,
            &milk,
            NutrientType::Macro(MacroElementsType::Fat),
        )
        .expect("conversion should succeed");
        assert_eq!(low, 1668);
        assert_eq!(high, 1669);
    }

    #[test]
    fn converts_using_custom_units() {
        let glass_unit = UnitData {
            amount: 250,
            divider: 2,
        };
        let olive = make_product("Olive", 91.0, &[(AllowedUnitsType::Custom, glass_unit)]);
        let milk = make_product("Milk", 3.0, &[(AllowedUnitsType::Custom, glass_unit)]);
        let (low, high) = ProductSwapper::get_amount_of_swapped_product(
            &olive,
            55.0,
            &milk,
            NutrientType::Macro(MacroElementsType::Fat),
            AllowedUnitsType::Custom,
        )
        .expect("conversion should succeed");
        assert_eq!(low.numerator, 13);
        assert_eq!(low.denominator, 2);
        assert_eq!(high.numerator, 14);
        assert_eq!(high.denominator, 2);
    }

    #[test]
    fn error_when_input_missing_nutrient() {
        let olive = make_product("Olive", 91.0, &[]);
        let milk = make_product("Milk", 3.0, &[]);
        let nutrient = NutrientType::Micro(MicroNutrientsType::Fiber);
        let err = ProductSwapper::get_amount_of_swapped_product(
            &olive,
            10.0,
            &milk,
            nutrient,
            AllowedUnitsType::Gram,
        )
        .expect_err("missing nutrient should return error");
        let expected = format!("Input product 'Olive' does not have nutrient '{nutrient:?}'");
        assert_eq!(err, expected);
    }

    #[test]
    fn error_when_allowed_unit_missing_on_input() {
        let olive = make_product("Olive", 91.0, &[]);
        let milk = make_product("Milk", 3.0, &[]);
        let err = ProductSwapper::get_amount_of_swapped_product(
            &olive,
            55.0,
            &milk,
            NutrientType::Macro(MacroElementsType::Fat),
            AllowedUnitsType::Cup,
        )
        .expect_err("missing unit should return error");
        assert_eq!(
            err,
            "Input product 'Olive' does not have allowed unit 'Cup'"
        );
    }
}
