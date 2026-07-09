//! Compatibility names for projects that still compile against MiniForge 0.9.3.
//!
//! Rhai itself is no longer embedded or executed. New code must import
//! `engine::luau_scripting`; these aliases will be removed in the next breaking
//! API release.

#[deprecated(note = "Rhai was removed; use LuauRunReport")]
pub type RhaiRunReport = super::luau_scripting::LuauRunReport;
#[deprecated(note = "Rhai was removed; use LuauScriptRuntime")]
pub type RhaiScriptRuntime = super::luau_scripting::LuauScriptRuntime;

pub use super::luau_scripting::{
    ScriptCommand, ScriptDebugSnapshot, ScriptTarget, ScriptTraceEntry,
};
