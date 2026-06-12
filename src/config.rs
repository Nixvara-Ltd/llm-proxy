use crate::db::DbStore;
use std::env;

pub fn inject_environment_keys(db: &DbStore) -> anyhow::Result<()> {
    // Optionally load from .env file if present
    let _ = dotenvy::dotenv();

    let providers = vec![
        ("openai", "OPENAI_API_KEY"),
        ("anthropic", "ANTHROPIC_API_KEY"),
        ("gemini", "GEMINI_API_KEY"),
        ("groq", "GROQ_API_KEY"),
    ];

    for (provider_name, env_var_name) in providers {
        let db_key = db.get_api_key(provider_name).unwrap_or(None);

        if let Some(key) = db_key {
            if !key.is_empty() {
                env::set_var(env_var_name, &key);
                tracing::info!("Loaded {} API key from DB", provider_name);
                continue;
            }
        }

        // Fallback to environment
        let env_key = env::var(env_var_name).unwrap_or_default();
        if !env_key.is_empty() {
            db.set_api_key(provider_name, &env_key)?;
            tracing::info!("Injected {} API key from environment to DB", provider_name);
        }
    }

    Ok(())
}
