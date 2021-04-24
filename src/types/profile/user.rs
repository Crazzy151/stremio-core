use crate::types::empty_string_as_none;
#[cfg(test)]
use chrono::offset::TimeZone;
use chrono::{DateTime, Utc};
#[cfg(test)]
use derivative::Derivative;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(test, derive(Default))]
pub struct GDPRConsent {
    pub tos: bool,
    pub privacy: bool,
    pub marketing: bool,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(test, derive(Derivative))]
#[cfg_attr(test, derivative(Default))]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id")]
    pub id: String,
    pub email: String,
    #[serde(deserialize_with = "empty_string_as_none", default)]
    pub fb_id: Option<String>,
    #[serde(deserialize_with = "empty_string_as_none", default)]
    pub avatar: Option<String>,
    #[cfg_attr(test, derivative(Default(value = "Utc.timestamp(0, 0)")))]
    pub last_modified: DateTime<Utc>,
    #[cfg_attr(test, derivative(Default(value = "Utc.timestamp(0, 0)")))]
    pub date_registered: DateTime<Utc>,
    #[serde(rename = "gdpr_consent")]
    pub gdpr_consent: GDPRConsent,
}
