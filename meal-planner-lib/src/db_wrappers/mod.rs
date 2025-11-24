mod db_wrapper;

mod local_db_cont;
#[cfg(test)]
mod mock_db;
mod open_food_facts_db_cont;

use local_db_cont::local_db;
use open_food_facts_db_cont::open_food_facts_db;

pub(crate) use db_wrapper::*;
