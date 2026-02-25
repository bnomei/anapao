// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Anapao-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Anapao and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

//! Minimal binary entrypoint for a library-first crate.

fn main() {
    eprintln!(
        "anapao is library-first. Use the crate API from your tests and tooling; this binary is intentionally minimal."
    );
}
