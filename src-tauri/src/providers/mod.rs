//! First-party provider adapters, catalog, probes, chat, and HTTP transport.

pub mod adapter;
pub mod catalog;
pub mod chat;
pub mod connection;
pub mod custom;
pub mod first_party;
pub mod probe;
pub mod transport;

pub use adapter::{Protocol, ProviderAdapter};
pub use catalog::{load_bundled_catalog, select_default_model, BUNDLED_CATALOG_JSON};
pub use chat::{
    complete_chat_turn, openai_assistant_text_body, openai_assistant_tool_body, AssistantTurn,
    ChatMessage, ToolCall,
};
pub use connection::{
    connect_first_party, create_custom_profile, get_ready_provider, list_catalog_cards,
    list_hero_cards, set_ready_provider_selection, ConnectResult, HeroCard, ProviderConnectionDto,
};
pub use first_party::{first_party_adapter, hero_provider_ids};
pub use probe::{
    probe_first_party, probe_with_model_fallback, run_probe, FixtureTransport, ProbeResult,
    SequenceTransport,
};
pub use transport::{FixtureHttpTransport, HttpTransport, LiveHttpTransport, live_http_enabled};
