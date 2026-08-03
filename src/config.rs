use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: Option<String>,
    pub port: u16,
    pub simulation_concurrency: usize,
    pub internal_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|_| "PORT must be a valid u16".to_string())?;
        let simulation_concurrency = env::var("SIMULATION_CONCURRENCY")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<usize>()
            .map_err(|_| "SIMULATION_CONCURRENCY must be a positive integer".to_string())?;
        if simulation_concurrency == 0 {
            return Err("SIMULATION_CONCURRENCY must be greater than zero".to_string());
        }
        let internal_api_key = env::var("INTERNAL_API_KEY")
            .map_err(|_| "INTERNAL_API_KEY must be configured".to_string())?;
        if internal_api_key.trim().len() < 16 {
            return Err("INTERNAL_API_KEY must contain at least 16 characters".to_string());
        }

        Ok(Self {
            database_url,
            port,
            simulation_concurrency,
            internal_api_key,
        })
    }
}
