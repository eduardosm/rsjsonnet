#![warn(
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_qualifications
)]
#![forbid(unsafe_code)]

use rsjsonnet_lang::span::{SpanContext, SpanContextId, SpanManager};

#[test]
fn test_span_manager() {
    let mut mgr = SpanManager::new();

    let (ctx1_id, src1_id) = mgr.insert_source_context(3);
    let (ctx2_id, src2_id) = mgr.insert_source_context(5);

    assert_ne!(src1_id, src2_id);
    assert_eq!(*mgr.get_context(ctx1_id), SpanContext::Source(src1_id));
    assert_eq!(*mgr.get_context(ctx2_id), SpanContext::Source(src2_id));

    #[track_caller]
    fn test_span(mgr: &mut SpanManager, context: SpanContextId, start: usize, end: usize) {
        let span_id = mgr.intern_span(context, start, end);
        assert_eq!(mgr.get_span(span_id), (context, start, end));
    }

    for s in 0..=3 {
        for e in s..=3 {
            test_span(&mut mgr, ctx1_id, s, e);
        }
    }

    for s in 0..=5 {
        for e in s..=5 {
            test_span(&mut mgr, ctx2_id, s, e);
        }
    }
}
