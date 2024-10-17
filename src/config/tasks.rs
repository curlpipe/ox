/// For dealing with tasks (part of the plug-in concurrency API)

#[derive(Default, Debug)]
pub struct Task {
    repeat: bool,
    delay: isize,
    remaining: isize,
    target: String,
}

/// A struct in charge of executing functions concurrently
#[derive(Default, Debug)]
pub struct TaskManager {
    pub tasks: Vec<Task>,
    pub to_execute: Vec<String>,
}

impl TaskManager {
    /// Thread to run and keep track of which tasks to execute
    pub fn cycle(&mut self) {
        for task in &mut self.tasks {
            // Decrement remaining time
            if task.remaining > 0 {
                task.remaining = task.remaining.saturating_sub(1);
            }
            // Check if activation is required
            if task.remaining == 0 {
                self.to_execute.push(task.target.clone());
                // Check whether to repeat or not
                if task.repeat {
                    // Re-load the task
                    task.remaining = task.delay;
                } else {
                    // Condemn the task to decrementing forever
                    task.remaining = -1;
                }
            }
        }
    }

    /// Obtain a list of functions to execute (and remove them from the execution list)
    pub fn execution_list(&mut self) -> Vec<String> {
        let mut new = vec![];
        std::mem::swap(&mut self.to_execute, &mut new);
        new
    }

    /// Define a new task
    pub fn attach(&mut self, delay: isize, target: String, repeat: bool) {
        self.tasks.push(Task {
            remaining: delay,
            delay,
            target,
            repeat,
        });
    }
}
