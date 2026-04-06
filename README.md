<table>
<tr>
<td align="left" valign="middle">
<h3 align="left"> Air</h3>
</td>
<td align="left" valign="middle">
<h3 align="left">
 🪁
</h3>
</td>
		<td align="left" valign="middle">
			<h3 align="left"> + </h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">
					<picture>
						<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Land.svg">
						<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Land.svg">
						<img width="28" alt="Land Logo" src="https://PlayForm.Cloud/Image/GitHub/Land.svg">
					</picture>
				</a>
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				<a href="https://Editor.Land" target="_blank">
					Land
				</a>
			</h3>
		</td>
		<td align="left" valign="middle">
			<h3 align="left">
				🏞️
			</h3>
		</td>
	</tr>
</table>

---

# **Air**&#x2001;🪁

The Native Background Daemon for Land 🏞️

> **VS Code cold-starts slowly because everything initializes fresh each launch.
> Updates require a full restart that kills open terminals and in-progress work.
> There is no mechanism to pre-stage work between sessions.**

_"The next version is already downloaded and verified before you decide to
update. No restart prompt ever."_

[![License: CC0-1.0](https://img.shields.io/badge/License-CC0_1.0-lightgrey.svg)](https://github.com/CodeEditorLand/Air/tree/Current/LICENSE)
[<img src="https://editor.land/Image/Rust.svg" width="14" alt="Rust" />](https://www.rust-lang.org/)&#x2001;[![Crates.io](https://img.shields.io/crates/v/Air.svg)](https://crates.io/crates/Air)
[<img src="https://editor.land/Image/Rust.svg" width="14" alt="Rust" />](https://www.rust-lang.org/)&#x2001;[![Rust Version](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)

📖 **[Rust API Documentation](https://Rust.Documentation.Editor.Land/Air/)**

Welcome to **Air**, the lightweight, persistent daemon that powers the
background capabilities of the **Land Code Editor**. While `Mountain` handles
the core application logic and UI, **Air** operates as a specialized sidecar
process dedicated to heavy lifting, network operations, and system maintenance.
It ensures that the main editor remains responsive by offloading
resource-intensive tasks such as updates, large downloads, and cryptographic
signing.

**Air** acts as the silent partner to `Mountain`, providing a robust server
environment that persists even when the main editor window is closed, enabling
seamless background updates and persistent state management.

---

## Key Features&#x2001;🔐

- **Native Sidecar Architecture:** Runs as a standalone process alongside the
  main `Mountain` application, communicating via high-performance IPC
  (gRPC/Vine) to handle requests without blocking the UI thread.
- **Dedicated Update Management:** Takes full control of the update lifecycle,
  including downloading, verifying, and applying patches for `Land`, ensuring
  the editor is always up-to-date without user interruption.
- **Isolated Authentication & Signing:** Manages sensitive cryptographic
  operations, including binary signing and secure login flows, keeping security
  logic isolated from the main application view.
- **Background Downloader:** Implements a resilient download manager for
  extensions, language servers, and dependencies, capable of pausing, resuming,
  and handling network interruptions gracefully.
- **Resource Offloading:** Acts as the designated handler for any "heavy" task
  that doesn't strictly require the main application loop, effectively
  decoupling infrastructure maintenance from the user experience.

---

## Deep Dive & Component Breakdown&#x2001;🔬

To understand how `Air`'s internal components interact to provide the background
daemon functionality, see the following source files:

- **[`Source/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/)** -
  Main daemon implementation
- **[`Source/Update/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Update/)** -
  Update lifecycle management
- **[`Source/Download/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Download/)** -
  Resilient download manager
- **[`Source/Auth/`](https://github.com/CodeEditorLand/Air/tree/Current/Source/Auth/)** -
  Authentication and cryptographic signing

The source files explain the gRPC server implementation, task delegation from
Mountain, and the progress event emission patterns.

---

## System Architecture Diagram&#x2001;🏗️

This diagram illustrates how **Air** sits alongside `Mountain` to handle
background operations.

```mermaid
graph LR
    classDef mountain fill:#f9f,stroke:#333,stroke-width:2px;
    classDef Air fill:#9cf,stroke:#333,stroke-width:2px;
    classDef external fill:#ddd,stroke:#666,stroke-dasharray: 5 5;

    subgraph "Land Runtime Ecosystem"
        direction TB

        subgraph "Mountain ⛰️ (Main App)"
            UI[User Interface]:::mountain
            CoreLogic[Core Logic]:::mountain
            CoreLogic -- Requests Task --> IPC_Client
        end

        subgraph "Air 🪁 (Daemon Sidecar)"
            IPC_Server[gRPC Server]:::Air
            UpdateMgr[Update Manager]:::Air
            Downloader[Resilient Downloader]:::Air
            AuthService[Signer & Auth]:::Air

            IPC_Server -- Routes to --> UpdateMgr
            IPC_Server -- Routes to --> Downloader
            IPC_Server -- Routes to --> AuthService
        end

        IPC_Client -- IPC (Vine Protocol) --> IPC_Server
    end

    subgraph "External World"
        Cloud[Update Servers / Registry]:::external
    end

    UpdateMgr -- Fetches --> Cloud
    Downloader -- Downloads --> Cloud
```

---

## `Air` in the Land Ecosystem&#x2001;🪁 + 🏞️

| Component           | Role \& Key Responsibilities                                                         |
| :------------------ | :----------------------------------------------------------------------------------- |
| **Daemon Process**  | The persistent executable that runs independently of the main window.                |
| **Server Host**     | Hosts a local server to accept commands from `Mountain` or other authorized clients. |
| **Update Delegate** | The sole authority for modifying the installation files of the parent application.   |
| **Signer**          | Handles cryptographic signing of artifacts and secure token storage for user login.  |
| **Traffic Manager** | Acts as a proxy/downloader to keep network load off the main renderer process.       |

---

## Getting Started&#x2001;🚀

### Installation&#x2001;📥

To add `Air` to your project workspace:

```toml
[dependencies]
Air = { git = "https://github.com/CodeEditorLand/Air.git", branch = "Current" }
```

### Usage Pattern&#x2001;🚀

**Air** is typically spawned automatically by `Mountain` during the startup
phase.

1. **Spawn:** `Mountain` detects if `Air` is running. If not, it spawns the
   binary.
2. **Connect:** `Mountain` establishes a Vine (gRPC) connection to `Air`'s local
   port `[::1]:50053` (reserved for Air, separate from Cocoon's port 50052).
3. **Delegate:** When a user requests an update or a large download, `Mountain`
   sends a command to `Air` and immediately returns control to the user.
4. **Monitor:** `Air` emits progress events back to `Mountain` to update the UI
   status bars.

### Port Allocation

- **Air**: Port `50053` (Vine/Air.proto protocol - Air daemon services)
- **Cocoon**: Port `50052` (Vine.proto protocol - VS Code extension hosting)

---

## See Also

- [Air Documentation](https://editor.land/Doc/air)
- [Architecture Overview](https://editor.land/Doc/architecture)
- [Why Rust](https://editor.land/Doc/why-rust)
- [Mountain](https://github.com/CodeEditorLand/Mountain)
- [Vine](https://github.com/CodeEditorLand/Vine)
- [Echo](https://github.com/CodeEditorLand/Echo)
- [Mist](https://github.com/CodeEditorLand/Mist)

---

## License&#x2001;⚖️

This project is released into the public domain under the **Creative Commons CC0
Universal** license. You are free to use, modify, distribute, and build upon
this work for any purpose, without any restrictions. For the full legal text,
see the [`LICENSE`](https://github.com/CodeEditorLand/Air/tree/Current/) file.

---

## Changelog&#x2001;📜

Stay updated with our progress! See
[`CHANGELOG.md`](https://github.com/CodeEditorLand/Air/tree/Current/) for a
history of changes specific to **Air**.

---

## Funding \& Acknowledgements&#x2001;🙏🏻

**Air** is a core element of the **Land** ecosystem. This project is funded
through [NGI0 Commons Fund](https://NLnet.NL/commonsfund), a fund established by
[NLnet](https://NLnet.NL) with financial support from the European Commission's
[Next Generation Internet](https://ngi.eu) program. Learn more at the
[NLnet project page](https://NLnet.NL/project/Land).

The project is operated by PlayForm, based in Sofia, Bulgaria.

PlayForm acts as the open-source steward for Code Editor Land under the NGI0
Commons Fund grant.

<table>
	<thead>
		<tr>
			<th align="left"><strong>Land</strong></th>
			<th align="left"><strong>PlayForm</strong></th>
			<th align="left"><strong>NLnet</strong></th>
			<th align="left"><strong>NGI0 Commons Fund</strong></th>
		</tr>
	</thead>
	<tbody>
		<tr>
			<td align="left" valign="middle">
				<a href="https://Editor.Land">
					<img width="60" src="https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Land.svg" alt="Land">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://PlayForm.Cloud">
					<img width="76" src="https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg" alt="PlayForm">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL">
					<img width="240" src="https://NLnet.NL/logo/banner.svg" alt="NLnet">
				</a>
			</td>
			<td align="left" valign="middle">
				<a href="https://NLnet.NL/commonsfund">
					<img width="240" src="https://NLnet.NL/image/logos/NGI0CommonsFund_tag_black_mono.svg" alt="NGI0 Commons Fund">
				</a>
			</td>
		</tr>
	</tbody>
</table>

---

**Project Maintainers**: Source Open
([Source/Open@Editor.Land](mailto:Source/Open@Editor.Land)) |
[GitHub Repository](https://github.com/CodeEditorLand/Air) |
[Report an Issue](https://github.com/CodeEditorLand/Air/issues) |
[Security Policy](https://github.com/CodeEditorLand/Air/security/policy)
