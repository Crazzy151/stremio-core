use crate::constants::{LIBRARY_COLLECTION_NAME, META_RESOURCE_NAME, STREAM_RESOURCE_NAME};
use crate::models::common::{
    eq_update, resources_update, resources_update_with_vector_content, Loadable, ResourceLoadable,
    ResourcesAction,
};
use crate::models::ctx::Ctx;
use crate::runtime::msg::{Action, ActionLoad, ActionMetaDetails, Internal, Msg};
use crate::runtime::{Effects, Env, UpdateWithCtx};
use crate::types::addon::{AggrRequest, ResourcePath, ResourceRequest};
use crate::types::api::{DatastoreCommand, DatastoreRequest};
use crate::types::library::{LibraryBucket, LibraryItem};
use crate::types::profile::Profile;
use crate::types::resource::{MetaItem, Stream};
use crate::types::streams::{StreamsBucket, StreamsItemKey};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::marker::PhantomData;
use stremio_watched_bitfield::WatchedBitField;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Selected {
    pub meta_path: ResourcePath,
    pub stream_path: Option<ResourcePath>,
}

#[derive(Default, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MetaDetails {
    pub selected: Option<Selected>,
    pub meta_items: Vec<ResourceLoadable<MetaItem>>,
    pub meta_streams: Vec<ResourceLoadable<Vec<Stream>>>,
    pub streams: Vec<ResourceLoadable<Vec<Stream>>>,
    pub suggested_stream: Option<Stream>,
    pub library_item: Option<LibraryItem>,
    #[serde(skip_serializing)]
    pub watched: Option<WatchedBitField>,
}

impl<E: Env + 'static> UpdateWithCtx<E> for MetaDetails {
    fn update(&mut self, msg: &Msg, ctx: &Ctx) -> Effects {
        match msg {
            Msg::Action(Action::Load(ActionLoad::MetaDetails(selected))) => {
                let selected_effects = eq_update(&mut self.selected, Some(selected.to_owned()));
                let meta_items_effects =
                    meta_items_update::<E>(&mut self.meta_items, &self.selected, &ctx.profile);
                let selected_override_effects =
                    selected_override_update(&mut self.selected, &self.meta_items);
                let meta_streams_effects =
                    meta_streams_update(&mut self.meta_streams, &self.selected, &self.meta_items);
                let streams_effects =
                    streams_update::<E>(&mut self.streams, &self.selected, &ctx.profile);
                let suggested_stream_effects = suggested_stream_update(
                    &mut self.suggested_stream,
                    &self.selected,
                    &self.meta_items,
                    &self.meta_streams,
                    &self.streams,
                    &ctx.streams,
                );
                let library_item_effects = library_item_update::<E>(
                    &mut self.library_item,
                    &self.selected,
                    &self.meta_items,
                    &ctx.library,
                );
                let watched_effects =
                    watched_update(&mut self.watched, &self.meta_items, &self.library_item);
                let libraty_item_sync_effects = library_item_sync(&self.library_item, &ctx.profile);
                libraty_item_sync_effects
                    .join(selected_effects)
                    .join(selected_override_effects)
                    .join(meta_items_effects)
                    .join(meta_streams_effects)
                    .join(streams_effects)
                    .join(suggested_stream_effects)
                    .join(library_item_effects)
                    .join(watched_effects)
            }
            Msg::Action(Action::Unload) => {
                let selected_effects = eq_update(&mut self.selected, None);
                let meta_items_effects = eq_update(&mut self.meta_items, vec![]);
                let meta_streams_effects = eq_update(&mut self.meta_streams, vec![]);
                let streams_effects = eq_update(&mut self.streams, vec![]);
                let library_item_effects = eq_update(&mut self.library_item, None);
                let suggested_stream_effects = eq_update(&mut self.suggested_stream, None);
                let watched_effects = eq_update(&mut self.watched, None);
                selected_effects
                    .join(meta_items_effects)
                    .join(meta_streams_effects)
                    .join(streams_effects)
                    .join(suggested_stream_effects)
                    .join(library_item_effects)
                    .join(watched_effects)
            }
            Msg::Action(Action::MetaDetails(ActionMetaDetails::MarkAsWatched(is_watched))) => {
                match &self.library_item {
                    Some(library_item) => {
                        let mut library_item = library_item.to_owned();
                        library_item.state.times_watched = match *is_watched {
                            true => library_item.state.times_watched + 1,
                            false => 0,
                        };
                        // update the last_watched for the LibraryItem
                        library_item.state.last_watched = Some(E::now());

                        Effects::msg(Msg::Internal(Internal::UpdateLibraryItem(library_item)))
                            .unchanged()
                    }
                    _ => Effects::none().unchanged(),
                }
            }
            Msg::Action(Action::MetaDetails(ActionMetaDetails::MarkVideoAsWatched(
                video,
                is_watched,
            ))) => match (&self.library_item, &self.watched) {
                (Some(library_item), Some(watched)) => {
                    let mut watched = watched.to_owned();
                    watched.set_video(&video.id, *is_watched);
                    let mut library_item = library_item.to_owned();
                    library_item.state.watched = Some(watched.into());
                    if *is_watched {
                        library_item.state.last_watched =
                            match (&library_item.state.last_watched, &video.released) {
                                (Some(last_watched), Some(released)) if last_watched < released => {
                                    Some(released.to_owned())
                                }
                                (None, released) => released.to_owned(),
                                (last_watched, _) => last_watched.to_owned(),
                            };
                    }
                    Effects::msg(Msg::Internal(Internal::UpdateLibraryItem(library_item)))
                        .unchanged()
                }
                _ => Effects::none().unchanged(),
            },
            Msg::Internal(Internal::ResourceRequestResult(request, result))
                if request.path.resource == META_RESOURCE_NAME =>
            {
                let meta_items_effects = resources_update::<E, _>(
                    &mut self.meta_items,
                    ResourcesAction::ResourceRequestResult { request, result },
                );
                let selected_override_effects =
                    selected_override_update(&mut self.selected, &self.meta_items);
                let streams_effects = if selected_override_effects.has_changed {
                    streams_update::<E>(&mut self.streams, &self.selected, &ctx.profile)
                } else {
                    Effects::default()
                };
                let meta_streams_effects =
                    meta_streams_update(&mut self.meta_streams, &self.selected, &self.meta_items);
                let suggested_stream_effects = suggested_stream_update(
                    &mut self.suggested_stream,
                    &self.selected,
                    &self.meta_items,
                    &self.meta_streams,
                    &self.streams,
                    &ctx.streams,
                );
                let library_item_effects = library_item_update::<E>(
                    &mut self.library_item,
                    &self.selected,
                    &self.meta_items,
                    &ctx.library,
                );
                let watched_effects =
                    watched_update(&mut self.watched, &self.meta_items, &self.library_item);
                selected_override_effects
                    .join(meta_items_effects)
                    .join(meta_streams_effects)
                    .join(streams_effects)
                    .join(suggested_stream_effects)
                    .join(library_item_effects)
                    .join(watched_effects)
            }
            Msg::Internal(Internal::ResourceRequestResult(request, result))
                if request.path.resource == STREAM_RESOURCE_NAME =>
            {
                resources_update_with_vector_content::<E, _>(
                    &mut self.streams,
                    ResourcesAction::ResourceRequestResult { request, result },
                )
            }
            Msg::Internal(Internal::LibraryChanged(_)) => {
                let library_item_effects = library_item_update::<E>(
                    &mut self.library_item,
                    &self.selected,
                    &self.meta_items,
                    &ctx.library,
                );
                let watched_effects =
                    watched_update(&mut self.watched, &self.meta_items, &self.library_item);
                library_item_effects.join(watched_effects)
            }
            Msg::Internal(Internal::ProfileChanged) => {
                let meta_items_effects =
                    meta_items_update::<E>(&mut self.meta_items, &self.selected, &ctx.profile);
                let meta_streams_effects =
                    meta_streams_update(&mut self.meta_streams, &self.selected, &self.meta_items);
                let streams_effects =
                    streams_update::<E>(&mut self.streams, &self.selected, &ctx.profile);
                let suggested_stream_effects = suggested_stream_update(
                    &mut self.suggested_stream,
                    &self.selected,
                    &self.meta_items,
                    &self.meta_streams,
                    &self.streams,
                    &ctx.streams,
                );
                let library_item_effects = library_item_update::<E>(
                    &mut self.library_item,
                    &self.selected,
                    &self.meta_items,
                    &ctx.library,
                );
                let watched_effects =
                    watched_update(&mut self.watched, &self.meta_items, &self.library_item);
                meta_items_effects
                    .join(meta_streams_effects)
                    .join(streams_effects)
                    .join(suggested_stream_effects)
                    .join(library_item_effects)
                    .join(watched_effects)
            }
            _ => Effects::none().unchanged(),
        }
    }
}

fn library_item_sync(library_item: &Option<LibraryItem>, profile: &Profile) -> Effects {
    match (library_item, profile.auth_key()) {
        (Some(library_item), Some(auth_key)) => {
            Effects::msg(Msg::Internal(Internal::LibrarySyncPlanResult(
                DatastoreRequest {
                    auth_key: auth_key.to_owned(),
                    collection: LIBRARY_COLLECTION_NAME.to_owned(),
                    command: DatastoreCommand::Meta {},
                },
                Ok((vec![library_item.id.to_owned()], vec![])),
            )))
            .unchanged()
        }
        _ => Effects::none().unchanged(),
    }
}

fn selected_override_update(
    selected: &mut Option<Selected>,
    meta_items: &[ResourceLoadable<MetaItem>],
) -> Effects {
    let meta_path = match &selected {
        Some(Selected { meta_path, .. }) => meta_path,
        _ => return Effects::default(),
    };
    let meta_item = match meta_items
        .iter()
        .find_map(|meta_item| match &meta_item.content {
            Some(Loadable::Ready(meta_item)) => Some(Some(meta_item)),
            Some(Loadable::Loading) => Some(None),
            _ => None,
        })
        .flatten()
    {
        Some(meta_item) => meta_item,
        _ => return Effects::default(),
    };
    let video_id = match (
        meta_item.videos.len(),
        &meta_item.preview.behavior_hints.default_video_id,
    ) {
        (_, Some(default_video_id)) => default_video_id.to_owned(),
        (0, None) => meta_item.preview.id.to_owned(),
        _ => return Effects::default(),
    };
    eq_update(
        selected,
        Some(Selected {
            meta_path: meta_path.to_owned(),
            stream_path: Some(ResourcePath {
                resource: STREAM_RESOURCE_NAME.to_owned(),
                r#type: meta_path.r#type.to_owned(),
                id: video_id,
                extra: vec![],
            }),
        }),
    )
}

fn meta_items_update<E: Env + 'static>(
    meta_items: &mut Vec<ResourceLoadable<MetaItem>>,
    selected: &Option<Selected>,
    profile: &Profile,
) -> Effects {
    match selected {
        Some(Selected { meta_path, .. }) => resources_update::<E, _>(
            meta_items,
            ResourcesAction::ResourcesRequested {
                request: &AggrRequest::AllOfResource(meta_path.to_owned()),
                addons: &profile.addons,
                // use existing loaded MetaItems instead of making a request every time.
                force: false,
            },
        ),
        _ => eq_update(meta_items, vec![]),
    }
}

fn meta_streams_update(
    meta_streams: &mut Vec<ResourceLoadable<Vec<Stream>>>,
    selected: &Option<Selected>,
    meta_items: &[ResourceLoadable<MetaItem>],
) -> Effects {
    match selected {
        Some(Selected {
            stream_path: Some(stream_path),
            ..
        }) => {
            let next_meta_streams = meta_items
                .iter()
                .find_map(|meta_item| match meta_item {
                    ResourceLoadable {
                        request,
                        content: Some(Loadable::Ready(meta_item)),
                    } => Some((request, meta_item)),
                    _ => None,
                })
                .and_then(|(request, meta_item)| {
                    meta_item
                        .videos
                        .iter()
                        .find(|video| video.id == stream_path.id)
                        .and_then(|video| {
                            if !video.streams.is_empty() {
                                Some(Cow::Borrowed(&video.streams))
                            } else {
                                Stream::youtube(&video.id)
                                    .map(|stream| vec![stream])
                                    .map(Cow::Owned)
                            }
                        })
                        .map(|streams| (request, streams))
                })
                .map(|(request, streams)| ResourceLoadable {
                    request: ResourceRequest {
                        base: request.base.to_owned(),
                        path: ResourcePath {
                            resource: STREAM_RESOURCE_NAME.to_owned(),
                            r#type: request.path.r#type.to_owned(),
                            id: stream_path.id.to_owned(),
                            extra: request.path.extra.to_owned(),
                        },
                    },
                    content: Some(Loadable::Ready(streams.into_owned())),
                })
                .into_iter()
                .collect();
            eq_update(meta_streams, next_meta_streams)
        }
        _ => Effects::none().unchanged(),
    }
}

fn streams_update<E: Env + 'static>(
    streams: &mut Vec<ResourceLoadable<Vec<Stream>>>,
    selected: &Option<Selected>,
    profile: &Profile,
) -> Effects {
    match selected {
        Some(Selected {
            stream_path: Some(stream_path),
            ..
        }) => resources_update_with_vector_content::<E, _>(
            streams,
            ResourcesAction::ResourcesRequested {
                request: &AggrRequest::AllOfResource(stream_path.to_owned()),
                addons: &profile.addons,
                // use existing loaded MetaItems instead of making a request every time.
                force: false,
            },
        ),
        _ => eq_update(streams, vec![]),
    }
}
/// Find a stream from addon responses, which should be played if binge watching or continuing to watch.
/// We've already loaded the next Video id and we need to find a proper stream for the binge watching.
///
/// First find the latest `StreamItem` stored based on last **30** videos from current video
/// (ie. we're in E4, so we're going to check E4, E3, E2, E1 in this order until we hit a stored `StreamItem`).
/// Then with the stream item we try to find a stream from addon responses (including the streams inside the meta itself `meta_streams`) -
/// we find the responses from the addon based on `StreamItem.stream_transport_url`,
/// then first we try to find the stream based on equality (as otherwise stored stream might be expired/no longer valid),
/// if not found we try to find a stream based on it's `StreamBehaviorHints.bingeGroup`.
/// One note, why we cannot return `StreamItem.stream` directly if it's for the same episode,
/// is that user might have played a stream from an addon which he no longer has due to some constrains (ie p2p addon),
/// that's why we have to try to find it first and verify that's it's still available.
fn suggested_stream_update(
    suggested_stream: &mut Option<Stream>,
    selected: &Option<Selected>,
    meta_items: &[ResourceLoadable<MetaItem>],
    meta_streams: &[ResourceLoadable<Vec<Stream>>],
    streams: &[ResourceLoadable<Vec<Stream>>],
    stream_bucket: &StreamsBucket,
) -> Effects {
    let next_suggested_stream = match selected {
        Some(Selected {
            meta_path,
            stream_path: Some(stream_path),
        }) => {
            meta_items
                .iter()
                .find_map(|meta_item| match &meta_item.content {
                    Some(Loadable::Ready(meta_item)) => Some(&meta_item.videos),
                    _ => None,
                })
                .and_then(|videos| {
                    // Check saved stream only for last 30 videos starting from the current video
                    videos
                        .iter()
                        .position(|video| video.id == stream_path.id)
                        .and_then(|max_index| {
                            videos[max_index.saturating_sub(30)..=max_index]
                                .iter()
                                .rev()
                                .find_map(|video| {
                                    stream_bucket.items.get(&StreamsItemKey {
                                        meta_id: meta_path.id.to_string(),
                                        video_id: video.id.to_owned(),
                                    })
                                })
                        })
                })
                .and_then(|stream_item| {
                    [meta_streams, streams]
                        .concat()
                        .iter()
                        .find(|resource| resource.request.base == stream_item.stream_transport_url)
                        .and_then(|resource| match &resource.content {
                            Some(Loadable::Ready(streams)) => Some(streams),
                            _ => None,
                        })
                        .and_then(|streams| {
                            streams
                                .iter()
                                .find(|stream| *stream == &stream_item.stream)
                                .or_else(|| {
                                    streams.iter().find(|stream| {
                                        stream.behavior_hints.binge_group.as_deref()
                                            == stream_item
                                                .stream
                                                .behavior_hints
                                                .binge_group
                                                .as_deref()
                                    })
                                })
                                .cloned()
                        })
                })
        }
        _ => None,
    };
    eq_update(suggested_stream, next_suggested_stream)
}

fn library_item_update<E: Env + 'static>(
    library_item: &mut Option<LibraryItem>,
    selected: &Option<Selected>,
    meta_items: &[ResourceLoadable<MetaItem>],
    library: &LibraryBucket,
) -> Effects {
    let meta_item = meta_items
        .iter()
        .find_map(|meta_item| match &meta_item.content {
            Some(Loadable::Ready(meta_item)) => Some(meta_item),
            _ => None,
        });
    let next_library_item = match selected {
        Some(selected) => library
            .items
            .get(&selected.meta_path.id)
            .map(|library_item| {
                meta_item.map_or_else(
                    || library_item.to_owned(),
                    |meta_item| LibraryItem::from((&meta_item.preview, library_item)),
                )
            })
            .or_else(|| {
                meta_item.map(|meta_item| LibraryItem::from((&meta_item.preview, PhantomData::<E>)))
            }),
        _ => None,
    };
    eq_update(library_item, next_library_item)
}

fn watched_update(
    watched: &mut Option<WatchedBitField>,
    meta_items: &[ResourceLoadable<MetaItem>],
    library_item: &Option<LibraryItem>,
) -> Effects {
    let next_watched = meta_items
        .iter()
        .find_map(|meta_item| match &meta_item.content {
            Some(Loadable::Ready(meta_item)) => Some(meta_item),
            _ => None,
        })
        .and_then(|meta_item| {
            library_item
                .as_ref()
                .map(|library_item| (meta_item, library_item))
        })
        .map(|(meta_item, library_item)| library_item.state.watched_bitfield(&meta_item.videos));
    eq_update(watched, next_watched)
}
