// =============================================================================
// thaid — Model Manager (lazy loading + idle unload)
// =============================================================================

use crate::config::ThaidConfig;
use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Tracks the current state of a loaded model.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelState {
    /// No model is loaded. Memory footprint: ~0 MB.
    Unloaded,
    /// Model is being pulled/loaded into Ollama.
    Loading,
    /// Model is loaded and ready to serve queries.
    Ready { name: String },
}

/// Manages the Ollama model lifecycle: lazy loading and idle unloading.
/// All public methods are async and safe to call from concurrent tasks.
pub struct ModelManager {
    ollama_url:     String,
    default_model:  String,
    idle_timeout:   Duration,
    state:          Arc<RwLock<ModelState>>,
    last_used:      Arc<RwLock<Option<Instant>>>,
    http:           Client,
}

impl ModelManager {
    pub fn new(
        ollama_url:    String,
        default_model: String,
        idle_timeout:  Duration,
    ) -> Self {
        Self {
            ollama_url,
            default_model,
            idle_timeout,
            state:     Arc::new(RwLock::new(ModelState::Unloaded)),
            last_used: Arc::new(RwLock::new(None)),
            http:      Client::builder()
                .timeout(Duration::from_secs(300)) // long timeout for model pulls
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Ensure a model is loaded, loading it lazily if needed.
    /// Returns the model name that is ready.
    pub async fn ensure_loaded(&self, requested_model: Option<&str>) -> Result<String> {
        let model_name = requested_model.unwrap_or(&self.default_model).to_string();

        // Fast path: already loaded
        {
            let state = self.state.read().await;
            if let ModelState::Ready { name } = &*state {
                if name == &model_name {
                    debug!(model = %model_name, "Model already loaded (fast path)");
                    self.update_last_used().await;
                    return Ok(name.clone());
                }
            }
        }

        // Need to load or switch model
        {
            let mut state = self.state.write().await;
            // Double-check under write lock
            if let ModelState::Ready { name } = &*state {
                if name == &model_name {
                    self.update_last_used().await;
                    return Ok(name.clone());
                }
            }

            info!(model = %model_name, "Loading AI model (lazy)...");
            *state = ModelState::Loading;
        }

        // Pull/load the model (this may take a while for large models)
        match self.ollama_pull(&model_name).await {
            Ok(()) => {
                info!(model = %model_name, "Model loaded successfully");
                let mut state = self.state.write().await;
                *state = ModelState::Ready { name: model_name.clone() };
                self.update_last_used().await;
                Ok(model_name)
            }
            Err(e) => {
                warn!(model = %model_name, error = %e, "Failed to load model");
                let mut state = self.state.write().await;
                *state = ModelState::Unloaded;
                Err(e)
            }
        }
    }

    /// Send a query to the loaded model and return the full response.
    pub async fn query(&self, prompt: &str, model: Option<&str>) -> Result<String> {
        let model_name = self.ensure_loaded(model).await?;

        #[derive(Serialize)]
        struct OllamaRequest<'a> {
            model:  &'a str,
            prompt: &'a str,
            stream: bool,
        }

        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
            done:     bool,
        }

        let request = OllamaRequest {
            model:  &model_name,
            prompt,
            stream: false,
        };

        let resp = self.http
            .post(format!("{}/api/generate", self.ollama_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send query to Ollama")?
            .error_for_status()
            .context("Ollama returned an error status")?
            .json::<OllamaResponse>()
            .await
            .context("Failed to parse Ollama response")?;

        if !resp.done {
            warn!("Ollama indicated generation was not complete");
        }

        Ok(resp.response)
    }

    /// Unload the current model, freeing RAM.
    pub async fn unload(&self) {
        let current_model = {
            let state = self.state.read().await;
            match &*state {
                ModelState::Ready { name } => Some(name.clone()),
                _ => None,
            }
        };

        if let Some(model) = current_model {
            info!(model = %model, "Unloading idle AI model");
            // Call Ollama API to evict the model from memory
            let _ = self.http
                .post(format!("{}/api/generate", self.ollama_url))
                .json(&serde_json::json!({
                    "model":     model,
                    "prompt":    "",
                    "keep_alive": 0  // 0 = unload immediately
                }))
                .send()
                .await;

            let mut state = self.state.write().await;
            *state = ModelState::Unloaded;
            info!("Model unloaded — RAM freed");
        }
    }

    /// Returns the current model state (for D-Bus status queries).
    pub async fn get_state(&self) -> ModelState {
        self.state.read().await.clone()
    }

    /// Background task: checks periodically for idle timeout and unloads.
    pub async fn run_idle_watcher(self: Arc<Self>) {
        let check_interval = Duration::from_secs(60); // check every minute
        loop {
            tokio::time::sleep(check_interval).await;

            let last = *self.last_used.read().await;
            if let Some(t) = last {
                if t.elapsed() >= self.idle_timeout {
                    self.unload().await;
                    // Reset last_used so we don't trigger again immediately
                    *self.last_used.write().await = None;
                }
            }
        }
    }

    // ---- Private helpers ---------------------------------------------------

    async fn update_last_used(&self) {
        *self.last_used.write().await = Some(Instant::now());
    }

    /// Pull a model from the Ollama model registry (or local cache).
    async fn ollama_pull(&self, model: &str) -> Result<()> {
        #[derive(Serialize)]
        struct PullRequest<'a> {
            name: &'a str,
            stream: bool,
        }

        self.http
            .post(format!("{}/api/pull", self.ollama_url))
            .json(&PullRequest {
                name: model,
                stream: false,
            })
            .send()
            .await
            .context("Pull request to Ollama failed")?
            .error_for_status()
            .context("Ollama pull returned error status")?;

        Ok(())
    }
}
