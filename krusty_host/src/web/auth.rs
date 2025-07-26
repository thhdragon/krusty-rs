//! Authentication backend trait and in-memory implementation for Axum API
//! Step 3 of Plan 4.5: Modular, pluggable authentication
//!
//! AuthBackend trait is now defined in krusty_shared; InMemoryAuthBackend moved to krusty_shared::auth_backend

// Future: add FileAuthBackend, DatabaseAuthBackend, ExternalAuthBackend, etc.
