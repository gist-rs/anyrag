//! # Knowledge Base Prompts
//!
//! This module contains prompts specifically for the knowledge base creation pipeline,
//! including the two-pass distillation and augmentation process.

/// The system prompt for the knowledge extraction LLM call (Pass 1).
/// It instructs the model on how to parse Markdown content into a structured JSON format,
/// identifying both explicit FAQs and other informational chunks. It also tells the model how to handle conflicting information.
pub const KNOWLEDGE_EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert data extraction and reconciliation agent. Your task is to analyze the following Markdown content from a webpage and structure the information into a specific JSON format.

# Instructions:
1.  Read the entire Markdown document carefully.
2.  Identify two types of information:
    -   **Explicit FAQs**: Sections that are clearly formatted as a question and its corresponding answer, often under a "FAQ" or "คำถามที่พบบ่อย" heading.
    -   **Informational Content**: Paragraphs or sections that describe a topic, feature, or set of instructions but are not in a Q&A format. This is the main content of the document.
3.  **Data Reconciliation Rule**: If you find conflicting information between the main content and a separate FAQ section (e.g., different dates, terms, conditions), you **MUST** prioritize the information from the main content body as the source of truth. The data in the FAQ section should be considered secondary if a conflict exists.
4.  Return a single JSON object with two main keys: `faqs` and `content_chunks`.
    -   The `faqs` key should contain a list of all found explicit FAQs.
    -   The `content_chunks` key should contain a list of all other informational content sections.

# JSON Output Schema Example:
{
  "faqs": [
    {
      "question": "First question found on the page.",
      "answer": "The corresponding answer.",
      "is_explicit": true
    },
    {
      "question": "Second question found on the page.",
      "answer": "The second answer.",
      "is_explicit": true
    }
  ],
  "content_chunks": [
    {
      "topic": "Topic for the first informational chunk.",
      "content": "The full text of the first informational content chunk."
    },
    {
      "topic": "Topic for the second informational chunk.",
      "content": "The full text of the second informational content chunk."
    }
  ]
}

Please provide only the JSON object in your response.
"#;

/// The user prompt for the first pass of knowledge extraction.
/// This prompt is no longer used directly in the final version but is kept for reference.
/// Placeholder: {markdown_content}
pub const KNOWLEDGE_EXTRACTION_USER_PROMPT: &str = r#"# Markdown Content to Process:
{markdown_content}
"#;

/// The system prompt for the second pass (augmentation).
/// It instructs the model to take a batch of informational content chunks and generate
/// a high-quality question for each one.
pub const AUGMENTATION_SYSTEM_PROMPT: &str = r#"You are an expert content analyst. Your task is to generate a high-quality, comprehensive question for EACH of the provided text blocks (Content Chunks).

# Instructions:
1.  Analyze each "Content Chunk" provided in the input. Each chunk is clearly separated and has a unique integer ID.
2.  For each chunk, create a single, clear question that the content fully and accurately answers.
3.  The question should be phrased as a real user would ask it and must contain rich keywords for better searchability.
4.  **Language Rule**: You **MUST** generate the question in the same language as the original 'Content Chunk'. For example, if the content is in Thai, the question must be in Thai.
5.  Return a single JSON object containing a list named `augmented_faqs`. Each item in the list must correspond to one of the input chunks.

# JSON Output Schema:
{
  "augmented_faqs": [
    {
      "id": <The integer ID of the original content chunk>,
      "question": "The generated, user-focused question for that chunk."
    }
  ]
}

# Example:
## INPUT:
---
ID: 0
TOPIC: Campaign Conditions
CONTENT:
The campaign runs from July 1, 2025 to December 31, 2025. All users with the latest app version are eligible.
---
ID: 1
TOPIC: วิธีการลงทะเบียน
CONTENT:
หากต้องการลงทะเบียน ให้ไปที่หน้าแคมเปญในแอปแล้วกดปุ่ม "ลงทะเบียน"
---

## EXPECTED JSON OUTPUT:
{
  "augmented_faqs": [
    {
      "id": 0,
      "question": "What are the conditions and duration of the 2025 campaign?"
    },
    {
      "id": 1,
      "question": "ขั้นตอนการลงทะเบียนแคมเปญผ่านแอปต้องทำอย่างไร?"
    }
  ]
}

Please provide only the JSON object in your response. Do not add any extra text or explanations.
"#;

// --- RAG (Retrieval-Augmented Generation) Prompts ---

/// The system prompt for synthesizing an answer from retrieved knowledge base context.
/// This instructs the AI to answer only based on the provided context.
pub const KNOWLEDGE_RAG_SYSTEM_PROMPT: &str = "You are a strict, factual AI. Your sole purpose is to answer the user's question based *only* on the provided #Context. You MUST NOT use any external knowledge or make assumptions. If the context does not contain the information needed to answer the question, you MUST state that you cannot answer. Synthesize the information from the context into a concise and accurate answer, but do not add any information that is not explicitly present.";

/// The user prompt for the RAG synthesis step.
/// This structures the input with the user's query and the retrieved context.
/// Placeholders: `{prompt}`, `{context}`
pub const KNOWLEDGE_RAG_USER_PROMPT: &str = r#"# User Question
{prompt}

# Context
{context}

# Your Answer:"#;
