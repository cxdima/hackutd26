use crate::config::ExecutionMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub session_id: Option<String>,
    pub leg_role: String,
    pub occurred_at: String,
    pub received_at: String,
    pub sequence: i64,
    pub policy_epoch: u64,
    pub execution_mode: ExecutionMode,
    pub data_origin: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub payload: Value,
}
