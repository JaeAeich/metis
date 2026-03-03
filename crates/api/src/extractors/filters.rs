use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

use crate::repositories::{RunFilter, State};

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct RunFilterParams {
    #[param(example = "RUNNING")]
    pub state: Option<String>,

    #[param(example = "user123")]
    pub user_id: Option<String>,

    #[param(example = "project")]
    pub tag_key: Option<String>,

    #[param(example = "genomics")]
    pub tag_value: Option<String>,

    #[param(example = "2024-01-15T10:30:00Z")]
    pub started_after: Option<String>,

    #[param(example = "2024-01-16T10:30:00Z")]
    pub started_before: Option<String>,
}

impl RunFilterParams {
    pub fn into_filter(self) -> Result<RunFilter, String> {
        let state = self.state.as_ref().map(|s| State::from_str(s)).transpose()?;
        let started_after = self.started_after.as_ref().map(|s| parse_datetime(s)).transpose()?;
        let started_before = self.started_before.as_ref().map(|s| parse_datetime(s)).transpose()?;

        if self.tag_value.is_some() && self.tag_key.is_none() {
            return Err("tag_key is required when tag_value is provided".to_string());
        }

        Ok(RunFilter {
            state,
            user_id: self.user_id,
            tag_key: self.tag_key,
            tag_value: self.tag_value,
            started_after,
            started_before,
        })
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("Invalid datetime format: {}", e))
}
