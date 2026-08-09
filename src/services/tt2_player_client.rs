use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};
use url::Url;

use crate::{
    config::Tt2Config,
    error::AppError,
    models::{
        player_data::{BoostedCard, Card, PlayerData, RaidStats},
        player_raid_data::PlayerRaidData,
    },
    services::taptitan::player_service::clean_data,
};

pub const PLAYER_PROPERTIES: [&str; 8] = [
    "player_code",
    "player_raid_level",
    "boosted_cards",
    "raid_research_tree",
    "raid_research_bonuses",
    "gemstone_research_tree_raid_bonuses",
    "equipment_set",
    "cards",
];

#[derive(Clone)]
pub struct TokenCipher(Aes256Gcm);

impl TokenCipher {
    pub fn from_base64(key: &str) -> Result<Self, String> {
        let bytes = STANDARD
            .decode(key.trim())
            .map_err(|_| "TT2_PLAYER_TOKEN_ENCRYPTION_KEY must be valid Base64".to_string())?;
        if bytes.len() != 32 {
            return Err(
                "TT2_PLAYER_TOKEN_ENCRYPTION_KEY must decode to exactly 32 bytes".to_string(),
            );
        }
        Ok(Self(
            Aes256Gcm::new_from_slice(&bytes).expect("validated AES-256 key length"),
        ))
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<(Vec<u8>, Vec<u8>), AppError> {
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = self
            .0
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| AppError::Internal("Could not encrypt player token".to_string()))?;
        Ok((ciphertext, nonce.to_vec()))
    }

    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<String, AppError> {
        if nonce.len() != 12 {
            return Err(AppError::Internal(
                "Stored player token nonce is invalid".to_string(),
            ));
        }
        let plaintext = self
            .0
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|_| {
                AppError::Internal("Stored player token could not be decrypted".to_string())
            })?;
        String::from_utf8(plaintext)
            .map_err(|_| AppError::Internal("Stored player token is not valid UTF-8".to_string()))
    }
}

pub struct Tt2PlayerClient {
    config: Tt2Config,
    cipher: TokenCipher,
    http: reqwest::Client,
    connected: AtomicBool,
}

impl Tt2PlayerClient {
    pub fn new(config: Tt2Config) -> Result<Arc<Self>, String> {
        let cipher = TokenCipher::from_base64(&config.player_token_encryption_key)?;
        Ok(Arc::new(Self {
            config,
            cipher,
            http: reqwest::Client::new(),
            connected: AtomicBool::new(false),
        }))
    }

    pub fn cipher(&self) -> &TokenCipher {
        &self.cipher
    }
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    pub async fn connect(self: &Arc<Self>) {
        let engine_url = match engine_url(&self.config) {
            Ok(url) => url,
            Err(error) => {
                tracing::error!(%error, "TT2 player socket configuration is invalid");
                return;
            }
        };
        tracing::info!(
            socket_url = %self.config.socket_url,
            handshake_path = %self.config.socket_handshake_path,
            namespace = "/player",
            "Connecting to TT2 Socket.IO"
        );
        let mut request = match engine_url.into_client_request() {
            Ok(request) => request,
            Err(error) => {
                tracing::error!(?error, "Could not build TT2 Socket.IO request");
                return;
            }
        };
        let header = match HeaderValue::from_str(&self.config.application_token) {
            Ok(header) => header,
            Err(_) => {
                tracing::error!("TT2 application token is not valid as an HTTP header");
                return;
            }
        };
        request.headers_mut().insert("API-Authenticate", header);
        let (mut socket, response) = match connect_async(request).await {
            Ok(connection) => connection,
            Err(error) => {
                self.connected.store(false, Ordering::Release);
                tracing::warn!(
                    ?error,
                    "TT2 /player socket unavailable; continuing in degraded mode"
                );
                return;
            }
        };
        tracing::info!(status = %response.status(), "TT2 Engine.IO WebSocket transport opened");

        while let Some(message) = socket.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    let packet = text.as_str();
                    if packet.starts_with('0') {
                        tracing::debug!(
                            "TT2 Engine.IO handshake received; joining /player namespace"
                        );
                        if let Err(error) = socket.send(Message::Text("40/player,".into())).await {
                            tracing::error!(?error, "Could not join TT2 /player namespace");
                            break;
                        }
                    } else if packet.starts_with("40/player") {
                        self.connected.store(true, Ordering::Release);
                        tracing::info!("TT2 /player Socket.IO namespace connected");
                    } else if packet == "2" {
                        if let Err(error) = socket.send(Message::Text("3".into())).await {
                            tracing::error!(?error, "Could not send TT2 Engine.IO pong");
                            break;
                        }
                    } else if packet.starts_with("41/player") {
                        self.connected.store(false, Ordering::Release);
                        tracing::warn!(packet, "TT2 /player disconnect event");
                    } else if let Some(payload) = packet.strip_prefix("42/player,") {
                        log_socket_event(payload, &self.connected);
                    } else if let Some(payload) = packet.strip_prefix("44/player,") {
                        self.connected.store(false, Ordering::Release);
                        tracing::error!(payload, "TT2 /player connect_error event");
                    } else {
                        tracing::debug!(packet, "TT2 Socket.IO packet received");
                    }
                }
                Ok(Message::Close(frame)) => {
                    tracing::warn!(?frame, "TT2 WebSocket closed");
                    break;
                }
                Ok(Message::Ping(payload)) => {
                    if let Err(error) = socket.send(Message::Pong(payload)).await {
                        tracing::error!(?error, "Could not send TT2 WebSocket pong");
                        break;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(?error, "TT2 WebSocket receive error");
                    break;
                }
            }
        }
        self.connected.store(false, Ordering::Release);
        tracing::warn!("TT2 /player socket stopped; automatic reconnect is disabled");
    }

    pub async fn fetch_player(&self, player_token: &str) -> Result<PublicPlayerData, AppError> {
        if !self.is_connected() {
            return Err(AppError::ServiceUnavailable(
                "TT2 /player socket is not connected; restart the backend after checking TT2 configuration".to_string(),
            ));
        }
        let url = format!(
            "{}/player/data",
            self.config.rest_base_url.trim_end_matches('/')
        );
        let response = self
            .http
            .post(url)
            .header("API-Authenticate", &self.config.application_token)
            .json(&PlayerDataRequest {
                player_token,
                properties: &PLAYER_PROPERTIES,
            })
            .send()
            .await
            .map_err(|error| {
                AppError::ServiceUnavailable(format!("Could not reach TT2 player API: {error}"))
            })?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(%status, "TT2 player API rejected a request");
            let message = match status {
                StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                    "TT2 rejected the application or player token".to_string()
                }
                StatusCode::TOO_MANY_REQUESTS => {
                    "TT2 player API rate limit was reached".to_string()
                }
                _ => format!(
                    "TT2 player API returned {status}: {}",
                    sanitize_upstream_message(&body)
                ),
            };
            return if status == StatusCode::TOO_MANY_REQUESTS {
                Err(AppError::TooManyRequests(message))
            } else {
                Err(AppError::BadRequest(message))
            };
        }
        response.json::<PublicPlayerData>().await.map_err(|error| {
            AppError::BadRequest(format!("TT2 returned malformed player data: {error}"))
        })
    }
}

fn engine_url(config: &Tt2Config) -> Result<String, String> {
    let mut url = Url::parse(&config.socket_url).map_err(|error| error.to_string())?;
    url.set_path(config.socket_handshake_path.trim_end_matches('/'));
    url.query_pairs_mut()
        .clear()
        .append_pair("EIO", "4")
        .append_pair("transport", "websocket");
    Ok(url.to_string())
}

fn log_socket_event(payload: &str, connected: &AtomicBool) {
    let parsed: Value = match serde_json::from_str(payload) {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(?error, payload, "Could not parse TT2 Socket.IO event");
            return;
        }
    };
    let Some(values) = parsed.as_array() else {
        tracing::warn!(payload, "TT2 Socket.IO event was not an array");
        return;
    };
    let event = values.first().and_then(Value::as_str).unwrap_or("unknown");
    let data = values.get(1).cloned().unwrap_or(Value::Null);
    match event {
        "connected" => {
            connected.store(true, Ordering::Release);
            tracing::info!("TT2 /player connected event received");
        }
        "disconnect" => {
            connected.store(false, Ordering::Release);
            tracing::warn!(?data, "TT2 /player disconnect event received");
        }
        "error" => tracing::error!(?data, "TT2 /player error event received"),
        "connect_error" => {
            connected.store(false, Ordering::Release);
            tracing::error!(?data, "TT2 /player connect_error event received");
        }
        _ => tracing::debug!(event, ?data, "Ignoring unexpected TT2 /player event"),
    }
}

fn sanitize_upstream_message(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(200).collect()
}

#[derive(Serialize)]
struct PlayerDataRequest<'a> {
    player_token: &'a str,
    properties: &'a [&'static str],
}

#[derive(Debug, Deserialize)]
pub struct PublicPlayerData {
    pub player_code: String,
    pub player_raid_level: String,
    #[serde(default)]
    pub boosted_cards: Vec<PublicBoostedCard>,
    #[serde(default)]
    pub raid_research_tree: HashMap<String, Value>,
    #[serde(default)]
    pub raid_research_bonuses: HashMap<String, Value>,
    #[serde(default)]
    pub gemstone_research_tree_raid_bonuses: HashMap<String, Value>,
    #[serde(default)]
    pub equipment_set: Vec<String>,
    pub cards: Vec<PublicCard>,
}

#[derive(Debug, Deserialize)]
pub struct PublicBoostedCard {
    pub boost_level: u16,
    pub skill_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PublicCard {
    pub level: u16,
    pub skill_name: String,
}

impl PublicPlayerData {
    pub fn into_raid_data(self, title: f32) -> Result<PlayerRaidData, AppError> {
        let raid_level = self.player_raid_level.parse::<u16>().map_err(|_| {
            AppError::BadRequest("TT2 player_raid_level is not a valid u16".to_string())
        })?;
        let mut seen = HashSet::new();
        let mut raid_cards = HashMap::new();
        for card in self.cards {
            if !seen.insert(card.skill_name.clone()) {
                return Err(AppError::BadRequest(format!(
                    "TT2 returned duplicate card {}",
                    card.skill_name
                )));
            }
            raid_cards.insert(
                card.skill_name,
                Card {
                    lv: card.level,
                    num: 0,
                },
            );
        }
        let raw = PlayerData {
            player_stats: Default::default(),
            raid_stats: RaidStats {
                raid_level: raid_level.to_string(),
                raid_level_base_damage: raid_level.saturating_add(100).to_string(),
                ..Default::default()
            },
            artifacts: HashMap::new(),
            splash_stats: Default::default(),
            raid_cards,
            boosted_cards: self
                .boosted_cards
                .into_iter()
                .map(|card| BoostedCard {
                    boost_level: card.boost_level,
                    skill_name: card.skill_name,
                })
                .collect(),
            raid_card_research: numeric_map_to_strings(self.raid_research_bonuses)?,
            titan_cards: HashMap::new(),
            titan_research: numeric_map_to_strings(self.raid_research_tree)?,
            gem_research: numeric_map_to_strings(self.gemstone_research_tree_raid_bonuses)?,
            equip_set: self.equipment_set,
        };
        let mut cleaned = clean_data(&raw);
        cleaned.title = title;
        Ok(cleaned)
    }
}

fn numeric_map_to_strings(
    values: HashMap<String, Value>,
) -> Result<HashMap<String, String>, AppError> {
    values
        .into_iter()
        .map(|(key, value)| {
            if !value.is_number() {
                return Err(AppError::BadRequest(format!(
                    "TT2 research value {key} is not numeric"
                )));
            }
            Ok((key, value.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::cards::CardName;
    use strum::IntoEnumIterator;

    fn test_key() -> String {
        STANDARD.encode([7_u8; 32])
    }

    #[test]
    fn token_encryption_round_trip_uses_unique_nonces() {
        let cipher = TokenCipher::from_base64(&test_key()).unwrap();
        let (first, first_nonce) = cipher.encrypt("player-token").unwrap();
        let (second, second_nonce) = cipher.encrypt("player-token").unwrap();
        assert_ne!(first_nonce, second_nonce);
        assert_ne!(first, second);
        assert_eq!(
            cipher.decrypt(&first, &first_nonce).unwrap(),
            "player-token"
        );
    }

    #[test]
    fn wrong_key_cannot_decrypt_token() {
        let cipher = TokenCipher::from_base64(&test_key()).unwrap();
        let other = TokenCipher::from_base64(&STANDARD.encode([8_u8; 32])).unwrap();
        let (encrypted, nonce) = cipher.encrypt("player-token").unwrap();
        assert!(other.decrypt(&encrypted, &nonce).is_err());
    }

    #[test]
    fn public_player_data_maps_all_supported_raid_fields() {
        let cards = CardName::iter()
            .map(|card| PublicCard {
                level: 10,
                skill_name: card.id().to_string(),
            })
            .collect();
        let data = PublicPlayerData {
            player_code: "eqw9d34".to_string(),
            player_raid_level: "1395".to_string(),
            boosted_cards: vec![PublicBoostedCard {
                boost_level: 47,
                skill_name: "CosmicBarb".to_string(),
            }],
            raid_research_tree: HashMap::from([
                ("HeadDamage".to_string(), serde_json::json!(0.25)),
                ("ChestDamage".to_string(), serde_json::json!(0.17)),
                ("LimbDamage".to_string(), serde_json::json!(0.17)),
            ]),
            raid_research_bonuses: HashMap::from([
                ("RaidBaseDamage".to_string(), serde_json::json!(156)),
                ("BurstBaseDamage".to_string(), serde_json::json!(150)),
            ]),
            gemstone_research_tree_raid_bonuses: HashMap::from([
                ("Enemy1BurstBaseDamage".to_string(), serde_json::json!(10)),
                ("TorsoBaseDamage".to_string(), serde_json::json!(5)),
            ]),
            equipment_set: vec!["Jade", "Jukk", "Airforce", "Dancer", "RoseAnniversary"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            cards,
        };
        let cleaned = data.into_raid_data(1.5).unwrap();
        assert_eq!(cleaned.player_raid_level, 1395);
        assert_eq!(cleaned.player_raid_base_damage, 1495);
        assert_eq!(cleaned.card_list.len(), 44);
        assert_eq!(cleaned.title, 1.5);
        assert!(cleaned.raid_set.jade_anniversary);
        assert!(cleaned.raid_set.jukk_juggernaut);
        assert!(cleaned.raid_set.airforce_ace);
        assert!(cleaned.raid_set.dancer_venom);
        assert!(cleaned.raid_set.rose_anniversary);
        let boosted = cleaned
            .card_list
            .iter()
            .find(|card| card.card_id == CardName::ElectroZap)
            .unwrap();
        assert_eq!(boosted.level, 47);
        assert_eq!(cleaned.raid_card_research.base_damage, 156);
        assert_eq!(cleaned.gem_stone_research.burst_lojak_damage, 10);
    }
}
