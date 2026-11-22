//! Scenario definition and configuration for the SII simulator (`docs/simulator-spec.md` §2).

use crate::error::{Result, SimError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Standard simulation scenarios for SII endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Scenario {
    /// Everything is approved and accepted by the SII (default).
    #[default]
    Aceptado,
    /// Envelope processing completed with rejected documents (`EPR` with `RECHAZADOS > 0`).
    Rechazado,
    /// Envelope was received and is currently in the processing queue (`REC`).
    EnProceso,
    /// Authentication token is invalid, rejected or expired (`ESTADO 11` or `STATUS 5`).
    TokenInvalido,
    /// Envelope upload is rejected at the transport / carátula gateway (`STATUS 1`).
    UploadRechazado,
    /// Seed acquisition returns an internal SII database/service error (`ESTADO -1`).
    SemillaError,
}

impl Scenario {
    /// Returns the static canonical string representation of the scenario.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Aceptado => "aceptado",
            Self::Rechazado => "rechazado",
            Self::EnProceso => "en_proceso",
            Self::TokenInvalido => "token_invalido",
            Self::UploadRechazado => "upload_rechazado",
            Self::SemillaError => "semilla_error",
        }
    }
}

impl fmt::Display for Scenario {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Scenario {
    type Err = String;

    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        let normalized = text.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "aceptado" | "aprobado" | "approve" | "approved" | "ok" => Ok(Self::Aceptado),
            "rechazado" | "reject" | "rejected" => Ok(Self::Rechazado),
            "en_proceso" | "in_process" | "pending" | "proceso" => Ok(Self::EnProceso),
            "token_invalido" | "invalid_token" | "token_error" => Ok(Self::TokenInvalido),
            "upload_rechazado" | "reject_upload" | "upload_error" => Ok(Self::UploadRechazado),
            "semilla_error" | "seed_error" => Ok(Self::SemillaError),
            other => Err(format!(
                "escenario desconocido '{other}'; permitidos: aceptado, rechazado, en_proceso, token_invalido, upload_rechazado, semilla_error"
            )),
        }
    }
}

/// Custom status configuration for an individual Track ID query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackStatusConfig {
    /// SII status code string (e.g. "EPR", "REC", "SOK", "RCT").
    pub estado: String,
    /// Explanatory description from the SII.
    pub glosa: String,
    /// Count of accepted documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aceptados: Option<u32>,
    /// Count of rejected documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rechazados: Option<u32>,
    /// Count of repaired documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reparos: Option<u32>,
    /// DTE document type code (e.g. 33).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tipo_docto: Option<u16>,
}

impl TrackStatusConfig {
    /// Creates an accepted status configuration with 1 accepted document.
    #[must_use]
    pub fn aceptado(glosa: impl Into<String>) -> Self {
        Self {
            estado: "EPR".to_string(),
            glosa: glosa.into(),
            aceptados: Some(1),
            rechazados: Some(0),
            reparos: Some(0),
            tipo_docto: Some(33),
        }
    }

    /// Creates a rejected status configuration with 1 rejected document.
    #[must_use]
    pub fn rechazado(glosa: impl Into<String>) -> Self {
        Self {
            estado: "EPR".to_string(),
            glosa: glosa.into(),
            aceptados: Some(0),
            rechazados: Some(1),
            reparos: Some(0),
            tipo_docto: Some(33),
        }
    }

    /// Creates an in-process status configuration (`REC`).
    #[must_use]
    pub fn en_proceso(glosa: impl Into<String>) -> Self {
        Self {
            estado: "REC".to_string(),
            glosa: glosa.into(),
            aceptados: None,
            rechazados: None,
            reparos: None,
            tipo_docto: None,
        }
    }
}

/// Data-driven configuration for the SII simulator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// Default scenario when no specific Track ID override applies.
    pub default_scenario: Scenario,
    /// Optional fixed seed string to return on `CrSeed.jws`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_seed: Option<String>,
    /// Optional fixed token string to return on `GetTokenFromSeed.jws`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_token: Option<String>,
    /// Specific status responses keyed by Track ID string.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub track_statuses: HashMap<String, TrackStatusConfig>,
}

impl Default for ScenarioConfig {
    fn default() -> Self {
        Self {
            default_scenario: Scenario::Aceptado,
            fixed_seed: None,
            fixed_token: None,
            track_statuses: HashMap::new(),
        }
    }
}

impl ScenarioConfig {
    /// Creates a new configuration with the specified default scenario.
    #[must_use]
    pub fn new(scenario: Scenario) -> Self {
        Self {
            default_scenario: scenario,
            fixed_seed: None,
            fixed_token: None,
            track_statuses: HashMap::new(),
        }
    }

    /// Sets a fixed authentication seed.
    #[must_use]
    pub fn with_fixed_seed(mut self, seed: impl Into<String>) -> Self {
        self.fixed_seed = Some(seed.into());
        self
    }

    /// Sets a fixed session token.
    #[must_use]
    pub fn with_fixed_token(mut self, token: impl Into<String>) -> Self {
        self.fixed_token = Some(token.into());
        self
    }

    /// Registers a custom status response for a specific Track ID.
    #[must_use]
    pub fn with_track_status(
        mut self,
        track_id: impl Into<String>,
        status: TrackStatusConfig,
    ) -> Self {
        self.track_statuses.insert(track_id.into(), status);
        self
    }

    /// Deserializes a `ScenarioConfig` from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::Json`] if parsing fails.
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(SimError::from)
    }

    /// Serializes this `ScenarioConfig` into a formatted JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::Json`] if serialization fails.
    pub fn to_json_pretty(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(SimError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_from_str() {
        assert_eq!("aceptado".parse::<Scenario>(), Ok(Scenario::Aceptado));
        assert_eq!("approve".parse::<Scenario>(), Ok(Scenario::Aceptado));
        assert_eq!("rechazado".parse::<Scenario>(), Ok(Scenario::Rechazado));
        assert_eq!("reject".parse::<Scenario>(), Ok(Scenario::Rechazado));
        assert_eq!("en-proceso".parse::<Scenario>(), Ok(Scenario::EnProceso));
        assert_eq!("in_process".parse::<Scenario>(), Ok(Scenario::EnProceso));
        assert_eq!(
            "token-invalido".parse::<Scenario>(),
            Ok(Scenario::TokenInvalido)
        );
        assert_eq!(
            "upload-rechazado".parse::<Scenario>(),
            Ok(Scenario::UploadRechazado)
        );
        assert_eq!(
            "semilla-error".parse::<Scenario>(),
            Ok(Scenario::SemillaError)
        );
        assert!("desconocido".parse::<Scenario>().is_err());
    }

    #[test]
    fn test_scenario_config_json_roundtrip() {
        let mut config = ScenarioConfig::new(Scenario::Aceptado)
            .with_fixed_seed("001234567890")
            .with_fixed_token("SIM_TOKEN_ABC123");
        config.track_statuses.insert(
            "998877".to_string(),
            TrackStatusConfig::rechazado("Rechazo por RUT"),
        );

        let json = config.to_json_pretty().expect("serialize json");
        let parsed = ScenarioConfig::from_json(&json).expect("deserialize json");
        assert_eq!(config, parsed);
    }
}
