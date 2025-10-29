use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uwuwind::uw;

#[repr(C)]
struct Exception {
    _uwe: uw::_Unwind_Exception,
    uwu: &'static str,
}

fn main() {
    let registry = tracing_subscriber::Registry::default().with(
        EnvFilter::builder()
            .with_default_directive(tracing::Level::TRACE.into())
            .from_env()
            .unwrap(),
    );

    let tree_layer = tracing_tree::HierarchicalLayer::new(2)
        .with_targets(true)
        .with_bracketed_fields(true);

    registry.with(tree_layer).init();
    unsafe {
        let exception = Box::into_raw(Box::new(Exception {
            _uwe: uw::_Unwind_Exception {
                exception_class: 123456,
                exception_cleanup0: |_, _| {},
                private_1: 0,
                private_2: 0,
            },
            uwu: "meow :3",
        }));

        uwuwind::_UnwindRaiseException(exception.cast::<uw::_Unwind_Exception>());
    }
}
