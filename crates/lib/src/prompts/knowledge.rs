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
3.  **Content Filtering Rule**: You **MUST** ignore navigation menus, lists of links, headers, footers, and any other non-substantive content. Focus exclusively on the main paragraphs and Q&A sections that provide real information.
4.  **Data Reconciliation Rule**: If you find conflicting information between the main content and a separate FAQ section (e.g., different dates, terms, conditions), you **MUST** prioritize the information from the main content body as the source of truth. The data in the FAQ section should be considered secondary if a conflict exists.
5.  **Crucial Language Rule**: You **MUST** preserve the original language of the content. Do NOT translate any of the text. For example, if the content is in Thai, all extracted `question`, `answer`, and `content` fields in your JSON response MUST be in Thai.
6.  Return a single JSON object with two main keys: `faqs` and `content_chunks`.
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

Please provide only the JSON object in your response. Do not add any extra text or explanations.
"#;

// --- Metadata Extraction ---

/// System prompt for extracting structured metadata (Entities and Keyphrases) from content.
pub const METADATA_EXTRACTION_SYSTEM_PROMPT: &str = r#"You are an expert document analyst. Your task is to analyze the following text and extract two types of metadata: **Entities** and **Keyphrases**.

# Instructions:
1.  **Entities**: Identify specific, proper nouns. These are unique, identifiable items like people, products, organizations, locations, or specific dates.
2.  **Keyphrases**: Identify the 5-10 most important thematic concepts or topics in the text. These should be broader than entities and capture the main ideas.
3.  **Language Rule**: You **MUST** generate the metadata in the same language as the original text. For example, if the content is in Thai, the metadata must be in Thai.
4.  Return a single JSON array of objects. Do not include any other text or explanations.

# JSON Object Schema:
Each object in the array must have the following keys:
- `type`: Must be either 'ENTITY' or 'KEYPHRASE'.
- `subtype`: For 'ENTITY', specify what it is (e.g., 'PERSON', 'PRODUCT', 'ORGANIZATION', 'CONCEPT'). For 'KEYPHRASE', use 'CONCEPT'.
- `value`: The extracted string value of the entity or keyphrase.

Please provide only the JSON object in your response.
"#;

// --- RAG (Retrieval-Augmented Generation) Prompts ---

/// The system prompt for synthesizing an answer from retrieved knowledge base context.
/// This instructs the AI to answer only based on the provided context.
pub const KNOWLEDGE_RAG_SYSTEM_PROMPT: &str =
    "You are a strict, factual AI. Your sole purpose is to answer the user's question based *only* on the provided #Context.
# Rules
- **DO NOT** use introductory phrases like 'Based on the provided context...' or 'From the information given...'. Start the answer directly.
- If the user includes an additional instruction, you must follow it.
- If the context does not contain the answer, state that the information is not available in the provided context.";

/// The user prompt for the RAG synthesis step.
/// This structures the input with the user's query and the retrieved context.
/// The `{instruction}` placeholder will be empty if no instruction is provided, which is harmless.
/// Placeholders: `{prompt}`, `{instruction}`, `{context}`
pub const KNOWLEDGE_RAG_USER_PROMPT: &str = r#"# User Question
{prompt}
{instruction}

# Context
{context}
# Your Answer:
"#;

// --- Hybrid Search Prompts ---

/// The system prompt for the query analysis step in a hybrid search.
/// It instructs the model to extract entities and keyphrases from a user's query.
pub const QUERY_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert query analyst. Your task is to extract key **Entities** and **Keyphrases** from the user's query. Respond with a JSON object containing two keys: "entities" and "keyphrases", which should be arrays of strings. If none are found, provide empty arrays."#;

/// The user prompt for the query analysis step.
/// Placeholder: {prompt}
pub const QUERY_ANALYSIS_USER_PROMPT: &str = "USER QUERY:\n{prompt}";

// --- GitHub Example Search Prompts ---

/// The system prompt for the query analysis step in a GitHub example search.
/// It instructs the model to extract code-related entities and keyphrases.
pub const GITHUB_EXAMPLE_SEARCH_ANALYSIS_SYSTEM_PROMPT: &str = r#"You are an expert code search analyst. Your task is to extract key **Entities** (like function names, library names, specific variables) and **Keyphrases** (like 'how to connect', 'example of authentication') from the user's query about code examples. Respond with a JSON object containing two keys: "entities" and "keyphrases", which should be arrays of strings. If none are found, provide empty arrays."#;

/// The user prompt for the GitHub example search query analysis step.
/// Placeholder: {prompt}
pub const GITHUB_EXAMPLE_SEARCH_ANALYSIS_USER_PROMPT: &str = "USER QUERY:\n{prompt}";
