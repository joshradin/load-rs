use crate::holder::GoalHolder;
use crate::status::Outcome::{Failed, Skipped, Success};
use crate::status::Status;
use std::borrow::Borrow;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Index;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct GoalId(usize);

impl GoalId {
    const NONE: Self = GoalId(0);
}

impl Display for GoalId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for GoalId {
    fn from(u: usize) -> Self {
        GoalId(u)
    }
}

pub trait Goal<Holder: GoalHolder>: Sized {
    fn new(name: impl AsRef<str>, parent: &GoalId, holder: &Arc<RwLock<Holder>>) -> Self;

    fn name(&self) -> &String;
    fn parent_goal(&self) -> Option<&GoalId>;
    fn child_goals(&self) -> &[GoalId];

    /// This goal has started
    fn start(&mut self);
    /// This goal has finished
    ///
    /// # Panic
    /// Will panic if a sub goal hasn't finished and the given result is `Ok(())`
    fn finish(self, outcome: Status);
    /// Shortcut to finishing with an error
    fn fail(self, error: impl Error + 'static) {
        self.finish(Status::Finished(Failed(Box::new(error))))
    }
    /// Goal finishes as a success
    fn succeed(self) {
        self.finish(Status::Finished(Success))
    }
    /// Goals finishes as a skip
    fn skip(self) {
        self.finish(Status::Finished(Skipped))
    }

    fn sub_goal<G, F>(&mut self, name: impl AsRef<str>, goal: impl Into<Option<F>>) -> G
    where
        G: Goal<Holder>,
        F: FnOnce(&Holder, &mut G);
}

/// Some error occurred within a Goal
#[derive(Error, Debug)]
pub enum GoalError {
    #[error("Missing goal (id = {0})")]
    MissingGoal(GoalId),
    #[error("Missing goal (name = {0})")]
    MissingGoalName(String),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

pub struct DefaultGoal<Holder>
where
    Holder: GoalHolder,
{
    name: String,
    parent_goal: GoalId,
    my_id: GoalId,
    child_goals: Vec<GoalId>,
    holder: Arc<RwLock<Holder>>,
}

impl<Holder> Goal<Holder> for DefaultGoal<Holder>
where
    Holder: GoalHolder,
{
    fn new(name: impl AsRef<str>, parent: &GoalId, holder: &Arc<RwLock<Holder>>) -> Self {
        assert!(!name.as_ref().is_empty(), "Must have a name");
        assert_ne!(*parent, GoalId::NONE, "Must have a parent goal");
        Self {
            name: name.as_ref().to_string(),
            parent_goal: *parent,
            my_id: GoalId::NONE,
            child_goals: vec![],
            holder: holder.clone(),
        }
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn parent_goal(&self) -> Option<&GoalId> {
        Some(&self.parent_goal)
    }

    fn child_goals(&self) -> &[GoalId] {
        self.child_goals.as_slice()
    }

    fn start(&mut self) {
        todo!()
    }

    fn finish(self, outcome: Status) {
        todo!()
    }

    fn sub_goal<G, F>(&mut self, name: impl AsRef<str>, configure: impl Into<Option<F>>) -> G
    where
        G: Goal<Holder>,
        F: FnOnce(&Holder, &mut G),
    {

        let mut goal = G::new(name, &self.my_id, &self.holder);

        let holder_guard = self.holder.read().expect("Failed to get holder (poisoned)");
        let ref holder = *holder_guard;

        match configure.into() {
            None => {}
            Some(configure_func) => (configure_func)(holder, &mut goal),
        }

        drop(holder_guard);


        let mut holder = self
            .holder
            .write()
            .expect("Failed to get holder (poisoned)");

        holder.add_goal(&goal);

        goal
    }
}


pub struct RootGoal<H : GoalHolder> {
    inner_goal: DefaultGoal<H>
}

impl<H: GoalHolder> Goal<H> for RootGoal<H> {
    fn new(name: impl AsRef<str>, parent: &GoalId, holder: &Arc<RwLock<H>>) -> Self {
        panic!("Can not create a new RootGoal")
    }

    fn name(&self) -> &String {
        self.inner_goal.name()
    }

    fn parent_goal(&self) -> Option<&GoalId> {
        None
    }

    fn child_goals(&self) -> &[GoalId] {
        self.inner_goal.child_goals()
    }

    fn start(&mut self) {
        self.inner_goal.start()
    }

    fn finish(self, outcome: Status) {
        self.inner_goal.finish(outcome);
    }

    fn sub_goal<G, F>(&mut self, name: impl AsRef<str>, goal: impl Into<Option<F>>) -> G where G: Goal<H>, F: FnOnce(&H, &mut G) {
        self.inner_goal.sub_goal(name, goal)
    }
}

impl<H: GoalHolder> RootGoal<H> {
    pub fn new(name: impl AsRef<str>, holder: &Arc<RwLock<H>>) -> Self {
        Self {
            inner_goal: DefaultGoal {
                name: name.as_ref().to_string(),
                parent_goal: GoalId::NONE,
                my_id: GoalId::NONE,
                child_goals: vec![],
                holder: holder.clone()
            }
        }
    }
}
