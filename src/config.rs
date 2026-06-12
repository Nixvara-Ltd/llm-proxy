use crate::db::DbStore;
use std::env;

pub fn inject_environment_keys(db: &DbStore) -> anyhow::Result<()> {
    // Optionally load from .env file if present
    let _ = dotenvy::dotenv();

    let providers = vec![
        ("openai", "OPENAI_API_KEY", ""),
        ("anthropic", "ANTHROPIC_API_KEY", ""),
        ("gemini", "GEMINI_API_KEY", ""),
        ("groq", "GROQ_API_KEY", ""),
    ];

    for (provider_name, env_var_name, fallback_key) in providers {
        // First try to read from actual environment
        let mut key = env::var(env_var_name).unwrap_or_default();
        
        // If not in environment, use the hardcoded fallback
        if key.is_empty() && !fallback_key.is_empty() {
            key = fallback_key.to_string();
            // Force it into the environment so `genai` can automatically pick it up!
            env::set_var(env_var_name, &key);
        }

        if !key.is_empty() {
            db.set_api_key(provider_name, &key)?;
            tracing::info!("Injected {} API key", provider_name);
        }
    }

    Ok(())
}
