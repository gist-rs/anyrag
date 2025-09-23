# MEMORAG_PLAN.md: The Evolution of Anyrag's Knowledge Base

This document outlines the vision and implementation plan for the next phase of `anyrag`'s development. We will evolve the current system from a static, document-oriented "knowledge bookshelf" into a dynamic, self-consolidating "memory stream," inspired by the concepts of MemoRag.

## 1. The Vision: From Bookshelf to Active Memory

Currently, `anyrag` excels at ingesting discrete documents and enabling search across them. This is like having a perfectly indexed library; you can find the right book, but you still have to read it to synthesize an answer.

The next evolution is to create a system that actively reads its own books, understands them, and writes new, more concise summary books. This transforms the knowledge base from a passive repository into an active, evolving memory that gets smarter and more efficient over time.

## 2. The Core Component: The `anyrag-curator`

To achieve this vision, we will introduce a new architectural component: the **Curator**.

The Curator will be a background service or a scheduled task (e.g., a new `anyrag-cli` command) that acts as the system's "memory editor." Its primary function is to perform **automated knowledge synthesis**.

### How the Curator Works:

1.  **Scan:** The Curator will periodically scan the `documents` table, selecting a group of related memories to process. This selection could be based on a shared source URL, a specific topic, or a date range (e.g., "all meeting notes from the past week").
2.  **Synthesize:** It will feed the content of these related documents into an LLM with a higher-level prompt. Examples of synthesis prompts include:
    *   *"Analyze these different versions of the same product page and create a single, up-to-date summary of the product's current features and price."*
    *   *"Read all these weekly project updates and generate a concise summary of the key decisions and blockers."*
    *   *"Identify and resolve any conflicting information found across these documents."*
3.  **Ingest:** The output from the LLM—a new, synthesized piece of knowledge—will be ingested back into the `documents` table as a *new memory*. This new memory is a higher-level insight derived from the raw, underlying facts.

This creates a virtuous cycle where the knowledge base not only grows but actively improves its own density and accuracy.

## 3. The Proof: A Test for Smarter RAG

To prove that this new approach is superior, we will create a specific, end-to-end test that highlights the failure mode of the current system and the success of the new, curated approach.

### The Test Scenario: Handling Evolving Information

The test will simulate the common real-world scenario of ingesting a webpage whose content changes over time.

**A. The Setup:**

1.  **Ingest Day 1:** Ingest a mock product page for "WidgetPro" that states the price is **$99**.
2.  **Ingest Day 2:** Ingest an updated version of the same page where the price has changed to **$119**.
3.  **Ingest Day 3:** Ingest a final version of the page where the price is now **$129**.

**B. The Baseline Test (Current System):**

1.  Ask the RAG system: `"What is the price of WidgetPro?"`
2.  **Expected Failure:** The hybrid search will retrieve all three versions of the page because they are all highly relevant to "WidgetPro". The context provided to the final LLM will be confusing and contradictory (e.g., "...the price is $99... the price is $119... the price is $129...").
3.  **Assertion:** The test will assert that the final AI-generated answer is ambiguous, likely containing multiple prices or a vague statement like "The price has been listed as $99, $119, and $129."

**C. The MemoRag Test (New System):**

1.  **Run the Curator:** After the three ingestions, trigger the Curator service. The Curator's task will be to process all documents with the source URL of the "WidgetPro" page. Its prompt will be: *"Synthesize a single, definitive summary of the current state of this product based on the provided versions, prioritizing the most recent information."*
2.  **Synthesized Memory:** The Curator will use an LLM to generate a new document, which it will ingest back into the knowledge base. This new document will say something like: *"As of Day 3, the current price for WidgetPro is $129."*
3.  **Ask the RAG system:** Ask the same question again: `"What is the price of WidgetPro?"`
4.  **Expected Success:** The hybrid search will now retrieve the new, synthesized summary document as the top result. It is the most concise, relevant, and up-to-date answer.
5.  **Assertion:** The test will assert that the final AI-generated answer is a single, correct, and confident statement: **"The price of WidgetPro is $129."**

This test will provide concrete, undeniable proof that the Curator's ability to consolidate and synthesize information leads to a fundamentally more intelligent and accurate RAG system.