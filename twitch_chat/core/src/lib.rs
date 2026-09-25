use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub user: String,
    pub text: String,
    pub user_id: String,
}
