use crate::constants::OFFICIAL_ADDONS;
use crate::models::common::Loadable;
use crate::runtime::msg::{Internal, Msg};
use crate::runtime::{EffectFuture, Effects, Env, EnvError, EnvFutureExt};
use crate::types::addon::{Descriptor, Manifest};
use futures::FutureExt;
use serde::Serialize;
use url::Url;

#[derive(PartialEq, Serialize)]
pub struct DescriptorLoadable {
    pub transport_url: Url,
    pub content: Loadable<Descriptor, EnvError>,
}

pub enum DescriptorAction<'a> {
    DescriptorRequested {
        transport_url: &'a Url,
    },
    ManifestRequestResult {
        transport_url: &'a Url,
        result: &'a Result<Manifest, EnvError>,
    },
}

pub fn descriptor_update<E: Env + 'static>(
    descriptor: &mut Option<DescriptorLoadable>,
    action: DescriptorAction,
) -> Effects {
    match action {
        DescriptorAction::DescriptorRequested { transport_url } => {
            if descriptor
                .as_ref()
                .map(|descriptor| &descriptor.transport_url)
                != Some(transport_url)
            {
                let transport_url = transport_url.to_owned();
                *descriptor = Some(DescriptorLoadable {
                    transport_url: transport_url.to_owned(),
                    content: Loadable::Loading,
                });
                Effects::future(EffectFuture::Concurrent(
                    E::addon_transport(&transport_url)
                        .manifest()
                        .map(move |result| {
                            Msg::Internal(Internal::ManifestRequestResult(transport_url, result))
                        })
                        .boxed_env(),
                ))
            } else {
                Effects::none().unchanged()
            }
        }
        DescriptorAction::ManifestRequestResult {
            transport_url,
            result,
        } => match descriptor {
            Some(DescriptorLoadable {
                transport_url: loading_transport_url,
                content: Loadable::Loading,
            }) if loading_transport_url == transport_url => {
                *descriptor = Some(DescriptorLoadable {
                    transport_url: transport_url.to_owned(),
                    content: match result {
                        Ok(manifest) => Loadable::Ready(Descriptor {
                            transport_url: transport_url.to_owned(),
                            manifest: manifest.to_owned(),
                            flags: OFFICIAL_ADDONS
                                .iter()
                                .find(|descriptor| descriptor.transport_url == *transport_url)
                                .map(|descriptor| descriptor.flags.to_owned())
                                .unwrap_or_default(),
                        }),
                        Err(error) => Loadable::Err(error.to_owned()),
                    },
                });
                Effects::none()
            }
            _ => Effects::none().unchanged(),
        },
    }
}
