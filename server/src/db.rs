use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                username TEXT PRIMARY KEY,
                last_ip TEXT,
                last_login TEXT,
                last_logout TEXT
            )",
            [],
        )?;
        
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn update_login(&self, username: &str, ip: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO users (username, last_ip, last_login)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(username) DO UPDATE SET
                last_ip = excluded.last_ip,
                last_login = excluded.last_login",
            rusqlite::params![username, ip, now],
        )?;
        Ok(())
    }

    pub fn update_logout(&self, username: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE users SET last_logout = ?1 WHERE username = ?2",
            rusqlite::params![now, username],
        )?;
        Ok(())
    }
}
