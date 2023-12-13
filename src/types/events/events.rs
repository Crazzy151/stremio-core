use chrono::{DateTime, Utc, Local};
use serde::Serialize;

use crate::{
    models::{common::Loadable, ctx::CtxError},
    types::api::{GetModalResponse, GetNotificationResponse},
};

#[derive(PartialEq, Eq, Serialize, Clone, Debug)]
pub struct Events {
    pub modal: ModalEvent,
    pub notification: NotificationEvent,
}

#[derive(PartialEq, Eq, Serialize, Clone, Debug)]
pub struct NotificationEvent {
    /// the notification contains the date that was sent with the request to retrieve the latest Notification
    pub notification: Loadable<(DateTime<Local>, Option<GetNotificationResponse>), CtxError>,
    pub last_updated: Option<DateTime<Utc>>,
    pub dismissed: bool,
}

#[derive(PartialEq, Eq, Serialize, Clone, Debug)]
pub struct ModalEvent {
    /// the modal contains the date that was sent with the request to retrieve the latest Modal
    pub modal: Loadable<(DateTime<Local>, Option<GetModalResponse>), CtxError>,
    pub last_updated: Option<DateTime<Utc>>,
    pub dismissed: bool,
}