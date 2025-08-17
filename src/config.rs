//! Tracing configuration module
//!
//! Contains the TraceConfig struct, Environment enum, and ConfigPlugin implementation.

use serde::{Deserialize, Serialize};
use tyl_config::{ConfigPlugin, ConfigResult};
use tyl_errors::TylError;

/// Tracing configuration with TYL config integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceConfig {
    pub service_name: String,
    pub environment: Environment,
    pub sampling_rate: f64,
    pub max_spans: usize,
}

/// Runtime environment detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl TraceConfig {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            environment: Environment::from_env(),
            sampling_rate: 1.0,
            max_spans: 1000,
        }
    }

    pub fn with_environment(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_sampling_rate(mut self, rate: f64) -> Self {
        self.sampling_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_max_spans(mut self, max_spans: usize) -> Self {
        self.max_spans = max_spans;
        self
    }
}

impl ConfigPlugin for TraceConfig {
    fn name(&self) -> &'static str {
        "tracing"
    }

    fn env_prefix(&self) -> &'static str {
        "TRACE"
    }

    fn validate(&self) -> ConfigResult<()> {
        if self.service_name.is_empty() {
            return Err(TylError::validation("service_name", "cannot be empty"));
        }
        if !(0.0..=1.0).contains(&self.sampling_rate) {
            return Err(TylError::validation(
                "sampling_rate",
                "must be between 0.0 and 1.0",
            ));
        }
        if self.max_spans == 0 {
            return Err(TylError::validation("max_spans", "must be greater than 0"));
        }
        Ok(())
    }

    fn from_env(&self) -> ConfigResult<Self> {
        let mut config = Self::new("app");
        config.merge_env()?;
        Ok(config)
    }

    fn merge_env(&mut self) -> ConfigResult<()> {
        // TYL_SERVICE_NAME or SERVICE_NAME
        if let Ok(service_name) =
            std::env::var("TYL_SERVICE_NAME").or_else(|_| std::env::var("SERVICE_NAME"))
        {
            self.service_name = service_name;
        }

        // TYL_TRACE_SAMPLING_RATE or TRACE_SAMPLING_RATE
        if let Ok(rate_str) = std::env::var("TYL_TRACE_SAMPLING_RATE")
            .or_else(|_| std::env::var("TRACE_SAMPLING_RATE"))
        {
            self.sampling_rate = rate_str
                .parse::<f64>()
                .map_err(|e| TylError::configuration(format!("invalid sampling rate: {}", e)))?
                .clamp(0.0, 1.0);
        }

        // TYL_TRACE_MAX_SPANS or TRACE_MAX_SPANS
        if let Ok(max_str) =
            std::env::var("TYL_TRACE_MAX_SPANS").or_else(|_| std::env::var("TRACE_MAX_SPANS"))
        {
            self.max_spans = max_str
                .parse::<usize>()
                .map_err(|e| TylError::configuration(format!("invalid max spans: {}", e)))?;
        }

        // TYL_ENVIRONMENT or ENVIRONMENT
        if let Ok(env_str) =
            std::env::var("TYL_ENVIRONMENT").or_else(|_| std::env::var("ENVIRONMENT"))
        {
            self.environment = match env_str.to_lowercase().as_str() {
                "development" | "dev" => Environment::Development,
                "production" | "prod" => Environment::Production,
                "test" | "testing" => Environment::Testing,
                _ => {
                    return Err(TylError::configuration(format!(
                        "invalid environment: {}",
                        env_str
                    )))
                }
            };
        }

        Ok(())
    }
}

impl Environment {
    pub fn from_env() -> Self {
        match std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "prod" | "production" => Environment::Production,
            "test" | "testing" => Environment::Testing,
            _ => Environment::Development,
        }
    }
}
