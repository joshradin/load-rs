use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use crate::goal::{Goal, GoalId, RootGoal};
use crate::holder::GoalHolder;
use crate::status::Status;

pub struct GoalHandler<H: GoalHolder> {
    holder: Arc<RwLock<H>>,
    root_goal: RootGoal<H>
}

impl<H: GoalHolder> GoalHandler<H> {

    pub fn new(name: impl AsRef<str>, holder: H) -> Self {
        let arc = Arc::new(RwLock::new(holder));
        let goal = RootGoal::new(name, &arc);
        Self {
            holder: arc,
            root_goal: goal
        }
    }

    pub fn root_goal(&self) -> &RootGoal<H> {
        &self.root_goal
    }

    pub fn root_goal_mut(&mut self) -> &mut RootGoal<H> {
        &mut self.root_goal
    }

    pub fn all_goals(&self) -> impl IntoIterator<Item=GoalId> {
        let holder = self.holder.read().unwrap();
        let goal_id = holder.get_goal_id(self.root_goal.name()).unwrap();
        holder.get_all_children(goal_id).unwrap()
    }
}

pub struct GoalReport {
    pub total_goals: usize,
    pub in_progress: Vec<String>,

}
