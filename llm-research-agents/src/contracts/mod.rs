//! Agentics Contracts Module
//!
//! This module defines ALL research, hypothesis, and metric schemas used by
//! LLM-Research-Lab agents. These contracts are the authoritative source for
//! agent input/output types and DecisionEvent structures.
//!
//! # Constitution Compliance
//!
//! Per PROMPT 0 (LLM-RESEARCH-LAB AGENT INFRASTRUCTURE CONSTITUTION):
//! - All agents import schemas EXCLUSIVELY from this module
//! - All inputs and outputs are validated against these contracts
//! - DecisionEvents follow the mandated schema

pub mod common;
pub mod decision_event;
pub mod hypothesis;
pub mod metrics;

pub use common::*;
pub use decision_event::*;
pub use hypothesis::*;
pub use metrics::*;
