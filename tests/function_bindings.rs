//! Generic functions dispatch to the implementation available for this call.
use std::cell::Cell;

pub trait Host {
    type Value;
    type Error;
    fn query(&self, id: u64) -> Result<Option<Self::Value>, Self::Error>;
}
struct NumericHost;
impl Host for NumericHost {
    type Value = u64;
    type Error = u8;
    fn query(&self, id: u64) -> Result<Option<u64>, u8> {
        Ok(Some(id))
    }
}
struct TextHost;
impl Host for TextHost {
    type Value = String;
    type Error = &'static str;
    fn query(&self, id: u64) -> Result<Option<String>, &'static str> {
        Ok(Some(id.to_string()))
    }
}
traitstub::deps! {
    query[H](host: &H, id: u64) -> Result<Option<H::Value>, H::Error>
        where [H: source::Host + ?Sized];
    enabled() -> bool;
}
traitstub::stubs! {
    query => |_, _| Ok(None);
    enabled => || false;
}
fn read<H: Host + ?Sized, D: Deps + calls::query<H>>(
    deps: &D,
    host: &H,
    id: u64,
) -> Result<Option<H::Value>, H::Error> {
    deps.query(host, id)
}

#[test]
fn production_bindings_forward_to_the_host_chosen_for_the_call() {
    let numeric = Deps(|host: &NumericHost, id| host.query(id), || true);
    let text = Deps(|host: &TextHost, id| host.query(id), || true);
    assert_eq!(read(&numeric, &NumericHost, 7), Ok(Some(7)));
    assert_eq!(read(&text, &TextHost, 7), Ok(Some("7".into())));
    assert!(numeric.enabled());
}
#[test]
fn captured_overrides_specialize_one_method_without_specializing_the_collection() {
    let calls = Cell::new(0);
    let value = 42;
    let deps = traitstub::defs! {
        query => |_: &NumericHost, id| {
            calls.set(calls.get() + 1);
            Ok(Some(value + id))
        };
    };
    assert_eq!(read(&deps, &NumericHost, 7), Ok(Some(49)));
    assert_eq!(calls.get(), 1);
    assert!(!deps.enabled());
    // The base remains universal, while this override supports NumericHost.
    assert_eq!(read(&Stubs, &TextHost, 7), Ok(None));
    let next = traitstub::defs! { target = deps; enabled => || true; };
    assert!(next.enabled());
    assert_eq!(read(&next, &NumericHost, 1), Ok(Some(43)));
    assert_eq!(calls.get(), 2);
}
