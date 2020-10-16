use crate::constants::CATALOG_PREVIEW_SIZE;
use crate::models::ctx::Ctx;
use crate::runtime::msg::{Internal, Msg};
use crate::runtime::{Effects, Env, UpdateWithCtx};
use crate::types::library::{LibraryBucket, LibraryItem};
use lazysort::SortedBy;
use serde::Serialize;

#[derive(Default, Serialize)]
pub struct ContinueWatchingPreview {
    pub library_items: Vec<LibraryItem>,
}

impl<E: Env + 'static> UpdateWithCtx<Ctx<E>> for ContinueWatchingPreview {
    fn update(&mut self, msg: &Msg, ctx: &Ctx<E>) -> Effects {
        match msg {
            Msg::Internal(Internal::LibraryChanged(_)) => {
                library_items_update(&mut self.library_items, &ctx.library)
            }
            _ => Effects::none().unchanged(),
        }
    }
}

fn library_items_update(library_items: &mut Vec<LibraryItem>, library: &LibraryBucket) -> Effects {
    let next_library_items = library
        .items
        .values()
        .filter(|library_item| library_item.is_in_continue_watching())
        .sorted_by(|a, b| b.mtime.cmp(&a.mtime))
        .take(CATALOG_PREVIEW_SIZE)
        .cloned()
        .collect::<Vec<_>>();
    if *library_items != next_library_items {
        *library_items = next_library_items;
        Effects::none()
    } else {
        Effects::none().unchanged()
    }
}