//! Authentication backend trait and in-memory implementation for Axum API
//! Step 3 of Plan 4.5: Modular, pluggable authentication

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[async_trait]
pub trait AuthBackend: Send + Sync + 'static {
    /// Validate credentials. Returns true if valid.
    async fn validate(&self, username: &str, password: &str) -> bool;
}

/// In-memory user store for demo/testing
pub struct InMemoryAuthBackend {
    users: Arc<Mutex<HashMap<String, String>>>,
}

impl InMemoryAuthBackend {
    pub fn new(users: Arc<Mutex<HashMap<String, String>>>) -> Self {
        Self { users }
    }
}

#[async_trait]
impl AuthBackend for InMemoryAuthBackend {
    async fn validate(&self, username: &str, password: &str) -> bool {
        let users = self.users.lock().unwrap();
        users.get(username).map(|pw| pw == password).unwrap_or(false)
    }
}

// Future: add FileAuthBackend, DatabaseAuthBackend, ExternalAuthBackend, etc.
