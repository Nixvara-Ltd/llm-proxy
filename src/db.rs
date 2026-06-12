use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::path::Path;
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct DbStore {
    conn: Arc<Mutex<Connection>>,
}

impl DbStore {
    pub fn new<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let conn = Connection::open(path).context("Failed to open local sqlite database")?;
        
        let store = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        
        store.init_schema()?;
        
        Ok(store)
    }

    fn init_schema(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS api_keys (
                provider TEXT PRIMARY KEY,
                key TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS request_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                model_name TEXT NOT NULL,
                tokens_used INTEGER NOT NULL,
                cost_est REAL NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        Ok(())
    }

    // TODO: Implement strong local    // Simple dummy encryption for Phase 1
    #[allow(dead_code)]
    fn encrypt(plain: &str) -> String {
        format!("ENCRYPTED:{}", plain)
    }

    #[allow(dead_code)]
    fn decrypt(cipher: &str) -> String {
        cipher.to_string()
    }

    pub fn set_api_key(&self, provider: &str, key: &str) -> anyhow::Result<()> {
        let encrypted_key = Self::encrypt(key);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO api_keys (provider, key) VALUES (?1, ?2)
             ON CONFLICT(provider) DO UPDATE SET key=excluded.key",
            [provider, &encrypted_key],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_api_key(&self, provider: &str) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key FROM api_keys WHERE provider = ?1")?;
        let mut rows = stmt.query([provider])?;
        if let Some(row) = rows.next()? {
            let encrypted_key: String = row.get(0)?;
            Ok(Some(Self::decrypt(&encrypted_key)))
        } else {
            Ok(None)
        }
    }

    pub fn log_request(&self, session_id: &str, model_name: &str, tokens_used: i64, cost_est: f64) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO request_logs (session_id, model_name, tokens_used, cost_est)
             VALUES (?1, ?2, ?3, ?4)",
            (session_id, model_name, tokens_used, cost_est),
        )?;
        Ok(())
    }

    pub fn get_daily_cost(&self) -> anyhow::Result<f64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT SUM(cost_est) FROM request_logs 
             WHERE date(timestamp) = date('now')"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let total: Option<f64> = row.get(0)?;
            Ok(total.unwrap_or(0.0))
        } else {
            Ok(0.0)
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> anyhow::Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        if let Some(row) = rows.next()? {
            let value: String = row.get(0)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn get_daily_limit(&self) -> anyhow::Result<f64> {
        let limit_str = self.get_setting("daily_limit")?.unwrap_or_else(|| "5.0".to_string());
        let limit = limit_str.parse::<f64>().unwrap_or(5.0);
        Ok(limit)
    }

    pub fn set_daily_limit(&self, limit: f64) -> anyhow::Result<()> {
        self.set_setting("daily_limit", &limit.to_string())
    }
}
