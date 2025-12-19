use fluent_bundle::{FluentBundle, FluentResource};
use once_cell::unsync::Lazy;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use unic_langid::{langid, LanguageIdentifier};

const EN_US_FTL: &str = include_str!("../assets/locales/en-US/main.ftl");
const PL_PL_FTL: &str = include_str!("../assets/locales/pl-PL/main.ftl");

type BundleRc = Rc<FluentBundle<FluentResource>>;

thread_local! {
    static BUNDLES: Lazy<HashMap<&'static str, BundleRc>> = Lazy::new(|| {
        let mut map = HashMap::new();
        map.insert("en-US", Rc::new(build_bundle(langid!("en-US"), EN_US_FTL)));
        map.insert("pl-PL", Rc::new(build_bundle(langid!("pl-PL"), PL_PL_FTL)));
        map
    });

    static CURRENT_LANG: RefCell<String> = RefCell::new("en-US".to_string());
}

fn build_bundle(lang: LanguageIdentifier, ftl: &str) -> FluentBundle<FluentResource> {
    let mut bundle = FluentBundle::new(vec![lang]);
    let resource =
        FluentResource::try_new(ftl.to_string()).expect("Failed to parse Fluent resources");
    bundle
        .add_resource(resource)
        .expect("Failed to add Fluent resources to bundle");
    bundle
}

fn bundle_for(lang: &str) -> BundleRc {
    BUNDLES.with(|bundles| {
        bundles
            .get(lang)
            .cloned()
            .or_else(|| bundles.get("en-US").cloned())
            .expect("Default locale bundle missing")
    })
}

fn translate(bundle: &FluentBundle<FluentResource>, key: &str) -> Option<String> {
    let message = bundle.get_message(key)?;
    let pattern = message.value()?;
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, None, &mut errors);
    if errors.is_empty() {
        Some(value.into_owned())
    } else {
        None
    }
}

pub fn set_locale(lang: &str) {
    BUNDLES.with(|bundles| {
        let target = if bundles.contains_key(lang) {
            lang
        } else {
            "en-US"
        };
        CURRENT_LANG.with(|current| {
            *current.borrow_mut() = target.to_string();
        });
    });
}

pub fn t(key: &str) -> String {
    let lang = CURRENT_LANG.with(|current| current.borrow().clone());
    let current_bundle = bundle_for(&lang);

    translate(&current_bundle, key)
        .or_else(|| translate(&bundle_for("en-US"), key))
        .unwrap_or_else(|| key.to_string())
}
