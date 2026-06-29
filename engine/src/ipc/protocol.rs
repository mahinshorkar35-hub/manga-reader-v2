//! JSON-RPC 2.0 protocol types.
//!
//! Defines the request and response structures used for IPC
//! communication between the engine and its clients.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A JSON-RPC 2.0 request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0").
    pub jsonrpc: String,
    /// Method name to invoke.
    pub method: String,
    /// Request identifier (used to correlate responses).
    pub id: serde_json::Value,
    /// Optional parameters (object or array).
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Extract a string parameter by name.
    pub fn param_str(&self, name: &str) -> Result<String> {
        self.params
            .as_ref()
            .and_then(|p| p.get(name))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameter: {}", name))
    }

    /// Extract an i64 parameter by name.
    pub fn param_i64(&self, name: &str) -> Result<i64> {
        self.params
            .as_ref()
            .and_then(|p| p.get(name))
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid parameter: {}", name))
    }

    /// Parse the params object into a concrete type.
    pub fn parse_params<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        let params = self
            .params
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing request parameters"))?;
        Ok(serde_json::from_value(params.clone())?)
    }
}

/// A JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Option<serde_json::Value>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// -32700: Parse error
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".into(),
            data: None,
        }
    }

    /// -32600: Invalid request
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid request".into(),
            data: None,
        }
    }

    /// -32601: Method not found
    pub fn method_not_found() -> Self {
        Self {
            code: -32601,
            message: "Method not found".into(),
            data: None,
        }
    }

    /// -32603: Internal error
    pub fn internal_error() -> Self {
        Self {
            code: -32603,
            message: "Internal error".into(),
            data: None,
        }
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = message;
        self
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}
