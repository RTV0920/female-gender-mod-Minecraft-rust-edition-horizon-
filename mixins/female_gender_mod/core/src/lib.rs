//! Female Gender Mod 5.1.0 for Horizon, ported from the Fabric mod by
//! WildfireMC (LGPL-3.0).
//!
//! The Fabric mod's settings, wire format and physics are kept exactly; this
//! crate is shared by the game, network and render mixin modules. See
//! `PORTING.md` for what is not ported (the cloud API and the custom hurt
//! sound assets).

pub mod config;
pub mod menu;
pub mod net;
pub mod physics;
pub mod state;
pub mod ui;

pub const MOD_ID: &str = "female_gender_mod";
pub const TITLE: &str = "Female Gender Mod";
pub const VERSION: &str = "5.1.0";
pub const UPSTREAM: &str =
    "https://github.com/WildfireMC/FemaleGenderMod (Female-Gender-Mod-5.0.0+mc26.3-fabric.jar)";
pub const LICENSE: &str = "LGPL-3.0-only";

/// The `config` key of Horizon's per-mod data folder.
pub const STORAGE_KEY: &str = "config";
