# `html` Crate

This crate provides a set of utilities for processing HTML content, focusing on cleaning and converting it into well-structured, clean Markdown. It's designed to be a simple but effective tool for web content extraction pipelines.

## Features

*   **HTML Tag Stripping**: Removes unwanted HTML tags like `<script>`, `<style>`, `<meta>`, and `<link>` to isolate the core content. This is configurable, allowing you to specify which tags to remove.
*   **HTML to Markdown Conversion**: Converts HTML content into Markdown format.
*   **Automatic Title Extraction**: Intelligently finds the content of the `<title>` tag in an HTML document and prepends it to the final Markdown output as a level 1 header (e.g., `# Page Title`).
*   **Markdown Cleaning**: Post-processes the converted Markdown to remove common artifacts, navigational text (like "Menu" or "Contact Us"), and excessive newlines, resulting in clean, readable content.
*   **URL Fetching**: Includes asynchronous functions to fetch content directly from a URL and run it through the conversion pipeline.

## Usage

### Converting HTML to Clean Markdown

The primary function is `html_to_clean_markdown`, which handles cleaning and conversion in one step.

```rust
use html::html_to_clean_markdown;

let html_content = r#"
<html>
    <head>
        <title>My Awesome Page</title>
        <style>body { font-family: sans-serif; }</style>
    </head>
    <body>
        <p>This is the main content of the page.</p>
        <script>console.log("This will be removed.");</script>
    </body>
</html>
"#;

let markdown = html_to_clean_markdown(html_content, None);

// The expected output will have the title as an H1 header
// and the script tag will be removed.
let expected = "# My Awesome Page\n\nThis is the main content of the page.";

assert_eq!(markdown.trim(), expected);
```

### Cleaning HTML

If you only need to remove specific tags without converting to Markdown, you can use the `clean_html` function.

```rust
use html::clean_html;

let html_content = "<p>Keep this.</p><script>Remove this.</script>";

// Remove default tags (`script`, `style`, etc.)
let cleaned = clean_html(html_content, None);
assert_eq!(cleaned, "<p>Keep this.</p>");

// Specify custom tags to remove
let cleaned_custom = clean_html(html_content, Some(&["p"]));
assert_eq!(cleaned_custom, "<script>Remove this.</script>");
```

### Fetching and Converting from a URL

You can fetch and convert content directly from a URL using `url_to_clean_markdown`.

```rust
use html::url_to_clean_markdown;

async fn get_content() {
    let url = "http://example.com";
    match url_to_clean_markdown(url, None).await {
        Ok(markdown) => println!("{}", markdown),
        Err(e) => eprintln!("Failed to fetch and convert: {}", e),
    }
}
```
