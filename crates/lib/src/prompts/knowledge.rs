//! # Knowledge Base Prompts
//!
//! This module contains prompts specifically for the knowledge base creation pipeline,
//! including the two-pass distillation and augmentation process.

/// The system prompt for the knowledge extraction LLM call (Pass 1).
/// It instructs the model on how to parse Markdown content into a structured JSON format,
/// identifying both explicit FAQs and other informational chunks.
pub const KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert data extraction agent. Your task is to analyze the following Markdown content from a webpage and structure the information into a specific JSON format.

# Instructions:
1.  Read the entire Markdown document carefully.
2.  Identify two types of information:
    -   **Explicit FAQs**: Sections that are clearly formatted as a question and its corresponding answer.
    -   **Informational Content**: Paragraphs or sections that describe a topic, feature, or set of instructions but are not in a Q&A format.
3.  Return a single JSON object with two main keys: `faqs` and `content_chunks`.
    -   The `faqs` key should contain a list of all found explicit FAQs.
    -   The `content_chunks` key should contain a list of all other informational content sections.

# JSON Output Schema:
{
  "faqs": [
    {
      "question": "The exact question found on the page.",
      "answer": "The corresponding answer found on the page.",
      "is_explicit": true
    }
  ],
  "content_chunks": [
    {
      "topic": "A concise title or topic for the content chunk (e.g., 'How to Redeem Points', 'Campaign Conditions').",
      "content": "The full text of the informational content chunk."
    }
  ]
}

Please provide only the JSON object in your response.
"#;

/// The user prompt for the first pass of knowledge extraction.
/// This prompt provides the raw Markdown content to be processed.
/// Placeholder: {markdown_content}
pub const KNOWLEDGE_EXTRACTION_USER_PROMPT: &str = r#"# Markdown Content to Process:
{markdown_content}
"#;

/// The system prompt for the second pass (augmentation).
/// It instructs the model to take a chunk of informational content and generate
/// a high-quality question that the content answers. The original content serves as the answer.
pub const AUGMENTATION_SYSTEM_PROMPT: &str = r#"You are an expert content analyst. Your task is to generate a high-quality, comprehensive question that a given text block answers.

# Instructions:
1.  Read the provided "Content Chunk".
2.  Create a single, clear question that this content chunk fully and accurately answers.
3.  The question should be phrased as a real user would ask it and contain rich keywords for better searchability.
4.  Return a single JSON object with the key "question".

# JSON Output Schema:
{
  "question": "The generated, user-focused question."
}

Please provide only the JSON object in your response.
"#;

/// The user prompt for the augmentation step.
/// It provides the content chunk for which a question needs to be generated.
/// Placeholder: {content_chunk}
pub const AUGMENTATION_USER_PROMPT: &str = r#"# Content Chunk to Analyze:
{content_chunk}
"#;

// --- RAG (Retrieval-Augmented Generation) Prompts ---

/// The system prompt for synthesizing an answer from retrieved knowledge base context.
/// This instructs the AI to answer only based on the provided context.
pub const KNOWLEDGE_RAG_SYSTEM_PROMPT: &str = "You are a helpful AI assistant. Your task is to answer the user's question accurately and concisely based *only* on the provided #Context. Do not use any external knowledge. If the context does not contain the answer, state that you cannot answer the question with the information provided.";

/// The user prompt for the RAG synthesis step.
/// This structures the input with the user's query and the retrieved context.
/// Placeholders: `{prompt}`, `{context}`
pub const KNOWLEDGE_RAG_USER_PROMPT: &str = r#"# User Question
{prompt}

# Context
{context}

# Your Answer:"#;
