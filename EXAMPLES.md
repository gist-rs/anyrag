# API Usage Examples

This document provides detailed examples for the `anyrag-server` API endpoints. All examples use `curl` for demonstration purposes.

## Authentication

For endpoints that require authentication, you will need to replace `<your_jwt>` with a valid JSON Web Token. If you do not provide an `Authorization` header, your request will be processed as the "Guest User."

```sh
# Example with authentication
curl -X POST http://localhost:9090/some/endpoint \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{...}'
```

## Knowledge Base Management API

These endpoints are for building and maintaining the self-improving knowledge base.

### `POST /ingest/web`

Fetches and processes content from a web URL.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, runs the full AI-based pipeline to distill the content into structured Q&A pairs. Defaults to `false`.
- `embed` (boolean, optional): If `true` (default), generates and stores vector embeddings for the ingested content.

**Request Body:** `{"url": "https://..."}`

**Example (Light Ingest - Just store the content):**
```sh
curl -X POST http://localhost:9090/ingest/web \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

**Example (Full FAQ Generation):**
```sh
curl -X POST "http://localhost:9090/ingest/web?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://www.true.th/betterliv/support/true-app-mega-campaign"
  }'
```

### `POST /ingest/rss`

Ingests articles from an RSS feed URL, storing each item as a separate document.

**Request Body:** `{"url": "https://..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/rss \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "http://example.com/feed.xml"
  }'
```

### `POST /ingest/pdf`

Processes a PDF from either a direct file upload or a URL.

**Query Parameters:**
- `faq`, `embed` (see `/ingest/web`).

**Request Body:** `multipart/form-data` containing either a `file` or a `url`.
- `extractor`: (Optional) Can be `"local"` (default) or `"gemini"`.

**Example (File Upload):**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "file=@/path/to/your/document.pdf" \
  -F "extractor=local"
```

**Example (From URL):**
```sh
curl -X POST "http://localhost:9090/ingest/pdf?faq=true" \
  -H "Authorization: Bearer <your_jwt>" \
  -F "url=https://arxiv.org/pdf/2403.05530.pdf" \
  -F "extractor=local"
```

### `POST /ingest/sheet`

Ingests data from a public Google Sheet.

**Query Parameters:**
- `faq` (boolean, optional): If `true`, ingests a sheet formatted with "Question" and "Answer" columns as Q&A pairs. If `false` (default), ingests it as a generic table.

**Request Body:** `{"url": "...", "gid": "...", "skip_header": true}`

**Example (Generic Table Ingest):**
```sh
curl -X POST http://localhost:9090/ingest/sheet \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit"
  }'
```

**Example (FAQ Ingest):**
```sh
curl -X POST "http://localhost:9090/ingest/sheet?faq=true" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "url": "https://docs.google.com/spreadsheets/d/your_sheet_id/edit",
    "gid": "856666263"
  }'
```

### `POST /ingest/text`

Ingests raw text directly.

**Query Parameters:**
- `faq` (boolean, optional): If `false` (default), the text is automatically chunked.

**Request Body:** `{"text": "...", "source": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/text \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "text": "This is the first document about Rust macros.\n\nThis is a second paragraph about the same topic.",
    "source": "rust_docs_macros"
  }'
```

### `POST /embed/new`

Generates vector embeddings for all unembedded documents.

**Request Body:** `{"limit": 100}` (Optional)

**Example:**
```sh
curl -X POST http://localhost:9090/embed/new \
  -H "Content-Type: application/json" \
  -d '{"limit": 50}'
```

### `GET /knowledge/export`

Exports the FAQ knowledge base into a JSONL file suitable for fine-tuning.

**Example:**
```sh
curl http://localhost:9090/knowledge/export -o finetuning_dataset.jsonl
```

## RAG & Search API

### `POST /search/knowledge`

**This is the primary RAG endpoint.** It takes a user's question, performs a hybrid search, and synthesizes a final answer.

**Request Body:** `{"query": "...", "limit": 5, "instruction": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/search/knowledge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_jwt>" \
  -d '{
    "query": "ทำยังไงถึงจะได้เทสล่า",
    "instruction": "สรุปเงื่อนไขการรับสิทธิ์ลุ้นเทสล่า"
  }'
```

## Advanced API

### `POST /prompt`

Translates a natural language prompt into a query, executes it, and formats the result.

**Example: Basic Query**
```sh
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "tell me a joke"
  }'
```

**Example: Shorthand Query**
```sh
# This shorthand...
curl -X POST http://localhost:9090/prompt \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "prompt": "ls pantip_topics_samples limit=20"
  }'
# ...is automatically translated into a full SQL query for the AI.
```

### `POST /db/query`

Executes a raw, read-only SQL query directly against a project's database.

**Request Body:** `{"db": "...", "query": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/db/query \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "query": "SELECT _id, title, rating FROM pantip_topics_samples WHERE rating >= 3 ORDER BY rating DESC LIMIT 10"
  }'
```

### `POST /gen/text`

A powerful two-step generation endpoint. It first runs a `context_prompt` to retrieve data, then uses that data as context for a `generation_prompt`.

**Example 1:**
```sh
curl -X POST 'http://localhost:9090/gen/text?debug=false' \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "model": "gemini-2.5-pro",
    "generation_prompt": "User Goal: Generate a Pantip-style post consisting of a title and a short, romantic story in the style of a modern Thai drama. The output must be in JSON format: {\"title\": \"...\", \"topic_detail\": \"...\"}. The topic_detail must be the story in Thai language (ภาษาไทย) only, maybe start with something like \"ตามหัวกท.เลยค่ะ สงสัยมาก\" (กท. stand for topic), told from a first-person perspective using \"ผม\" (male) or \"เรา\" (female) to make it feel personal and intimate. Aim for 400-600 characters in the topic_detail to keep it concise yet engaging.\n\nThe story should feel authentic and raw, like a real personal anecdote shared on an online forum such as Pantip. Incorporate everyday casual language, Thai slang (e.g., ''อะ'', ''ง่ะ'', ''เลยอะ'', ''ว่ะ'', ''ซิ่'', ''อ้าว'', ''เฮ้อ'', ''5555''), emojis (e.g., 😭, 😂, 🥺, 🤣), emotional confessions, twists, and reflections that mirror real-life relationship struggles. Avoid overly dramatic or scripted dialogue; make it conversational and heartfelt, as if the narrator is venting or sharing their story online. Focus on one main theme to keep it coherent, such as unexpected love leading to personal growth despite financial hardships, with a bittersweet or uplifting ending that includes hope or reflection. Vary the themes to include positive, heartwarming elements alongside struggles, avoiding repetitive negative tropes like gambling betrayal; blend in elements of tenderness, sacrifice, or redemption for balance. Optional end the topic_detail with 1-2 subtle open-ended questions or reflections to encourage comments and engagement, like pondering opinions or similar experiences in a natural, non-direct way e.g. \"เราคิดว่างั้นค่ะ เคยเท่าที่เจอมา\" instead of \"จริงมั้ยคะ?\".\n\nKey Elements to Include:\n- Romantic Theme: Focus on a bittersweet or positive romance involving themes like unexpected love, financial hardships in relationships, jealousy, unrequited feelings, or personal growth through love. Ensure it is romantic with moments of tenderness amid struggles, and incorporate variety to avoid similarity in outputs.\n- First-Person Perspective: Use \"ผม\" for a male narrator or \"เรา\" for a female narrator to add authenticity, sharing inner thoughts, regrets, and hopes.\n- Modern Thai Drama Style: Include elements like urban settings (e.g., Bangkok nightlife, apartments, workplaces), family pressures, social media influences, and emotional highs/lows typical in Thai series (e.g., love triangles, sacrifices, redemptions). Do not list too many problems; focus on 1-2 key conflicts for depth.\n\nEmphasize creating a focused narrative with romantic elements, drawing from real-life anecdotes like unexpected encounters in nightlife leading to deep connections, financial struggles testing love, and personal reflections on growth. Make the title short, avoiding clichés and ensuring it fits the story''s tone positively or thoughtfully but more real human expression not a book title, less drama and more realistic forums topic, .\n\nOutput exactly in the specified JSON format, with no additional text.",
    "context_prompt": "Use top ten `rating` stories where the `topic_detail` contains `รัก`,`แฟน`,`อกหัก`,`เหงา`,`ใจ` in the `pantip_topics_samples` table as inspiration."
  }'
```

**Example 2:**
```sh
curl -X POST 'http://localhost:9090/gen/text?debug=false' \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "model": "gemini-2.5-pro",
    "generation_prompt": "User Goal: Generate a Pantip-style post in Thai language.",
    "context_prompt": "ความรักที่ไม่สมหวังซ้ำๆ ซากๆ"
  }'
```

### `POST /ingest/firebase`

Triggers a server-side dump of a Firestore collection into the local SQLite database.

**Request Body:** `{"project_id": "...", "collection": "...", ...}`

**Example:**
```sh
curl -X POST http://localhost:9090/ingest/firebase \
  -H "Content-Type: application/json" \
  -d '{
    "project_id": "kratooded",
    "collection": "pantip_topics_samples"
  }'
```

### `POST /graph/build`

Builds or updates the in-memory Knowledge Graph from a specified table.

**Request Body:** `{"db": "...", "table_name": "..."}`

**Example:**
```sh
curl -X POST http://localhost:9090/graph/build \
  -H "Content-Type: application/json" \
  -d '{
    "db": "kratooded",
    "table_name": "pantip_topics_samples"
  }'
```
