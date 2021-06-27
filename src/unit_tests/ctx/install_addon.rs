use crate::constants::PROFILE_STORAGE_KEY;
use crate::models::ctx::Ctx;
use crate::runtime::msg::{Action, ActionCtx};
use crate::runtime::{Effects, Env, Runtime, RuntimeAction, TryEnvFuture};
use crate::types::addon::{Descriptor, Manifest};
use crate::types::api::{APIResult, SuccessResponse};
use crate::types::profile::{Auth, AuthKey, GDPRConsent, Profile, User};
use crate::types::True;
use crate::unit_tests::{
    default_fetch_handler, Request, TestEnv, FETCH_HANDLER, REQUESTS, STORAGE,
};
use futures::{future, FutureExt};
use semver::Version;
use std::any::Any;
use stremio_derive::Model;
use url::Url;

#[test]
fn actionctx_installaddon_install() {
    #[derive(Model, Default)]
    #[model(TestEnv)]
    struct TestModel {
        ctx: Ctx,
    }
    let addon = Descriptor {
        manifest: Manifest {
            id: "id".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url").unwrap(),
        flags: Default::default(),
    };
    TestEnv::reset();
    let (runtime, _rx) = Runtime::<TestEnv, _>::new(
        TestModel {
            ctx: Ctx {
                profile: Profile {
                    addons: vec![],
                    ..Default::default()
                },
                ..Default::default()
            },
        },
        Effects::none().unchanged(),
        1000,
    );
    TestEnv::run(|| {
        runtime.dispatch(RuntimeAction {
            field: None,
            action: Action::Ctx(ActionCtx::InstallAddon(addon.to_owned())),
        })
    });
    assert_eq!(
        runtime.model().unwrap().ctx.profile.addons,
        vec![addon.to_owned()],
        "addon installed successfully"
    );
    assert!(
        STORAGE
            .read()
            .unwrap()
            .get(PROFILE_STORAGE_KEY)
            .map_or(false, |data| {
                serde_json::from_str::<Profile>(&data).unwrap().addons == vec![addon.to_owned()]
            }),
        "addon updated successfully in storage"
    );
    assert!(
        REQUESTS.read().unwrap().is_empty(),
        "No requests have been sent"
    );
}

#[test]
fn actionctx_installaddon_install_with_user() {
    #[derive(Model, Default)]
    #[model(TestEnv)]
    struct TestModel {
        ctx: Ctx,
    }
    fn fetch_handler(request: Request) -> TryEnvFuture<Box<dyn Any>> {
        match request {
            Request {
                url, method, body, ..
            } if url == "https://api.strem.io/api/addonCollectionSet"
                && method == "POST"
                && body == "{\"type\":\"AddonCollectionSet\",\"authKey\":\"auth_key\",\"addons\":[{\"manifest\":{\"id\":\"id\",\"version\":\"0.0.1\",\"name\":\"name\",\"contactEmail\":null,\"description\":null,\"logo\":null,\"background\":null,\"types\":[],\"resources\":[],\"idPrefixes\":null,\"catalogs\":[],\"addonCatalogs\":[],\"behaviorHints\":{\"adult\":false,\"p2p\":false,\"configurable\":false,\"configurationRequired\":false}},\"transportUrl\":\"https://transport_url/\",\"flags\":{\"official\":false,\"protected\":false}}]}" =>
            {
                future::ok(Box::new(APIResult::Ok {
                    result: SuccessResponse { success: True {} },
                }) as Box<dyn Any>).boxed_local()
            }
            _ => default_fetch_handler(request),
        }
    }
    let addon = Descriptor {
        manifest: Manifest {
            id: "id".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url").unwrap(),
        flags: Default::default(),
    };
    TestEnv::reset();
    *FETCH_HANDLER.write().unwrap() = Box::new(fetch_handler);
    let (runtime, _rx) = Runtime::<TestEnv, _>::new(
        TestModel {
            ctx: Ctx {
                profile: Profile {
                    auth: Some(Auth {
                        key: AuthKey("auth_key".to_owned()),
                        user: User {
                            id: "user_id".to_owned(),
                            email: "user_email".to_owned(),
                            fb_id: None,
                            avatar: None,
                            last_modified: TestEnv::now(),
                            date_registered: TestEnv::now(),
                            gdpr_consent: GDPRConsent {
                                tos: true,
                                privacy: true,
                                marketing: true,
                            },
                        },
                    }),
                    addons: vec![],
                    ..Default::default()
                },
                ..Default::default()
            },
        },
        Effects::none().unchanged(),
        1000,
    );
    TestEnv::run(|| {
        runtime.dispatch(RuntimeAction {
            field: None,
            action: Action::Ctx(ActionCtx::InstallAddon(addon.to_owned())),
        })
    });
    assert_eq!(
        runtime.model().unwrap().ctx.profile.addons,
        vec![addon.to_owned()],
        "addon installed successfully"
    );
    assert!(
        STORAGE
            .read()
            .unwrap()
            .get(PROFILE_STORAGE_KEY)
            .map_or(false, |data| {
                serde_json::from_str::<Profile>(&data).unwrap().addons == vec![addon.to_owned()]
            }),
        "addon updated successfully in storage"
    );
    assert_eq!(
        REQUESTS.read().unwrap().len(),
        1,
        "One request has been sent"
    );
    assert_eq!(
        REQUESTS.read().unwrap().get(0).unwrap().to_owned(),
        Request {
            url: "https://api.strem.io/api/addonCollectionSet".to_owned(),
            method: "POST".to_owned(),
            body: "{\"type\":\"AddonCollectionSet\",\"authKey\":\"auth_key\",\"addons\":[{\"manifest\":{\"id\":\"id\",\"version\":\"0.0.1\",\"name\":\"name\",\"contactEmail\":null,\"description\":null,\"logo\":null,\"background\":null,\"types\":[],\"resources\":[],\"idPrefixes\":null,\"catalogs\":[],\"addonCatalogs\":[],\"behaviorHints\":{\"adult\":false,\"p2p\":false,\"configurable\":false,\"configurationRequired\":false}},\"transportUrl\":\"https://transport_url/\",\"flags\":{\"official\":false,\"protected\":false}}]}"
                .to_owned(),
            ..Default::default()
        },
        "addonCollectionSet request has been sent"
    );
}

#[test]
fn actionctx_installaddon_update() {
    #[derive(Model, Default)]
    #[model(TestEnv)]
    struct TestModel {
        ctx: Ctx,
    }
    let addon = Descriptor {
        manifest: Manifest {
            id: "id1.0".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url1").unwrap(),
        flags: Default::default(),
    };
    let addon2 = Descriptor {
        manifest: Manifest {
            id: "id2".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url2").unwrap(),
        flags: Default::default(),
    };
    TestEnv::reset();
    let (runtime, _rx) = Runtime::<TestEnv, _>::new(
        TestModel {
            ctx: Ctx {
                profile: Profile {
                    addons: vec![
                        Descriptor {
                            manifest: Manifest {
                                id: "id1".to_owned(),
                                version: Version::new(0, 0, 1),
                                name: "name".to_owned(),
                                contact_email: None,
                                description: None,
                                logo: None,
                                background: None,
                                types: vec![],
                                resources: vec![],
                                id_prefixes: None,
                                catalogs: vec![],
                                addon_catalogs: vec![],
                                behavior_hints: Default::default(),
                            },
                            transport_url: Url::parse("https://transport_url1").unwrap(),
                            flags: Default::default(),
                        },
                        addon2.to_owned(),
                    ],
                    ..Default::default()
                },
                ..Default::default()
            },
        },
        Effects::none().unchanged(),
        1000,
    );
    TestEnv::run(|| {
        runtime.dispatch(RuntimeAction {
            field: None,
            action: Action::Ctx(ActionCtx::InstallAddon(addon.to_owned())),
        })
    });
    assert_eq!(
        runtime.model().unwrap().ctx.profile.addons,
        vec![addon.to_owned(), addon2.to_owned()],
        "addon updated successfully in memory"
    );
    assert!(
        STORAGE
            .read()
            .unwrap()
            .get(PROFILE_STORAGE_KEY)
            .map_or(false, |data| {
                serde_json::from_str::<Profile>(&data).unwrap().addons
                    == vec![addon.to_owned(), addon2.to_owned()]
            }),
        "addon updated successfully in storage"
    );
    assert!(
        REQUESTS.read().unwrap().is_empty(),
        "No requests have been sent"
    );
}

#[test]
fn actionctx_installaddon_upgrade() {
    #[derive(Model, Default)]
    #[model(TestEnv)]   
    struct TestModel {
        ctx: Ctx,
    }
    let addon1 = Descriptor {
        manifest: Manifest {
            id: "id1".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url1").unwrap(),
        flags: Default::default(),
    };
    let addon2 = Descriptor {
        manifest: Manifest {
            id: "id1".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url2").unwrap(),
        flags: Default::default(),
    };
    TestEnv::reset();
    let (runtime, _rx) = Runtime::<TestEnv, _>::new(
        TestModel {
            ctx: Ctx {
                profile: Profile {
                    addons: vec![addon1.to_owned()],
                    ..Default::default()
                },
                ..Default::default()
            },
        },
        Effects::none().unchanged(),
        1000,
    );
    TestEnv::run(|| {
        runtime.dispatch(RuntimeAction {
            field: None,
            action: Action::Ctx(ActionCtx::UpgradeAddon(addon2.to_owned())),
        })
    });
    
    assert_eq!(
        runtime.model().unwrap().ctx.profile.addons,
        vec![addon2.to_owned()],
        "addon upgrade successfully in memory"
    );
    assert!(
        STORAGE
            .read()
            .unwrap()
            .get(PROFILE_STORAGE_KEY)
            .map_or(false, |data| {
                serde_json::from_str::<Profile>(&data).unwrap().addons
                    == vec![addon2.to_owned()]
            }),
        "addon upgrade successfully in storage"
    );
    assert!(
        REQUESTS.read().unwrap().is_empty(),
        "No requests have been sent"
    );
}

#[test]
fn actionctx_installaddon_already_installed() {
    #[derive(Model, Default)]
    #[model(TestEnv)]
    struct TestModel {
        ctx: Ctx,
    }
    let addon = Descriptor {
        manifest: Manifest {
            id: "id".to_owned(),
            version: Version::new(0, 0, 1),
            name: "name".to_owned(),
            contact_email: None,
            description: None,
            logo: None,
            background: None,
            types: vec![],
            resources: vec![],
            id_prefixes: None,
            catalogs: vec![],
            addon_catalogs: vec![],
            behavior_hints: Default::default(),
        },
        transport_url: Url::parse("https://transport_url").unwrap(),
        flags: Default::default(),
    };
    let profile = Profile {
        addons: vec![addon.to_owned()],
        ..Default::default()
    };
    TestEnv::reset();
    STORAGE.write().unwrap().insert(
        PROFILE_STORAGE_KEY.to_owned(),
        serde_json::to_string(&profile).unwrap(),
    );
    let (runtime, _rx) = Runtime::<TestEnv, _>::new(
        TestModel {
            ctx: Ctx {
                profile,
                ..Default::default()
            },
        },
        Effects::none().unchanged(),
        1000,
    );
    TestEnv::run(|| {
        runtime.dispatch(RuntimeAction {
            field: None,
            action: Action::Ctx(ActionCtx::InstallAddon(addon.to_owned())),
        })
    });
    assert_eq!(
        runtime.model().unwrap().ctx.profile.addons,
        vec![addon.to_owned()],
        "addons in memory not updated"
    );
    assert!(
        STORAGE
            .read()
            .unwrap()
            .get(PROFILE_STORAGE_KEY)
            .map_or(false, |data| {
                serde_json::from_str::<Profile>(&data).unwrap().addons == vec![addon.to_owned()]
            }),
        "addons in storage not updated"
    );
    assert!(
        REQUESTS.read().unwrap().is_empty(),
        "No requests have been sent"
    );
}
