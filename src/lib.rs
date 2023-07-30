//! # `DroidWorks`
//!
//! `droidworks` is the main crate of the `DroidWorks` Android application analysis
//! project. The project is subdivided into multiple crates, `droidworks` acts as
//! entry point by reexporting important structs and functions from those
//! sub-crates. Most of the reexport are done within the `droidworks::prelude`
//! namespace.
//!
//! ## Library basics
//!
//! Depending on what kind of dex manipulation you aim using the `DroidWorks`
//! library, there are two main ways to open a dex file.
//!
//! For manipulations and inspections closed to the dex format, one should use
//! the `dw_dex` API:
//!
//! ```rust
//! use droidworks::prelude::*;
//! use droidworks::dex;
//!
//! let dex = dex::open("apk/pickcontacts.dex")?;
//! println!("dex version: {}", dex.version());
//! # Ok::<(), DwError>(())
//! ```
//!
//! For more advanced analysis, such as building control flow graphs, inspecting
//! classes hierarchy, or typechecking the bytecode, the abstraction called `Repo`
//! stores objects (classes, interfaces, etc.) and offer analysis API:
//!
//! ```rust
//! use droidworks::prelude::*;
//! use droidworks::dex;
//!
//! let dex = dex::open("apk/pickcontacts.dex")?;
//! let mut repository = Repo::new();
//! repository.register_dex(&dex, false)?;
//! println!("classes count: {}", repository.nb_classes());
//! println!("methods count: {}", repository.nb_classes());
//! # Ok::<(), DwError>(())
//! ```
//!
//! A more complete example on how to crawl a `Repo` to inspect objects is
//! available as a tool binary ([bin/example.rs](../src/example/example.rs.html#41)).
//!
//! ## Sub-crates
//!
//! The `DroidWorks` project is divided into several crates. Some of them as
//! (completely or partially) re-exported as parts of [`prelude`], but some
//! features may be accessible only by importing a given sub-crate. Here is a list
//! of those sub-crates:
//!
//!  - [`dw_package`] (apk), [`dw_dex`] and [`dw_resources`] (Android resources
//!    and Android manifest files) contain the definitions, types and basic accessors,
//!    setters and constructors for the various file formats that
//!    are manipulated in the context of Android applications analysis,
//!  - [`dw_analysis`] contain all the analysis algorithms and rely heavily on the
//!    previously cited crates,
//!  - [`dw_utils`] contain the small functions all the other crates can benefit.

mod errors;

pub mod cli;
pub mod dw_aresources;
pub mod dw_callgraph;
pub mod dw_dexdissect;
pub mod dw_disas;
pub mod dw_hierarchy;
pub mod dw_manifest;
pub mod dw_nsc;
pub mod dw_packageinfo;
pub mod dw_permissions;
pub mod dw_stats;
pub mod dw_strip;
pub mod dw_typecheck;
pub mod owndex;

pub use dw_analysis as analysis;
pub use dw_dex as dex;
pub use dw_resources as resources;
pub use dw_utils as utils;

/// Reexport module of commonly used structures and functions from `DroidWorks` project
/// sub-crates:
///
/// ```rust
/// use droidworks::prelude::*;
/// ```
pub mod prelude {
    pub use crate::errors::{DwError, DwResult};

    pub use dw_analysis::callgraph;
    pub use dw_analysis::controlflow;
    pub use dw_analysis::repo::{Class, Field, Method, Repo};

    pub use dw_dex::{Addr, Dex};

    pub use dw_package::{errors::PackageError, Options as PackageOptions, Package};

    pub use dw_resources::{
        errors::ResourcesError,
        manifest::{Manifest, ManifestTag},
    };

    use clap::ArgMatches;

    pub fn init_logger(args: &ArgMatches) {
        let env = env_logger::Env::new()
            .filter_or("DW_LOG", "info")
            .write_style("DW_LOG_STYLE");

        let mut builder = env_logger::Builder::from_env(env);
        if args.get_flag("verbose") {
            builder.filter_level(log::LevelFilter::Trace);
        } else if args.get_flag("debug") {
            builder.filter_level(log::LevelFilter::Debug);
        }
        if args.get_flag("ecslog") {
            builder.format(ecs_logger::format);
        }
        builder.init();
    }
}
