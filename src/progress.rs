use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use crate::holder::DefaultHolder;
use crate::view::GoalContainer;
use lazy_static::lazy_static;
use gag::BufferRedirect;

lazy_static! {
    static ref REDIRECTED_OUTPUT: RwLock<Option<BufferRedirect>> = RwLock::new(None);
}


pub trait ProgressView {
    fn start_progress(&mut self) {
        let result = REDIRECTED_OUTPUT.write().unwrap();
        *result = Some(BufferRedirect::stdout().unwrap());
    }

    fn end_progress(self) {
        let result = REDIRECTED_OUTPUT.write().unwrap();
        let buffer = std::mem::replace(&mut *result, None);
        drop(buffer);
    }
}


struct ProgressViewHolder<P : ProgressView> {
    view: P
}

pub struct BasicProgressView {
    goal_container: GoalContainer<DefaultHolder>
}
