//! # Knowledge Base Prompts
//!
//! This module contains prompts specifically for the knowledge base creation pipeline,
//! including the two-pass distillation and augmentation process.

// --- RAG (Retrieval-Augmented Generation) Prompts ---

/// The system prompt for synthesizing an answer from retrieved knowledge base context.
/// This instructs the AI to answer only based on the provided context.
pub const KNOWLEDGE_RAG_SYSTEM_PROMPT: &str = "You are a strict, factual AI. Your sole purpose is to answer the user's question based *only* on the provided #Context. A 'Definitive Answer from Knowledge Graph' takes absolute priority; if present, you MUST use it as your answer and ignore other context. If no definitive answer is provided, synthesize an answer from the rest of the context. You MUST NOT use any external knowledge or make assumptions. If the context is insufficient, state that you cannot answer.";

/// The user prompt for the RAG synthesis step.
/// This structures the input with the user's query and the retrieved context.
/// Placeholders: `{prompt}`, `{context}`
pub const KNOWLEDGE_RAG_USER_PROMPT: &str = r#"# User Question
{prompt}

# Context
{context}

# Your Answer:"#;
