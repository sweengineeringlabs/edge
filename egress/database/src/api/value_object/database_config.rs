//! Database connection configuration.

use serde::{Deserialize, Serialize};

/// Supported database backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    Postgres,
    Mysql,
    Sqlite,
    Memory,
}

/// Configuration for a database connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub db_type: DatabaseType,
    pub url: Option<String>,
    pub max_connections: u32,
    pub connect_timeout_secs: u64,
    pub idle_timeout_secs: Option<u64>,
}

impl DatabaseConfig {
    pub fn memory() -> Self {
        Self { db_type: DatabaseType::Memory, url: None, max_connections: 1, connect_timeout_secs: 0, idle_timeout_secs: None }
    }

    pub fn postgres(url: impl Into<String>) -> Self {
        Self { db_type: DatabaseType::Postgres, url: Some(url.into()), max_connections: 10, connect_timeout_secs: 30, idle_timeout_secs: Some(600) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// @covers: memory
    #[test]
    fn test_memory_creates_memory_database_config() {
        let cfg = DatabaseConfig::memory();
        assert_eq!(cfg.db_type, DatabaseType::Memory);
        assert!(cfg.url.is_none());
    }

    /// @covers: postgres
    #[test]
    fn test_postgres_creates_postgres_config_with_url() {
        let cfg = DatabaseConfig::postgres("postgres://localhost/db");
        assert_eq!(cfg.db_type, DatabaseType::Postgres);
        assert_eq!(cfg.url, Some("postgres://localhost/db".to_string()));
    }
}
