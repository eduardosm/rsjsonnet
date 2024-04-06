use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::{Gc, GcContext, GcTrace, GcTraceCtx, GcView};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) struct Stats {
    pub(super) total: usize,
    pub(super) dead: usize,
    pub(super) maybe: usize,
    pub(super) view: usize,
}

impl GcContext {
    fn stats(&self) -> Stats {
        let inner = self.inner.borrow();
        let mut dead = 0;
        let mut maybe = 0;
        let mut view = 0;
        for obj in inner.objs.iter() {
            let has_strong = Rc::strong_count(obj) > 1;
            let has_weak = Rc::weak_count(obj) != 0;
            match (has_strong, has_weak) {
                (false, false) => dead += 1,
                (false, true) => maybe += 1,
                (true, _) => view += 1,
            }
        }
        Stats {
            total: inner.objs.len(),
            dead,
            maybe,
            view,
        }
    }

    #[track_caller]
    fn expect_stats(&self, stats: Stats) {
        assert_eq!(self.stats(), stats);
    }
}

#[test]
fn test_empty() {
    let ctx = GcContext::new();
    ctx.expect_stats(Stats {
        total: 0,
        dead: 0,
        maybe: 0,
        view: 0,
    });
    ctx.gc();
    ctx.expect_stats(Stats {
        total: 0,
        dead: 0,
        maybe: 0,
        view: 0,
    });
}

struct TestObj {
    id: u32,
    sub_objs: RefCell<Vec<Gc<Self>>>,
}

impl GcTrace for TestObj {
    fn trace(&self, ctx: &mut GcTraceCtx) {
        self.sub_objs.trace(ctx);
    }
}

enum TestAction {
    Create { id: u32, view: bool },
    Remove { id: u32, dead: bool },
    PushSub { dst_id: u32, src_id: u32 },
    CheckObj { id: u32 },
    CheckSubObj { id: u32, index: usize, sub_id: u32 },
    Gc { dead: usize, maybe: usize },
}

enum TestObjRef {
    Gc(Gc<TestObj>),
    View(GcView<TestObj>),
}

fn test(actions: &[TestAction]) {
    let ctx = GcContext::new();
    let mut objs = HashMap::new();
    let mut num_dead = 0;
    let mut num_extra_maybe = 0;

    for action in actions.iter() {
        match *action {
            TestAction::Create { id, view } => {
                if view {
                    let obj = ctx.alloc_view(TestObj {
                        id,
                        sub_objs: RefCell::new(Vec::new()),
                    });
                    let prev = objs.insert(id, TestObjRef::View(obj));
                    assert!(prev.is_none());
                } else {
                    let obj = ctx.alloc(TestObj {
                        id,
                        sub_objs: RefCell::new(Vec::new()),
                    });
                    let prev = objs.insert(id, TestObjRef::Gc(obj));
                    assert!(prev.is_none());
                }
            }
            TestAction::Remove { id, dead } => {
                drop(objs.remove(&id).unwrap());
                if dead {
                    num_dead += 1;
                } else {
                    num_extra_maybe += 1;
                }
            }
            TestAction::PushSub { dst_id, src_id } => {
                let src_obj = match objs.get(&src_id).unwrap() {
                    TestObjRef::Gc(obj) => obj.clone(),
                    TestObjRef::View(obj) => Gc::from(obj),
                };
                match objs.get(&dst_id).unwrap() {
                    TestObjRef::Gc(obj) => obj.view().sub_objs.borrow_mut().push(src_obj),
                    TestObjRef::View(obj) => obj.sub_objs.borrow_mut().push(src_obj),
                }
            }
            TestAction::CheckObj { id } => {
                let obj_id = match objs.get(&id).unwrap() {
                    TestObjRef::Gc(obj) => obj.view().id,
                    TestObjRef::View(obj) => obj.id,
                };
                assert_eq!(obj_id, id);
            }
            TestAction::CheckSubObj { id, index, sub_id } => {
                let sub_obj_id = match objs.get(&id).unwrap() {
                    TestObjRef::Gc(obj) => obj.view().sub_objs.borrow()[index].view().id,
                    TestObjRef::View(obj) => obj.sub_objs.borrow()[index].view().id,
                };
                assert_eq!(sub_obj_id, sub_id);
            }
            TestAction::Gc { dead, maybe } => {
                ctx.gc();
                num_dead -= dead;
                num_extra_maybe -= maybe;
            }
        }

        let total = objs.len() + num_dead + num_extra_maybe;
        ctx.expect_stats(Stats {
            total,
            dead: num_dead,
            maybe: objs
                .values()
                .filter(|obj| matches!(obj, TestObjRef::Gc(_)))
                .count()
                + num_extra_maybe,
            view: objs
                .values()
                .filter(|obj| matches!(obj, TestObjRef::View(_)))
                .count(),
        });
    }

    drop(objs);
    ctx.gc();
    ctx.expect_stats(Stats {
        total: 0,
        dead: 0,
        maybe: 0,
        view: 0,
    });
}

#[test]
fn test_simple() {
    test(&[
        TestAction::Create { id: 1, view: false },
        TestAction::CheckObj { id: 1 },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 0 },
    ]);

    test(&[
        TestAction::Create { id: 1, view: true },
        TestAction::CheckObj { id: 1 },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 0 },
    ]);
}

#[test]
fn test_acyclic() {
    test(&[
        TestAction::Create { id: 1, view: false },
        TestAction::Create { id: 2, view: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 1 },
    ]);

    // Created in reverse order
    test(&[
        TestAction::Create { id: 2, view: false },
        TestAction::Create { id: 1, view: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 1 },
    ]);

    // With view
    test(&[
        TestAction::Create { id: 1, view: true },
        TestAction::Create { id: 2, view: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 1 },
    ]);

    // With view, in reverse order
    test(&[
        TestAction::Create { id: 2, view: false },
        TestAction::Create { id: 1, view: true },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 1 },
    ]);

    // Twice in same object
    test(&[
        TestAction::Create { id: 1, view: false },
        TestAction::Create { id: 2, view: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::Remove { id: 1, dead: true },
        TestAction::Gc { dead: 1, maybe: 1 },
    ]);
}

#[test]
fn test_cyclic_one() {
    test(&[
        TestAction::Create { id: 1, view: false },
        TestAction::CheckObj { id: 1 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 1 },
    ]);

    // With view
    test(&[
        TestAction::Create { id: 1, view: true },
        TestAction::CheckObj { id: 1 },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 1 },
    ]);
}

#[test]
fn test_cyclic_two() {
    test(&[
        TestAction::Create { id: 1, view: false },
        TestAction::Create { id: 2, view: false },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::PushSub {
            dst_id: 2,
            src_id: 1,
        },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 2 },
    ]);

    // In reverse order
    test(&[
        TestAction::Create { id: 2, view: false },
        TestAction::Create { id: 1, view: false },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::PushSub {
            dst_id: 2,
            src_id: 1,
        },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 2 },
    ]);

    // With view
    test(&[
        TestAction::Create { id: 1, view: true },
        TestAction::Create { id: 2, view: true },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::PushSub {
            dst_id: 2,
            src_id: 1,
        },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 2 },
    ]);

    // With view, in reverse order
    test(&[
        TestAction::Create { id: 2, view: true },
        TestAction::Create { id: 1, view: true },
        TestAction::PushSub {
            dst_id: 1,
            src_id: 2,
        },
        TestAction::PushSub {
            dst_id: 2,
            src_id: 1,
        },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 1 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 1,
            index: 0,
            sub_id: 2,
        },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 1, dead: false },
        TestAction::Gc { dead: 0, maybe: 0 },
        TestAction::CheckObj { id: 2 },
        TestAction::CheckSubObj {
            id: 2,
            index: 0,
            sub_id: 1,
        },
        TestAction::Remove { id: 2, dead: false },
        TestAction::Gc { dead: 0, maybe: 2 },
    ]);
}

#[test]
fn test_doubly_linked_big() {
    const NUM_SETS: usize = 4;
    const NUM_OBJS: usize = if cfg!(miri) { 10 } else { 1000 };

    let ctx = GcContext::new();

    let mut sets_last = Vec::new();
    for _ in 0..NUM_SETS {
        let mut last = ctx.alloc_view(TestObj {
            id: 0,
            sub_objs: RefCell::new(Vec::new()),
        });

        for i in 1..NUM_OBJS {
            let obj = ctx.alloc_view(TestObj {
                id: i.try_into().unwrap(),
                sub_objs: RefCell::new(Vec::new()),
            });
            last.sub_objs.borrow_mut().push(Gc::from(&obj));
            obj.sub_objs.borrow_mut().push(Gc::from(&last));
            last = obj;
        }

        sets_last.push(Gc::from(&last));
    }

    drop(sets_last.pop().unwrap());
    ctx.gc();
    ctx.expect_stats(Stats {
        total: (NUM_SETS - 1) * NUM_OBJS,
        dead: 0,
        maybe: (NUM_SETS - 1) * NUM_OBJS,
        view: 0,
    });

    drop(sets_last);
    ctx.gc();
    ctx.expect_stats(Stats {
        total: 0,
        dead: 0,
        maybe: 0,
        view: 0,
    });
}

#[test]
fn test_doubly_linked_big_with_view() {
    const NUM_SETS: usize = 4;
    const NUM_OBJS: usize = if cfg!(miri) { 10 } else { 1000 };

    let ctx = GcContext::new();

    let mut sets_last = Vec::new();
    for _ in 0..NUM_SETS {
        let mut last = ctx.alloc_view(TestObj {
            id: 0,
            sub_objs: RefCell::new(Vec::new()),
        });

        for i in 1..NUM_OBJS {
            let obj = ctx.alloc_view(TestObj {
                id: i.try_into().unwrap(),
                sub_objs: RefCell::new(Vec::new()),
            });
            last.sub_objs.borrow_mut().push(Gc::from(&obj));
            obj.sub_objs.borrow_mut().push(Gc::from(&last));
            last = obj;
        }

        sets_last.push(last);
    }

    drop(sets_last.pop().unwrap());
    ctx.gc();
    ctx.expect_stats(Stats {
        total: (NUM_SETS - 1) * NUM_OBJS,
        dead: 0,
        maybe: (NUM_SETS - 1) * (NUM_OBJS - 1),
        view: NUM_SETS - 1,
    });

    drop(sets_last);
    ctx.gc();
    ctx.expect_stats(Stats {
        total: 0,
        dead: 0,
        maybe: 0,
        view: 0,
    });
}
