//! In-memory JWT token blacklist for revocation/rotation demo
//! (Plan 4.5 Step 3: Token revocation/rotation)
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TokenBlacklist {
    inner: Arc<Mutex<HashSet<String>>>,
}

impl TokenBlacklist {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashSet::new())) }
    }
    pub fn insert(&self, token: String) {
        self.inner.lock().unwrap().insert(token);
    }
    pub fn contains(&self, token: &str) -> bool {
        self.inner.lock().unwrap().contains(token)
    }
}
