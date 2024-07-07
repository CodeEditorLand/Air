#![allow(non_snake_case)]

pub mod Yell;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Represents different types of actions that can be performed.
///
/// # Variants
///
/// * `Read` - Represents a read action with a specified file path.
/// * `Write` - Represents a write action with a specified file path and content.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Action {
	/// Read action with the specified file path.
	Read { Path: String },

	/// Write action with the specified file path and content.
	Write { Path: String, Content: String },
}

/// Represents the result of an action that has been processed.
///
/// # Fields
///
/// * `Action` - The action that was processed.
/// * `Result` - The result of the action, which is a `Result` type containing either a success message (`String`) or an error message (`String`).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionResult {
	pub Action: Action,
	pub Result: Result<String, String>,
}

/// A trait that defines the behavior for processing actions.
///
/// Types that implement this trait must be able to handle actions asynchronously.
#[async_trait::async_trait]
pub trait Worker: Send + Sync {
	/// Processes a given action and returns the result.
	///
	/// # Arguments
	///
	/// * `Action` - The action to be processed.
	///
	/// # Returns
	///
	/// An `ActionResult` containing the result of the action.
	async fn Receive(&self, Action: Action) -> ActionResult;
}

/// Represents a work queue that holds actions to be processed.
pub struct Work {
	Queue: Arc<Mutex<Vec<Action>>>,
}

impl Work {
	/// Creates a new `Work` instance with an empty queue.
	///
	/// # Returns
	///
	/// A new `Work` instance
	pub fn Begin() -> Self {
		Work { Queue: Arc::new(Mutex::new(Vec::new())) }
	}

	/// Assigns a new action to the work queue.
	///
	/// # Arguments
	///
	/// * `Action` - The action to be added to the queue.
	pub async fn Assign(&self, Action: Action) {
		self.Queue.lock().await.push(Action);
	}

	/// Executes the next action from the work queue.
	///
	/// # Returns
	///
	/// An `Option` containing the next action if available, or `None` if the queue is empty.
	pub async fn Execute(&self) -> Option<Action> {
		self.Queue.lock().await.pop()
	}
}

/// Asynchronously processes actions from a work queue and sends the results to an approval channel.
///
/// # Arguments
///
/// * `Site` - An `Arc` reference to a type that implements the `Worker` trait. This is used to process the actions.
/// * `Work` - An `Arc` reference to a `Work` instance that contains the queue of actions to be processed.
/// * `Approval` - An unbounded sender channel to send the results of the processed actions.
///
/// # Behavior
///
/// This function runs an infinite loop where it continuously checks for actions in the `Work` queue.
/// If an action is found, it is processed by the `Site` and the result is sent to the `Approval` channel.
/// If sending the result fails, the loop breaks. If no action is found, the function sleeps for 100 milliseconds
/// before checking again.
pub async fn Fn(
	Site: Arc<dyn Worker>,
	Work: Arc<Work>,
	Approval: tokio::sync::mpsc::UnboundedSender<ActionResult>,
) {
	loop {
		if let Some(Action) = Work.Execute().await {
			if Approval.send(Site.Receive(Action).await).is_err() {
				break;
			}
		} else {
			tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		}
	}
}
