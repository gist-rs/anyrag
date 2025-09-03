//! # RSS-Specific Prompts
//!
//! This module contains prompt templates specifically tailored for processing
//! content from RSS feeds.

/// The system prompt for handling RSS feed content.
pub const RSS_QUERY_SYSTEM_PROMPT: &str = "You are an AI assistant that specializes in analyzing and summarizing content from RSS feeds. Answer the user's question based on the provided article snippets.";

/// The user prompt for handling RSS feed content.
pub const RSS_QUERY_USER_PROMPT: &str = "# User Question\n{prompt}\n\n# Article Content\n{context}"; //
