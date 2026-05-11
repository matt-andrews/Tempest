use serde::{Deserialize, Serialize};
use crate::models::option_model::OptionModel;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConfigModel{
    pub options: OptionModel
}