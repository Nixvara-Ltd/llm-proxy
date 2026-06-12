use crate::db::DbStore;

pub struct RouteDecision {
    pub final_model: String,
}

pub fn determine_route(db: &DbStore, requested_model: &str) -> anyhow::Result<RouteDecision> {
    // 1. Check Cost-Cap kill switch
    let daily_cost = db.get_daily_cost().unwrap_or(0.0);
    let limit = db.get_daily_limit().unwrap_or(5.0);
    
    if daily_cost >= limit {
        tracing::warn!("Cost cap exceeded! Current: ${}, Limit: ${}", daily_cost, limit);
        return Err(anyhow::anyhow!("Cost-Cap Kill Switch activated. Daily limit of ${} reached.", limit));
    }

    // 2. Routing logic
    // If asking for expensive model, fall back to cheaper if approaching limit.
    let mut final_model = requested_model.to_string();
    
    if requested_model == "gpt-4" || requested_model.starts_with("gpt-4o") || requested_model == "claude-3-opus-20240229" {
        if daily_cost > (limit * 0.8) {
            tracing::info!("Approaching daily limit (${}/${}). Routing expensive {} to cheaper gemini-2.0-flash", daily_cost, limit, requested_model);
            final_model = "gemini-2.0-flash".to_string();
        }
    }

    Ok(RouteDecision {
        final_model,
    })
}

pub fn estimate_cost(model: &str, tokens: i64) -> f64 {
    let cost_per_1k = match model {
        "gpt-4" | "gpt-4o" | "gpt-4o-2024-05-13" => 0.01,
        "claude-3-opus-20240229" => 0.015,
        "gemini-2.0-flash" | "claude-3-haiku-20240307" => 0.00025,
        _ => 0.002,
    };
    (tokens as f64 / 1000.0) * cost_per_1k
}
