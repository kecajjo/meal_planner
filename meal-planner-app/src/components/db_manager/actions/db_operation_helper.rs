use dioxus::prelude::*;

use dioxus_i18n::t;
use meal_planner_lib::data_types as data;
use meal_planner_lib::database_access as db_access;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum DbOperation {
    Add,
    Edit,
    Delete,
    None,
}

pub(super) fn operation_triggered(
    input: Signal<Option<data::Product>>,
    mut result_signal: Signal<Option<Result<(), String>>>,
    operation: DbOperation,
) {
    let product = match input() {
        Some(prod) => prod,
        None => {
            result_signal.set(Some(Err(t!("error-no-product"))));
            return;
        }
    };
    let product_id = product.id();

    spawn({
        let mut result_signal = result_signal.clone();
        async move {
            tracing::info!("Creating DB access");
            let Some(mut db) = db_access::get_mutable_db(db_access::DataBaseTypes::Local(
                db_access::LOCAL_DB_DEFAULT_FILE.to_string(),
            ))
            .await
            else {
                result_signal.set(Some(Err(t!("error-db-access"))));
                return;
            };
            tracing::info!("DB Accessed");
            let res = match operation {
                DbOperation::Add => db.add_product(&product_id, product).await,
                DbOperation::Edit => db.update_product(&product_id, product).await,
                DbOperation::Delete => db.delete_product(&product_id).await,
                DbOperation::None => Ok(()),
            };
            result_signal.set(Some(res));
        }
    });
}
