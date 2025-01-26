// Copyright 2025 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use embed_resource::CompilationResult;

extern crate embed_resource;

fn main() -> Result<(), CompilationResult> {
    let macros = vec![
        format!("CARGO_PKG_VERSION=\"{}\"", env!("CARGO_PKG_VERSION")),
        format!(
            "CARGO_PKG_VERSION_MAJOR={}",
            env!("CARGO_PKG_VERSION_MAJOR")
        ),
        format!(
            "CARGO_PKG_VERSION_MINOR={}",
            env!("CARGO_PKG_VERSION_MINOR")
        ),
        format!(
            "CARGO_PKG_VERSION_PATCH={}",
            env!("CARGO_PKG_VERSION_PATCH")
        ),
    ];

    embed_resource::compile("nt-load-order-gui.rc", macros).manifest_required()
}
