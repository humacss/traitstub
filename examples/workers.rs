//! Generic tasks and captured state shared between threads.
//!
//! This file shows Fn, FnMut and FnOnce callback arguments, generic results and
//! thread bounds. Tests compare repeated reads, mutable captures and consuming
//! tasks, then share an atomic counter between concurrent callers.
//! These bounds describe callback arguments; captured dependency overrides use Fn.
//!
//! Read common.rs first for the basic setup.
//! Run with `cargo run --example workers` or `cargo test --example workers`.

traitstub::deps! {
    // Multiple function generics: every call chooses a task and its result type.
    // The where clause keeps the real launcher's FnOnce, Send and 'static bounds.
    spawn[T, F](name: String, run: F) -> std::io::Result<std::thread::JoinHandle<T>>
    where [T: Send + 'static, F: FnOnce() -> T + Send + 'static],
        |name, run| std::thread::Builder::new().name(name).spawn(run);

    // Fn: call through a shared reference, repeatedly. The callback can read
    // borrowed data; it does not need to mutate or consume its captures.
    run_twice[T, F](run: F) -> (T, T) where [F: Fn() -> T],
        |run| (run(), run());

    // FnMut: repeated calls may change captured data, such as a local counter.
    // The dependency owns run and can borrow it mutably without being FnMut
    // itself. These callbacks run locally, so no Send/'static bounds are needed.
    run_twice_mut[T, F](run: F) -> (T, T) where [F: FnMut() -> T],
        |mut run| (run(), run());

    // Captured instrumentation: production passes the value through. Tests can
    // count requests with an AtomicUsize without changing the work being tested.
    // Rust checks whether the captured data allows sharing the implementation.
    observe(value: u64) -> u64, |value| value;
}

pub fn work(value: u64) -> std::io::Result<u64> {
    work_with_deps(&Deps, value)
}

fn work_with_deps(deps: &impl Deps, value: u64) -> std::io::Result<u64> {
    let value = deps.observe(value);
    let worker = deps.spawn("example-worker".into(), move || value + 1)?;
    worker
        .join()
        .map_err(|_| std::io::Error::other("worker panicked"))
}

fn main() {
    println!("{:?}", work(41));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // A test must explicitly opt into launching a worker. No production
    // dependency runs just because a test forgot to supply its stub.
    traitstub::stubs! {
        spawn => |_name, _run| Err(std::io::Error::other("spawn is not configured"));
        run_twice => |_run| panic!("callback is not configured");
        run_twice_mut => |_run| panic!("callback is not configured");
        observe => |value| value;
    }

    #[test]
    fn the_public_entry_point_uses_the_real_launcher() {
        assert_eq!(work(41).unwrap(), 42);
    }

    #[test]
    fn one_implementation_accepts_different_tasks_and_results() {
        let deps = traitstub::defs! {
            spawn(name, run) => std::thread::Builder::new().name(name).spawn(run);
        };
        assert_eq!(work_with_deps(&deps, 41).unwrap(), 42);
        // This task consumes an owned String, exercising FnOnce and a result
        // type different from the u64 used by work on the same implementation.
        let owned = String::from("worker result");
        let worker = deps.spawn("text-worker".into(), move || owned).unwrap();
        assert_eq!(worker.join().unwrap(), "worker result");
    }

    #[test]
    fn fn_callbacks_can_read_captured_data_repeatedly() {
        let deps = traitstub::defs! {
            run_twice(run) => (run(), run());
        };
        let message = String::from("shared result");
        // Fn borrows the captured String and can return a slice on each call.
        assert_eq!(
            deps.run_twice(|| message.as_str()),
            ("shared result", "shared result")
        );
        // Ordinary function items also satisfy Fn (and FnMut and FnOnce).
        assert_eq!(deps.run_twice(u64::default), (0, 0));
        assert_eq!(Deps.run_twice(u64::default), (0, 0));
    }

    #[test]
    fn fn_mut_callbacks_can_modify_captured_data_between_calls() {
        let deps = traitstub::defs! {
            run_twice_mut => |mut run| (run(), run());
        };
        let mut count = 0;
        // This task needs FnMut because it changes its captured count. The
        // dependency override remains Fn: it only mutates its own run argument.
        let result = deps.run_twice_mut(|| {
            count += 1;
            count
        });
        assert_eq!(result, (1, 2));
        assert_eq!(count, 2);
        assert_eq!(
            Deps.run_twice_mut(|| {
                count += 1;
                count
            }),
            (3, 4)
        );
    }

    #[test]
    fn captured_counters_can_be_shared_by_concurrent_callers() {
        // Fn captures the atomic by reference. RefCell would fail Rust's Sync
        // checks here; Traitstub supplies no lock or special shared/local mode.
        let calls = AtomicUsize::new(0);
        let deps = traitstub::defs! {
            spawn(name, run) => std::thread::Builder::new().name(name).spawn(run);
            observe(value: u64) -> u64 => |value| {
                calls.fetch_add(1, Ordering::Relaxed);
                value
            };
        };
        std::thread::scope(|scope| {
            let a = scope.spawn(|| work_with_deps(&deps, 41).unwrap());
            let b = scope.spawn(|| work_with_deps(&deps, 7).unwrap());
            assert_eq!((a.join().unwrap(), b.join().unwrap()), (42, 8));
        });
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn a_launch_failure_does_not_change_the_shared_stubs() {
        let deps = traitstub::defs! {
            spawn(_name, _run) => Err(std::io::Error::other("injected failure"));
        };
        assert_eq!(
            work_with_deps(&deps, 41).unwrap_err().to_string(),
            "injected failure"
        );
        assert_eq!(
            work_with_deps(&Defs, 41).unwrap_err().to_string(),
            "spawn is not configured"
        );
    }

    #[test]
    fn a_worker_panic_becomes_a_consumer_error() {
        let deps = traitstub::defs! {
            spawn(name, run) => std::thread::Builder::new().name(name).spawn(move || {
                drop(run);
                panic!("injected worker panic");
            });
        };
        assert_eq!(
            work_with_deps(&deps, 41).unwrap_err().to_string(),
            "worker panicked"
        );
    }
}
