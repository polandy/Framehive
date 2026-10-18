// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Kernel of Framehive: the module contract, the module registry, the event bus and
//! commands (architecture §4).
//!
//! The kernel knows modules only through the contract. Modules never talk to each
//! other directly; they use events and commands defined here.

#![forbid(unsafe_code)]

mod module_id;

pub use module_id::{ModuleId, ModuleIdError};
