# 📣 [Echo] — Asynchronous Action Processing System

`Echo` is a Rust library designed for managing and executing asynchronous
actions efficiently. It leverages a worker-stealer pattern and asynchronous
queues to handle complex workflows with features like metadata management,
function planning, and robust error handling.

## Table of Contents

-   [Introduction](#Introduction)
-   [Features](#Features)
-   [Installation](#Installation)
-   [Usage](#Usage)
-   [Architecture](#Architecture)
-   [Contributing](CONTRIBUTING.md)
-   [License](LICENSE)

## Introduction

`Echo` provides a robust framework for defining, queuing, and executing actions
asynchronously. It's designed to handle complex workflows with features like
metadata management, function planning, and error handling.

## Features

-   **Asynchronous Operations:** Built with Rust's async/await syntax for
    non-blocking execution.
-   **Action Planning:** Define and execute actions with custom logic using a
    flexible Plan system.
-   **Metadata Management:** Attach metadata to actions for additional Life and
    control.
-   **Error Handling:** Comprehensive error management with custom `Error`
    types.
-   **Retry Mechanism:** Built-in retry logic for failed actions with
    exponential backoff.
-   **Hooks:** Supports pre and post-execution hooks for added flexibility.
-   **Serialization:** Actions can be serialized and deserialized for
    persistence or network transfer (in progress).

## Installation

To get started with `Echo`, follow these steps:

1. **Add to your Cargo.toml**:

```toml
[dependencies]
Echo = { git = "HTTPS://GitHub.Com/CodeEditorLand/Echo.git" }
```

2. **Build the Project**:

```bash
cargo build
```

## Usage

Here's a basic example demonstrating how to define and execute an Action:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Define the Action's logic
	let Read = |_Argument: Vec<serde_json::Value>| async move {
		// Access the provided path (replace with actual logic)
		let Path = "path/to/file.txt";

		// Simulate reading from the path
		let Content = format!("Content read from: {}", Path);

		Ok(json!(Content))
	};

	// Create an Action Plan
	let Plan = Plan::New()
		.WithSignature(Echo::Struct::Sequence::Action::Signature::Struct {
			Name: "Read".to_string(),
		})
		.WithFunction("Read", Read)?
		.Build();

	// Create a work queue
	let Production = Arc::new(Production::New());

	// Create a lifecycle Life (replace with your actual configuration)
	let Life = Life::Struct {
		Span: Arc::new(dashmap::DashMap::new()),
		Fate: Arc::new(config::Config::default()),
		Cache: Arc::new(Mutex::new(dashmap::DashMap::new())),
		Karma: Arc::new(dashmap::DashMap::new()),
	};

	// Define a Site to execute actions
	struct SimpleSite;

	#[async_trait::async_trait]
	impl Site for SimpleSite {
		async fn Receive(
			&self,
			Action: Box<dyn ActionTrait>,
			Life: &Life,
		) -> Result<(), Error> {
			Action.Execute(Life).await
		}
	}
	let Site = Arc::new(SimpleSite);

	// Create an Action Sequence
	let Sequence = Arc::new(Sequence::New(Site, Production.clone(), Life));

	// Create an Action and add it to the queue
	let Action = Action::New(
		"Read",
		json!("SomeData"),
		Arc::clone(&Plan),
	);

	Production.Assign(Box::new(Action)).await;

	// Run the Sequence
	Sequence.Run().await;

	Ok(())
}

use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

use Echo::Sequence::{
	Action::{Error::Enum as Error, Struct as Action, Trait as ActionTrait},
	Life::Struct as Life,
	Plan::{Formality::Struct as Formality, Struct as Plan},
	Production::Struct as Production,
	Site::Trait as Site,
	Struct as Sequence,
};
```

## Architecture

### Core Components

-   **Action:** Represents a unit of Production with associated metadata,
    content, and execution logic.
-   **Plan:** Defines the structure and functions for different Action types.
-   **Production:** A thread-safe queue for managing pending actions.
-   **Site:** Implements the logic for receiving and executing actions from the
    queue.
-   **Sequence:** Orchestrates the execution of actions using workers and the
    work queue.
-   **Life:** Provides a shared Life and configuration for actions during
    execution.

### Diagrams

#### Class Diagram

```mermaid
classDiagram
    class Action {
        +Metadata: Vector
        +Content
        +License: Signal
        +Plan
        +New() Action
        +WithMetadata() Struct
        +Execute() Result
        -License() Result
        -Delay() Result
        -Hooks() Result
        -Function() Result
        -Next() Result
        -Argument() Result
        -Result() Result
        Represents an action with metadata, content, license, and plan.
    }

    class Life {
        +Span
        +Fate
        +Cache
        +Karma
        Represents the lifecycle context for a sequence of actions.
    }

    class Formality {
        -Signature
        -Function
        +New() Formality
        +Sign() Formality
        +Add() Result
        +Remove() Option
        A structure that manages signatures and functions for actions.
    }

    class Production {
        -Line
        +New() Production
        +Do() Option
        +Assign()
        Represents a thread-safe queue of actions to be processed.
    }

    class Signal {
        -value
        +New() Signal
        +Get()
        +Set()
        A thread-safe wrapper around a value.
    }

    class Vector {
        -Entry
        +New() Vector
        +Insert()
        +Get() Option
        A thread-safe key-value store.
    }

    class Signature {
        +Name: String
        Represents a signature for an action.
    }

    class enum Error {
        License
        Execution
        Routing
        Cancellation
        Represents various error types that can occur during sequence actions.
    }

    Action --> Vector : has
    Action --> Signal : has
    Plan --> Formality : has
    Life --> "*" Production : has
    Production --> "*" Action : contains
    Formality --> "*" Signature: has
    Action ..> Error : throws
    Trait~Action~ <|.. Action
    Trait~Site~ -- Action: uses
```

#### Sequence Diagram

```mermaid
sequenceDiagram
    participant Client
    participant Action
    participant Metadata
    participant License
    participant Context
    participant Plan
    participant Hooks
    participant Function

    activate Client
    Client->>Action: Execute(Context)
    activate Action
    Note right of Action: The client initiates the execution of an action represented by the 'Action' object
	
    Action->>Metadata: Get("Action")
    alt "Action" not found
        Action->>Action: Return Error
        Note right of Action: Returns an error if "Action" is not found in the metadata
    else "Action" found
        Metadata-->>Action: Return Action
        Action->>License: Get()
        alt License Invalid
            Action->>Action: Return Error
            Note right of Action: Return an error if the action is not properly licensed
        else License Valid
            Action->>Metadata: Get("Delay")
            alt Delay exists
                Metadata-->>Action: Return Delay
                Action->>Action: sleep(Delay)
                Note right of Action: If a delay is specified, wait for the given duration
            end
            Action->>Metadata: Get("Hooks")
            alt Hooks exist
                Metadata-->>Action: Return Hooks
                loop Hook in Hooks
                    Action->>Context: Span.get(Hook)
                    alt Hook Function found
                        Context-->>Action: Return HookFn
                        Action->>HookFn: call()
                        alt HookFn Error
                            Action->>Action: Return Error
                            Note right of Action: If a hook function returns an error, stop execution and return the error 
                        end
                    end
                end
            end
            Action->>Plan: Remove(Action)
            alt Function not found
                Action->>Action: Return Error
                Note right of Action: Return an error if no function is found for the given action
            else Function found
                Plan-->>Action: Return Function
                Action->>Action: Argument()
                Action->>Function: call(Argument)
                activate Function
                Function-->>Action: Return Result
                deactivate Function
                alt Function Error
                    Action->>Action: Return Error
                    Note right of Action: If the function execution returns an error, propagate the error
                else Function Success
                    Action->>Action: Result(Result)
                    Action->>Metadata: Get("NextAction")
                    alt NextAction exists
                        Metadata-->>Action: Return NextAction
                        Action->>Action: Execute(NextAction, Context)
                        alt NextAction Error
                            Action->>Action: Return Error
                            Note right of Action: If the execution of the next action results in an error, return the error
                        end
                    end 
                end
            end
        end
    end
    deactivate Action
    Client->>Client: Return Result
    Note right of Client: Returns the result of the action execution, which can be a success or an error

```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for
guidelines.

## License

This project is licensed under the [LICENSE](LICENSE) file.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a history of changes to this component.

[Echo]: HTTPS://GitHub.Com/CodeEditorLand/Echo
