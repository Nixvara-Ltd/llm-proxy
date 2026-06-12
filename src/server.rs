use axum::{
    extract::{State, Json},
    routing::post,
    Router,
    response::{IntoResponse, sse::{Sse, Event}},
    http::StatusCode,
};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::StreamExt;
use std::convert::Infallible;
use crate::db::DbStore;
use crate::events::ProxyEvent;
use tokio::sync::mpsc::Sender;
use genai::{Client, chat::{ChatRequest, ChatMessage, ChatStreamEvent}};

#[derive(Clone)]
pub struct AppState {
    pub db: DbStore,
    pub genai_client: Arc<Client>,
    pub tx: Sender<ProxyEvent>,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    #[allow(dead_code)]
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
}

pub async fn start_server(state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("Server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}

async fn chat_completions(
    State(state): State<AppState>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    tracing::info!("Received chat completion request for model: {}", payload.model);
    
    let mut chat_req = ChatRequest::new(vec![]);
    for msg in &payload.messages {
        let genai_msg = match msg.role.as_str() {
            "user" => ChatMessage::user(msg.content.clone()),
            "system" => ChatMessage::system(msg.content.clone()),
            "assistant" => ChatMessage::assistant(msg.content.clone()),
            _ => ChatMessage::user(msg.content.clone()),
        };
        chat_req = chat_req.append_message(genai_msg);
    }
    
    let session_id = uuid::Uuid::new_v4().to_string();
    
    // Phase 3: Routing Logic & Cost-Cap Kill Switch
    let route_decision = match crate::routing::determine_route(&state.db, &payload.model) {
        Ok(decision) => decision,
        Err(e) => {
            tracing::error!("Request blocked by routing engine: {}", e);
            return Err((StatusCode::TOO_MANY_REQUESTS, e.to_string()));
        }
    };
    
    let model = route_decision.final_model;
    
    // Estimate tokens roughly by string length for now
    let estimated_tokens = (payload.messages.iter().map(|m| m.content.len()).sum::<usize>() / 4) as i64;
    let estimated_cost = crate::routing::estimate_cost(&model, estimated_tokens);
    
    let prompt_summary = payload.messages.first()
        .map(|m| m.content.chars().take(100).collect::<String>())
        .unwrap_or_default();
    
    // Log it
    let _ = state.db.log_request(&session_id, &model, estimated_tokens, estimated_cost);
    let _ = state.tx.send(ProxyEvent {
        session_id: session_id.clone(),
        model: model.clone(),
        tokens: estimated_tokens,
        cost: estimated_cost,
        prompt_summary,
    }).await;
    
    if payload.stream.unwrap_or(false) {
        // Stream response
        let chat_stream_res = state.genai_client.exec_chat_stream(&model, chat_req.clone(), None).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        let stream_session_id = session_id.clone();
        let stream_model = model.clone();
        let stream = chat_stream_res.stream.map(move |event| {
            match event {
                Ok(ChatStreamEvent::Chunk(chunk)) => {
                    let content = chunk.content;
                    let chunk_json = serde_json::json!({
                        "id": format!("chatcmpl-{}", stream_session_id),
                        "object": "chat.completion.chunk",
                        "model": &stream_model,
                        "choices": [{
                            "index": 0,
                            "delta": {
                                "content": content
                            }
                        }]
                    });
                    Ok::<_, Infallible>(Event::default().data(chunk_json.to_string()))
                },
                _ => {
                    // Send [DONE] or ignore
                    Ok::<_, Infallible>(Event::default().data("[DONE]"))
                }
            }
        });
        
        Ok(Sse::new(stream).into_response())
    } else {
        // Non-stream response
        let chat_res = state.genai_client.exec_chat(&model, chat_req.clone(), None).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            
        let content = chat_res.content_text_as_str().unwrap_or("").to_string();
        
        let dummy_response = serde_json::json!({
            "id": format!("chatcmpl-{}", session_id),
            "object": "chat.completion",
            "model": &model,
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": content,
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 0,
                "completion_tokens": 0,
                "total_tokens": 0
            }
        });

        Ok(Json(dummy_response).into_response())
    }
}
