use crate::goal::{Goal, GoalError, GoalId};
use crate::status::{Outcome, Status};
use more_collection_macros::{iter, map};
use petgraph::prelude::*;
use std::any::Any;
use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::ops::Index;
use crate::view::GoalHandler;

pub trait GoalHolder
where
    for<'a> Self: Sized
        + Index<&'a str, Output = GoalId>
        + Index<String, Output = GoalId>
        + Index<&'a String, Output = GoalId>,
{
    fn add_goal<G: Goal<Self>>(&mut self, goal: &G) -> GoalId;
    fn get_goal_outcome(&self, id: GoalId) -> Result<&Status, GoalError>;
    fn get_goal_id(&self, id: &String) -> Result<&GoalId, GoalError>;

    fn set_goal_status<G: Goal<Self>>(&mut self, id: &G, status: Status) -> Result<(), GoalError>;

    fn all_goals(&self) -> Vec<GoalId>;

    fn get_parent(&self, id: &GoalId) -> Result<GoalId, GoalError>;
    fn get_direct_children(&self, id: &GoalId) -> Result<HashSet<GoalId>, GoalError>;

    fn get_all_children(&self, id: &GoalId) -> Result<HashSet<GoalId>, GoalError> {
        let mut output = HashSet::new();
        let mut visited = HashSet::new();
        let mut id_stack = Vec::new();
        id_stack.push(*id);

        while let Some(id) = id_stack.pop() {
            if visited.contains(&id) {
                continue;
            } else {
                output.insert(id);
            }

            for children in self.get_direct_children(&id)? {
                id_stack.push(children);
            }

            visited.insert(id);
        }

        Ok(output)
    }

    fn all_children_finished(&self, id: &GoalId) -> Result<bool, GoalError> {
        let children = self.get_all_children(id)?;
        for child in children {
            let outcome = self.get_goal_outcome(child)?;
            match outcome {
                Status::Finished(_) => {}
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn any_child_errored(&self, id: &GoalId) -> Result<bool, GoalError> {
        let children = self.get_all_children(id)?;
        for child in children {
            let outcome = self.get_goal_outcome(child)?;
            match outcome {
                Status::Finished(Outcome::Failed(_)) => return Ok(true),
                _ => {}
            }
        }
        Ok(false)
    }
}

#[derive(Debug)]
pub struct DefaultHolder<G : GoalHandler<Self> = Self> {
    name_to_id: HashMap<String, GoalId>,
    all_ids: HashSet<GoalId>,
    goal_id_to_status: HashMap<GoalId, Status>,
    default_status: Status,
    next_id: usize,
    goal_graph: DiGraphMap<GoalId, ()>,
    goal_handler: G
}

impl<G : GoalHandler<Self>> Index<&str> for DefaultHolder<G> {
    type Output = GoalId;

    fn index(&self, index: &str) -> &Self::Output {
        self.get_goal_id(&index.to_string()).unwrap()
    }
}

impl<G : GoalHandler<Self>> Index<String> for DefaultHolder<G> {
    type Output = GoalId;

    fn index(&self, index: String) -> &Self::Output {
        self.get_goal_id(&index).unwrap()
    }
}

impl<G : GoalHandler<Self>> Index<&String> for DefaultHolder<G> {
    type Output = GoalId;

    fn index(&self, index: &String) -> &Self::Output {
        self.get_goal_id(index).unwrap()
    }
}

impl<GH : GoalHandler<Self>> GoalHolder for DefaultHolder<GH> {
    fn add_goal<G: Goal<Self>>(&mut self, goal: &G) -> GoalId {
        let next_id = self.next_id;
        self.next_id += 1;
        let goal_id = GoalId::from(next_id);

        let name_entry = self.name_to_id.entry(goal.name().clone());
        match name_entry {
            Entry::Occupied(_) => {
                panic!("Goal Already Exists (name = {})", goal.name());
            }
            Entry::Vacant(v) => {
                v.insert(goal_id);
            }
        };

        self.goal_graph.add_node(goal_id);

        if let Some(&p_id) = goal.parent_goal() {
            self.goal_graph.add_edge(p_id, goal_id, ());
        }

        for &c_id in goal.child_goals() {
            self.goal_graph.add_edge(goal_id, c_id, ());
        }

        goal_id
    }

    fn get_goal_outcome(&self, id: GoalId) -> Result<&Status, GoalError> {
        match self.all_ids.contains(&id) {
            true => Ok(self
                .goal_id_to_status
                .get(&id)
                .unwrap_or(&self.default_status)),
            false => Err(GoalError::MissingGoal(id)),
        }
    }

    fn get_goal_id(&self, id: &String) -> Result<&GoalId, GoalError> {
        self.name_to_id
            .get(id)
            .ok_or(GoalError::MissingGoalName(id.to_string()))
    }

    fn set_goal_status<G: Goal<Self>>(
        &mut self,
        goal: &G,
        status: Status,
    ) -> Result<(), GoalError> {
        let name = goal.name();
        let id = *self.get_goal_id(name)?;

        match self.goal_id_to_status.entry(id) {
            Entry::Occupied(mut occ) => {
                occ.insert(status);
            }
            Entry::Vacant(v) => {
                v.insert(status);
            }
        };

        let ref status = self.goal_id_to_status[&id];
        self.goal_handler.handle_goal_status_change(goal, status);

        Ok(())
    }

    fn all_goals(&self) -> Vec<GoalId> {
        self.all_ids.iter().copied().collect()
    }

    fn get_parent(&self, id: &GoalId) -> Result<GoalId, GoalError> {
        if !self.goal_graph.contains_node(*id) {
            return Err(GoalError::MissingGoal(*id));
        }
        let directed = self.goal_graph.neighbors_directed(*id, Direction::Incoming);
        let mut result: Vec<_> = directed.collect();
        if result.len() > 1 {
            panic!(
                "Can not have more than one parent (parents = {})",
                result.len()
            );
        }
        let p = result.remove(0);
        Ok(p)
    }

    fn get_direct_children(&self, id: &GoalId) -> Result<HashSet<GoalId>, GoalError> {
        if !self.goal_graph.contains_node(*id) {
            return Err(GoalError::MissingGoal(*id));
        }
        let directed = self.goal_graph.neighbors_directed(*id, Direction::Outgoing);
        Ok(directed.collect())
    }
}
