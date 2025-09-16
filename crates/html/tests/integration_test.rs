//! # HTML Crate Integration Tests
//!
//! This file contains integration tests for the `html` crate, verifying
//! functionalities like HTML cleaning, Markdown conversion, and URL fetching.

#[cfg(test)]
mod tests {
    use html::{clean_html, html_to_clean_markdown, url_to_md};

    #[test]
    fn test_clean_html() {
        let html_content = r#"
        <html>
            <head>
                <title>Test</title>
                <style>body { color: red; }</style>
                <script>alert("hello");</script>
                <link rel="stylesheet" href="style.css">
            </head>
            <body>
                <h1>Hello</h1>
                <p>This is a test.</p>
                <meta name="author" content="Test">
            </body>
        </html>
        "#;

        // Test with default tags
        let cleaned_default = clean_html(html_content, None);
        assert!(!cleaned_default.contains("<style>"));
        assert!(!cleaned_default.contains("<script>"));
        assert!(!cleaned_default.contains("<link"));
        assert!(!cleaned_default.contains("<meta"));
        assert!(cleaned_default.contains("<h1>Hello</h1>"));

        // Test with custom tags
        let cleaned_custom = clean_html(html_content, Some(&["p", "h1"]));
        assert!(cleaned_custom.contains("<style>")); // Should not be removed
        assert!(!cleaned_custom.contains("<h1>Hello</h1>"));
        assert!(!cleaned_custom.contains("<p>This is a test.</p>"));

        // Test with no tags
        let cleaned_none = clean_html(html_content, Some(&[]));
        assert_eq!(cleaned_none, html_content);
    }

    #[tokio::test]
    async fn test_url_to_md() {
        // This is a simple integration test to ensure the full flow works.
        let url = "https://www.gpf.or.th/thai2019/10contact/main.php?page=7&menu=askfreq&lang=th&size=n&pattern=n";
        let result = url_to_md(url, None).await;
        assert!(result.is_ok(), "url_to_md failed: {:?}", result.err());
        let file_name = result.unwrap();
        assert!(
            std::path::Path::new(&file_name).exists(),
            "File '{file_name}' was not created"
        );

        // Check if content is somewhat reasonable (not empty)
        let content = std::fs::read_to_string(&file_name).unwrap();
        assert!(!content.is_empty());

        // cleanup
        // std::fs::remove_file(&file_name).unwrap();
    }
    #[test]
    fn test_html_to_markdown_with_title() {
        let html_content = "<html><head><title>My Page Title</title></head><body><p>Some content.</p></body></html>";
        let expected_markdown = "# My Page Title\n\nSome content.";

        let markdown = html_to_clean_markdown(html_content, None);
        assert_eq!(markdown.trim(), expected_markdown);
    }
}
