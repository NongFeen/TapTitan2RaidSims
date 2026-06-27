use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "success", content = "data")]
pub enum ApiResponse<T: Serialize> {
    #[serde(rename = "true")]
    Success { data: T },

    #[serde(rename = "false")]
    Error { error: ApiError },
}

#[derive(Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
