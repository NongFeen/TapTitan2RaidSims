use crate::models::ttboss::Boss;
use crate::services::taptitan::ttboss_service;

#[derive(Clone)]
pub struct AppState {
    pub boss: Boss,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            boss: ttboss_service::new_boss("Lojak"),
        }
    }
}