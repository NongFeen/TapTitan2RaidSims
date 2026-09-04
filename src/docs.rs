use axum::Router;
use utoipa::{
    Modify, OpenApi,
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
};
use utoipa_swagger_ui::SwaggerUi;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "internal_api_key",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("x-internal-api-key"))),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Server status"),
        (name = "players", description = "Player rosters and stats"),
        (name = "jobs", description = "Simulation jobs and batches"),
        (name = "recommendations", description = "Deck recommendations"),
        (name = "raids", description = "Sims boss and live raid state"),
        (name = "taptitan", description = "Static TT2 card data"),
        (name = "internal", description = "Requires the x-internal-api-key header; disabled entirely when INTERNAL_API_ENABLED=false"),
    ),
    paths(
        crate::routes::health::handler,
        crate::routes::players::list,
        crate::routes::players::get,
        crate::routes::players::current_stats,
        crate::routes::players::attack_log,
        crate::routes::players::update_token,
        crate::routes::players::clear_token,
        crate::routes::players::fetch_tt2_stats,
        crate::routes::players::tt2_status,
        crate::routes::players::tt2_clan_status,
        crate::routes::players::fetch_tt2_clan_stats,
        crate::routes::jobs::create,
        crate::routes::jobs::create_batch,
        crate::routes::jobs::get_batch,
        crate::routes::jobs::get,
        crate::routes::jobs::list_for_player,
        crate::routes::jobs::retry,
        crate::routes::raid_cycle::current,
        crate::routes::recommendations::generate_for_player,
        crate::routes::recommendations::current_for_player,
        crate::routes::simulation_debug::run,
        crate::routes::taptitan::get_all_card_definitions,
        crate::routes::raids::update_current_boss,
        crate::routes::raids::current,
        crate::routes::raids::live_from_attack,
        crate::routes::raids::live_current_boss_stream,
        crate::routes::raids::live_attacking_players,
        crate::routes::raids::live_attacking_players_stream,
    ),
    components(schemas(
        crate::routes::health::HealthResponse,
        crate::models::app::PlayerSummary,
        crate::models::app::PlayerDetail,
        crate::models::app::UpdateAutoSimsRequest,
        crate::models::app::UpdatePlayerTokenRequest,
        crate::models::app::Tt2PlayerStatus,
        crate::models::app::Tt2ClanStatus,
        crate::models::app::Tt2ClanFetchResult,
        crate::models::app::PlayerStatsVersion,
        crate::models::app::PlayerAttackLogEntry,
        crate::models::app::SimulationJobView,
        crate::models::app::CreateSimulationJobRequest,
        crate::models::app::CurrentBossUpdateRequest,
        crate::models::app::RaidEventAccepted,
        crate::models::app::CurrentBossView,
        crate::models::app::LiveAttackBossView,
        crate::models::app::AreaBonusView,
        crate::models::app::LiveBossDisplayPart,
        crate::models::app::LiveAttackingCard,
        crate::models::app::LiveAttackingPlayer,
        crate::models::app::RecommendationView,
        crate::models::db_enums::TokenStatus,
        crate::models::db_enums::JobStatus,
        crate::dtos::cards::CardDefinitionDto,
        crate::models::cards::CardType,
        crate::models::cards::CardName,
        crate::models::boss::BossPartName,
        crate::models::boss::PartState,
        crate::models::boss::CurseType,
        crate::models::boss::BossName,
        crate::models::boss::GlobalRaidModifier,
        crate::models::boss::DamageResult,
        crate::models::boss::BossPart,
        crate::models::boss::Boss,
        crate::models::damage_source::DamageSource,
        crate::services::taptitan::sim_service::SimulationPhase,
        crate::services::taptitan::sim_service::SimDeckResult,
        crate::services::taptitan::sim_service::SimPatternResult,
        crate::services::taptitan::sim_service::SimCardDamageResult,
        crate::routes::jobs::JobAccepted,
        crate::routes::jobs::CreateSimulationBatchRequest,
        crate::routes::jobs::SimulationBatchAccepted,
        crate::routes::jobs::SimulationBatchView,
        crate::routes::raid_cycle::RaidCycleView,
        crate::routes::recommendations::GenerateRecommendationRequest,
        crate::routes::recommendations::GenerateRecommendationResponse,
        crate::routes::simulation_debug::DebugSimulationRequest,
        crate::routes::simulation_debug::DebugSimulationResponse,
    )),
    info(
        title = "Feen TT2 raid sim backend",
        description = "Routes under /internal require the x-internal-api-key header and can be fully disabled via INTERNAL_API_ENABLED=false.",
        version = env!("CARGO_PKG_VERSION"),
    )
)]
struct ApiDoc;

/// Mounts Swagger UI at `/docs` (spec served at `/api-docs/openapi.json`).
/// Only call this when `SWAGGER_UI_ENABLED=true` -- see `Config::swagger_ui_enabled`.
pub fn swagger_router() -> Router {
    Router::new().merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
