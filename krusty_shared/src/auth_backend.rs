use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::AuthBackend;

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
