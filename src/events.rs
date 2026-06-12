#[derive(Debug, Clone)]
pub struct ProxyEvent {
    pub session_id: String,
    pub model: String,
    pub tokens: i64,
    pub cost: f64,
    pub prompt_summary: String,
}
