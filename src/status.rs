use std::error::Error;

#[derive(Debug)]
pub enum Outcome {
    Skipped,
    Success,
    Failed(Box<dyn Error>),
}

#[derive(Debug)]
pub enum Status {
    Waiting,
    InProgress,
    Finished(Outcome),
}
