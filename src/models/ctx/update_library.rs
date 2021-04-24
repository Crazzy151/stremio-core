use crate::constants::{
    LIBRARY_COLLECTION_NAME, LIBRARY_RECENT_COUNT, LIBRARY_RECENT_STORAGE_KEY, LIBRARY_STORAGE_KEY,
};
use crate::models::ctx::{CtxError, CtxStatus, OtherError};
use crate::runtime::msg::{Action, ActionCtx, Event, Internal, Msg};
use crate::runtime::{Effect, Effects, Env};
use crate::types::api::{
    fetch_api, APIResult, DatastoreCommand, DatastoreRequest, LibraryItemModified, SuccessResponse,
};
use crate::types::library::{
    LibraryBucket, LibraryBucketRef, LibraryItem, LibraryItemBehaviorHints, LibraryItemState,
};
use crate::types::profile::AuthKey;
use futures::future::Either;
use futures::{future, FutureExt, TryFutureExt};
use std::collections::HashMap;

pub fn update_library<E: Env + 'static>(
    library: &mut LibraryBucket,
    auth_key: Option<&AuthKey>,
    status: &CtxStatus,
    msg: &Msg,
) -> Effects {
    match msg {
        Msg::Action(Action::Ctx(ActionCtx::Logout)) => {
            let next_library = LibraryBucket::default();
            if *library != next_library {
                *library = next_library;
                Effects::msg(Msg::Internal(Internal::LibraryChanged(false)))
            } else {
                Effects::none().unchanged()
            }
        }
        Msg::Action(Action::Ctx(ActionCtx::AddToLibrary(meta_preview))) => {
            let mut library_item = LibraryItem {
                id: meta_preview.id.to_owned(),
                r#type: meta_preview.r#type.to_owned(),
                name: meta_preview.name.to_owned(),
                poster: meta_preview.poster.to_owned(),
                poster_shape: meta_preview.poster_shape.to_owned(),
                behavior_hints: LibraryItemBehaviorHints {
                    default_video_id: meta_preview.behavior_hints.default_video_id.to_owned(),
                },
                removed: false,
                temp: false,
                mtime: E::now(),
                ctime: Some(E::now()),
                state: LibraryItemState::default(),
            };
            if let Some(LibraryItem { ctime, state, .. }) = library.items.get(&meta_preview.id) {
                library_item.state = state.to_owned();
                if let Some(ctime) = ctime {
                    library_item.ctime = Some(ctime.to_owned());
                };
            };
            Effects::msg(Msg::Internal(Internal::UpdateLibraryItem(library_item)))
                .join(Effects::msg(Msg::Event(Event::LibraryItemAdded {
                    id: meta_preview.id.to_owned(),
                })))
                .unchanged()
        }
        Msg::Action(Action::Ctx(ActionCtx::RemoveFromLibrary(id))) => match library.items.get(id) {
            Some(library_item) => {
                let mut library_item = library_item.to_owned();
                library_item.removed = true;
                Effects::msg(Msg::Internal(Internal::UpdateLibraryItem(library_item)))
                    .join(Effects::msg(Msg::Event(Event::LibraryItemRemoved {
                        id: id.to_owned(),
                    })))
                    .unchanged()
            }
            _ => Effects::msg(Msg::Event(Event::Error {
                error: CtxError::from(OtherError::LibraryItemNotFound),
                source: Box::new(Event::LibraryItemRemoved { id: id.to_owned() }),
            }))
            .unchanged(),
        },
        Msg::Action(Action::Ctx(ActionCtx::RewindLibraryItem(id))) => match library.items.get(id) {
            Some(library_item) => {
                let mut library_item = library_item.to_owned();
                library_item.state.time_offset = 0;
                Effects::msg(Msg::Internal(Internal::UpdateLibraryItem(library_item)))
                    .join(Effects::msg(Msg::Event(Event::LibraryItemRewinded {
                        id: id.to_owned(),
                    })))
                    .unchanged()
            }
            _ => Effects::msg(Msg::Event(Event::Error {
                error: CtxError::from(OtherError::LibraryItemNotFound),
                source: Box::new(Event::LibraryItemRewinded { id: id.to_owned() }),
            }))
            .unchanged(),
        },
        Msg::Action(Action::Ctx(ActionCtx::SyncLibraryWithAPI)) => match auth_key {
            Some(auth_key) => Effects::one(plan_sync_with_api::<E>(library, auth_key)).unchanged(),
            _ => Effects::msg(Msg::Event(Event::Error {
                error: CtxError::from(OtherError::UserNotLoggedIn),
                source: Box::new(Event::LibrarySyncWithAPIPlanned {
                    plan: Default::default(),
                }),
            }))
            .unchanged(),
        },
        Msg::Internal(Internal::UpdateLibraryItem(library_item)) => {
            let mut library_item = library_item.to_owned();
            library_item.mtime = E::now();
            let push_to_api_effects = match auth_key {
                Some(auth_key) => Effects::one(push_items_to_api::<E>(
                    vec![library_item.to_owned()],
                    auth_key,
                ))
                .unchanged(),
                _ => Effects::none().unchanged(),
            };
            let push_to_storage_effects = Effects::one(update_and_push_items_to_storage::<E>(
                library,
                vec![library_item],
            ));
            push_to_api_effects
                .join(push_to_storage_effects)
                .join(Effects::msg(Msg::Internal(Internal::LibraryChanged(true))))
        }
        Msg::Internal(Internal::LibraryChanged(persisted)) if !persisted => {
            Effects::one(push_library_to_storage::<E>(library)).unchanged()
        }
        Msg::Internal(Internal::CtxAuthResult(auth_request, result)) => match (status, result) {
            (CtxStatus::Loading(loading_auth_request), Ok((auth, _, library_items)))
                if loading_auth_request == auth_request =>
            {
                let next_library =
                    LibraryBucket::new(Some(auth.user.id.to_owned()), library_items.to_owned());
                if *library != next_library {
                    *library = next_library;
                    Effects::msg(Msg::Internal(Internal::LibraryChanged(false)))
                } else {
                    Effects::none().unchanged()
                }
            }
            _ => Effects::none().unchanged(),
        },
        Msg::Internal(Internal::LibrarySyncPlanResult(
            DatastoreRequest {
                auth_key: loading_auth_key,
                ..
            },
            result,
        )) if Some(loading_auth_key) == auth_key => match result {
            Ok((pull_ids, push_ids)) => {
                let push_items = library
                    .items
                    .iter()
                    .filter(move |(id, _)| push_ids.iter().any(|push_id| push_id == *id))
                    .map(|(_, item)| item)
                    .cloned()
                    .collect::<Vec<_>>();
                let push_items_to_api_effects = if push_items.is_empty() {
                    Effects::none().unchanged()
                } else {
                    Effects::one(push_items_to_api::<E>(push_items, loading_auth_key)).unchanged()
                };
                let pull_items_from_api_effects = if pull_ids.is_empty() {
                    Effects::none().unchanged()
                } else {
                    Effects::one(pull_items_from_api::<E>(
                        pull_ids.to_owned(),
                        loading_auth_key,
                    ))
                    .unchanged()
                };
                Effects::msg(Msg::Event(Event::LibrarySyncWithAPIPlanned {
                    plan: (pull_ids.to_owned(), push_ids.to_owned()),
                }))
                .join(push_items_to_api_effects)
                .join(pull_items_from_api_effects)
                .unchanged()
            }
            Err(error) => Effects::msg(Msg::Event(Event::Error {
                error: error.to_owned(),
                source: Box::new(Event::LibrarySyncWithAPIPlanned {
                    plan: Default::default(),
                }),
            }))
            .unchanged(),
        },
        Msg::Internal(Internal::LibraryPullResult(
            DatastoreRequest {
                auth_key: loading_auth_key,
                command: DatastoreCommand::Get { ids, .. },
                ..
            },
            result,
        )) if Some(loading_auth_key) == auth_key => match result {
            Ok(items) => Effects::msg(Msg::Event(Event::LibraryItemsPulledFromAPI {
                ids: ids.to_owned(),
            }))
            .join(Effects::one(update_and_push_items_to_storage::<E>(
                library,
                items.to_owned(),
            )))
            .join(Effects::msg(Msg::Internal(Internal::LibraryChanged(true)))),
            Err(error) => Effects::msg(Msg::Event(Event::Error {
                error: error.to_owned(),
                source: Box::new(Event::LibraryItemsPulledFromAPI {
                    ids: ids.to_owned(),
                }),
            }))
            .unchanged(),
        },
        _ => Effects::none().unchanged(),
    }
}

fn update_and_push_items_to_storage<E: Env + 'static>(
    library: &mut LibraryBucket,
    items: Vec<LibraryItem>,
) -> Effect {
    let ids = items
        .iter()
        .map(|item| &item.id)
        .cloned()
        .collect::<Vec<_>>();
    let are_items_in_recent = library.are_ids_in_recent(&ids);
    library.merge_items(items);
    let push_to_storage_future = if library.items.len() <= LIBRARY_RECENT_COUNT {
        Either::Left(
            future::try_join_all(vec![
                E::set_storage(LIBRARY_RECENT_STORAGE_KEY, Some(&library)),
                E::set_storage::<()>(LIBRARY_STORAGE_KEY, None),
            ])
            .map_ok(|_| ()),
        )
    } else {
        let (recent_items, other_items) = library.split_items_by_recent();
        if are_items_in_recent {
            Either::Right(Either::Left(E::set_storage(
                LIBRARY_RECENT_STORAGE_KEY,
                Some(&LibraryBucketRef::new(&library.uid, &recent_items)),
            )))
        } else {
            Either::Right(Either::Right(
                future::try_join_all(vec![
                    E::set_storage(
                        LIBRARY_RECENT_STORAGE_KEY,
                        Some(&LibraryBucketRef::new(&library.uid, &recent_items)),
                    ),
                    E::set_storage(
                        LIBRARY_STORAGE_KEY,
                        Some(&LibraryBucketRef::new(&library.uid, &other_items)),
                    ),
                ])
                .map_ok(|_| ()),
            ))
        }
    };
    push_to_storage_future
        .map(move |result| match result {
            Ok(_) => Msg::Event(Event::LibraryItemsPushedToStorage { ids }),
            Err(error) => Msg::Event(Event::Error {
                error: CtxError::from(error),
                source: Box::new(Event::LibraryItemsPushedToStorage { ids }),
            }),
        })
        .boxed_local()
        .into()
}

fn push_library_to_storage<E: Env + 'static>(library: &LibraryBucket) -> Effect {
    let ids = library.items.keys().cloned().collect();
    let (recent_items, other_items) = library.split_items_by_recent();
    future::try_join_all(vec![
        E::set_storage(
            LIBRARY_RECENT_STORAGE_KEY,
            Some(&LibraryBucketRef::new(&library.uid, &recent_items)),
        ),
        E::set_storage(
            LIBRARY_STORAGE_KEY,
            Some(&LibraryBucketRef::new(&library.uid, &other_items)),
        ),
    ])
    .map(move |result| match result {
        Ok(_) => Msg::Event(Event::LibraryItemsPushedToStorage { ids }),
        Err(error) => Msg::Event(Event::Error {
            error: CtxError::from(error),
            source: Box::new(Event::LibraryItemsPushedToStorage { ids }),
        }),
    })
    .boxed_local()
    .into()
}

fn push_items_to_api<E: Env + 'static>(items: Vec<LibraryItem>, auth_key: &AuthKey) -> Effect {
    let ids = items.iter().map(|item| &item.id).cloned().collect();
    fetch_api::<E, _, SuccessResponse>(&DatastoreRequest {
        auth_key: auth_key.to_owned(),
        collection: LIBRARY_COLLECTION_NAME.to_owned(),
        command: DatastoreCommand::Put { changes: items },
    })
    .map_err(CtxError::from)
    .and_then(|result| match result {
        APIResult::Ok { result } => future::ok(result),
        APIResult::Err { error } => future::err(CtxError::from(error)),
    })
    .map(move |result| match result {
        Ok(_) => Msg::Event(Event::LibraryItemsPushedToAPI { ids }),
        Err(error) => Msg::Event(Event::Error {
            error,
            source: Box::new(Event::LibraryItemsPushedToAPI { ids }),
        }),
    })
    .boxed_local()
    .into()
}

fn pull_items_from_api<E: Env + 'static>(ids: Vec<String>, auth_key: &AuthKey) -> Effect {
    let request = DatastoreRequest {
        auth_key: auth_key.to_owned(),
        collection: LIBRARY_COLLECTION_NAME.to_owned(),
        command: DatastoreCommand::Get { ids, all: false },
    };
    fetch_api::<E, _, _>(&request)
        .map_err(CtxError::from)
        .and_then(|result| match result {
            APIResult::Ok { result } => future::ok(result),
            APIResult::Err { error } => future::err(CtxError::from(error)),
        })
        .map(move |result| Msg::Internal(Internal::LibraryPullResult(request, result)))
        .boxed_local()
        .into()
}

fn plan_sync_with_api<E: Env + 'static>(library: &LibraryBucket, auth_key: &AuthKey) -> Effect {
    let local_mtimes = library
        .items
        .iter()
        .filter(|(_, item)| item.should_sync())
        .map(|(id, item)| (id.to_owned(), item.mtime.to_owned()))
        .collect::<HashMap<_, _>>();
    let request = DatastoreRequest {
        auth_key: auth_key.to_owned(),
        collection: LIBRARY_COLLECTION_NAME.to_owned(),
        command: DatastoreCommand::Meta {},
    };
    fetch_api::<E, _, Vec<LibraryItemModified>>(&request)
        .map_err(CtxError::from)
        .and_then(|result| match result {
            APIResult::Ok { result } => future::ok(result),
            APIResult::Err { error } => future::err(CtxError::from(error)),
        })
        .map_ok(|remote_mtimes| {
            remote_mtimes
                .into_iter()
                .map(|LibraryItemModified(id, mtime)| (id, mtime))
                .collect::<HashMap<_, _>>()
        })
        .map_ok(move |remote_mtimes| {
            let pull_ids = remote_mtimes
                .iter()
                .filter(|(id, remote_mtime)| {
                    local_mtimes
                        .get(*id)
                        .map_or(true, |local_mtime| local_mtime < remote_mtime)
                })
                .map(|(id, _)| id)
                .cloned()
                .collect();
            let push_ids = local_mtimes
                .iter()
                .filter(|(id, local_mtime)| {
                    remote_mtimes
                        .get(*id)
                        .map_or(true, |remote_mtime| remote_mtime < local_mtime)
                })
                .map(|(id, _)| id)
                .cloned()
                .collect();
            (pull_ids, push_ids)
        })
        .map(move |result| Msg::Internal(Internal::LibrarySyncPlanResult(request, result)))
        .boxed_local()
        .into()
}
