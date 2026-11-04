// SPDX-FileCopyrightText: 2026 Framehive contributors
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests of the guided import (CLI-002, CLI-003). The golden tests run the dialog against a
//! scripted terminal, memory cards in memory and a manual clock.

#![allow(clippy::unwrap_used, reason = "test helpers may panic on bad fixtures")]

mod backup;
mod environment;
mod history;
mod import;
mod naming;
mod readiness;
mod sequence;
mod start;
mod state;
mod summary;
mod support;
