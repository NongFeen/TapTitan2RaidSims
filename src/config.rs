use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: Option<String>,
    pub port: u16,
    pub simulation_concurrency: usize,
    pub simulation_worker_count: usize,
    pub internal_api_key: String,
    pub cors_allowed_origins: Vec<String>,
    pub tt2: Option<Tt2Config>,
}

#[derive(Debug, Clone)]
pub struct Tt2Config {
    pub socket_url: String,
    pub socket_handshake_path: String,
    pub rest_base_url: String,
    pub application_token: String,
    pub player_token_encryption_key: String,
    pub raid_subscription_player_token: String,
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
        let simulation_worker_count = env::var("SIM_WORKER_COUNT")
            .unwrap_or_else(|_| "1".to_string())
            .parse::<usize>()
            .map_err(|_| {
                "SIM_WORKER_COUNT must be a non-negative integer (0 uses all available CPUs)"
                    .to_string()
            })?;
        let internal_api_key = env::var("INTERNAL_API_KEY")
            .map_err(|_| "INTERNAL_API_KEY must be configured".to_string())?;
        if internal_api_key.trim().len() < 16 {
            return Err("INTERNAL_API_KEY must contain at least 16 characters".to_string());
        }

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .map_err(|_| "CORS_ALLOWED_ORIGINS must be configured".to_string())?
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if cors_allowed_origins.is_empty() {
            return Err(
                "CORS_ALLOWED_ORIGINS must contain at least one origin".to_string(),
            );
        }

        let tt2_values = [
            env::var("TT2_SOCKET_URL").ok(),
            env::var("TT2_SOCKET_HANDSHAKE_PATH").ok(),
            env::var("TT2_REST_BASE_URL").ok(),
            env::var("TT2_APPLICATION_TOKEN").ok(),
            env::var("TT2_PLAYER_TOKEN_ENCRYPTION_KEY").ok(),
            env::var("TT2_RAID_SUBSCRIPTION_PLAYER_TOKEN").ok(),
        ];
        let tt2 = if tt2_values
            .iter()
            .all(|value| value.as_ref().is_some_and(|value| !value.trim().is_empty()))
        {
            Some(Tt2Config {
                socket_url: tt2_values[0].clone().unwrap(),
                socket_handshake_path: tt2_values[1].clone().unwrap(),
                rest_base_url: tt2_values[2].clone().unwrap(),
                application_token: tt2_values[3].clone().unwrap(),
                player_token_encryption_key: tt2_values[4].clone().unwrap(),
                raid_subscription_player_token: tt2_values[5].clone().unwrap(),
            })
        } else {
            if tt2_values.iter().any(|value| value.is_some()) {
                return Err(
                    "TT2 configuration must provide all TT2_* variables or none of them"
                        .to_string(),
                );
            }
            None
        };

        Ok(Self {
            database_url,
            port,
            simulation_concurrency,
            simulation_worker_count,
            internal_api_key,
            cors_allowed_origins,
            tt2,
        })
    }
}
