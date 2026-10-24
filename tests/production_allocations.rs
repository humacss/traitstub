use rstest::rstest;

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};
thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
        }
        // Forward the allocation request unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // ptr and layout are passed unchanged from the allocator's caller.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn twice(value: u64) -> u64 {
    value * 2
}
traitstub::deps! {
    add(value: u64) -> u64, |value| value + 1;
    double(value: u64) -> u64, twice;
}
traitstub::stubs! { add => |value| value + 10; double => |value| value * 2; }
fn calculate(deps: &impl Deps, value: u64) -> u64 {
    deps.add(value) + deps.double(value)
}
fn production_and_named_defaults() {
    let first = calculate(&Deps, std::hint::black_box(7));
    let defaults = Deps;
    let second = calculate(&defaults, std::hint::black_box(7));
    let named = calculate(&Defs, std::hint::black_box(7));
    assert_eq!((first, second, named), (22, 22, 31));
}

mod declarative {

    traitstub::deps! {
        add(value: u64) -> u64, |value| value + 1;
        double(value: u64) -> u64, |value| value * 2;
    }
    traitstub::stubs! {
        add => |value| value + 10;
        double => |value| value * 2;
    }
    pub fn calculate() -> (u64, u64, u64) {
        let calls = std::cell::Cell::new(0);
        let deps = traitstub::defs! {
            add(value: u64) -> u64 => |value| { calls.set(calls.get() + 1); value + 100 };
            double(value) => value * 2;
        };
        let result = (Deps.add(7), Defs.add(7), deps.add(7) + deps.double(7));
        assert_eq!(calls.get(), 1);
        result
    }
    pub fn calculate_raw_method() -> u64 {
        let deps = traitstub::defs! {
            add { fn add(&self, value: u64) -> u64 { value + 100 } }
        };
        deps.add(7) + deps.double(7)
    }
}
fn captured_override_layers() {
    let result = declarative::calculate();
    assert_eq!(result, (8, 17, 121));
}

fn raw_method_override_layers() {
    let result = declarative::calculate_raw_method();
    assert_eq!(result, 121);
}

mod parameterized {
    traitstub::deps! {
        for [T];
        value(input: &T) -> u64, |_| 1;
    }
    traitstub::stubs! { value => |_| 0; }
    pub fn run() {
        let adjustment = std::cell::Cell::new(2_u64);
        let deps = traitstub::defs! { value => |input: &u64| *input + adjustment.get(); };
        assert_eq!(deps.value(&7), 9);
        assert_eq!(Deps.value(&7), 1);
        assert_eq!(Defs.value(&7), 0);
        let deps = traitstub::defs! { target = deps; value => |input| *input + 3; };
        assert_eq!(deps.value(&7), 10);
    }
}

#[rstest]
#[case::production_and_shared(production_and_named_defaults)]
#[case::captured_layers(captured_override_layers)]
#[case::raw_methods(raw_method_override_layers)]
#[case::inferred_generic_captures(parameterized::run)]
fn implementations_need_no_infrastructure_allocations(#[case] operation: fn()) {
    ALLOCATIONS.with(|count| count.set(0));
    ACTIVE.with(|active| active.set(true));
    operation();
    ACTIVE.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
