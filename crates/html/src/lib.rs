use regex::Regex;
use scraper::{Html, Selector};
use std::error::Error;
use std::fmt;
use std::fs;

/// Cleans specified HTML tags from a string.
///
/// # Arguments
///
/// * `html` - The HTML content as a string.
/// * `remove_tags` - An optional slice of HTML tags to remove.
///   If `None`, a default set of tags (`script`, `style`, `meta`, `link`) will be removed.
///
/// # Returns
///
/// A `String` with the specified HTML tags removed.
pub fn clean_html(html: &str, remove_tags: Option<&[&str]>) -> String {
    let default_tags = &["script", "style", "meta", "link", "a", "img"];
    let tags_to_remove = remove_tags.unwrap_or(default_tags);

    let mut cleaned_html = html.to_string();
    for tag in tags_to_remove {
        // This regex handles both block tags (<script>...</script>) and self-closing/simple tags (<meta>, <link>).
        let re = Regex::new(&format!(r"(?is)<{tag}[^>]*>.*?</{tag}>|<{tag}[^>]*>")).unwrap();
        cleaned_html = re.replace_all(&cleaned_html, "").to_string();
    }
    cleaned_html
}

/// Converts raw HTML to cleaned Markdown in a single step.
///
/// This function first cleans the HTML by removing specified tags, then converts the
/// result to Markdown, and finally cleans the resulting Markdown to remove
/// common artifacts.
///
/// # Arguments
///
/// * `html` - The raw HTML content to convert.
/// * `remove_tags` - An optional slice of HTML tags to remove before conversion.
///
/// # Returns
///
/// A `String` containing the cleaned Markdown.
pub fn html_to_clean_markdown(html: &str, remove_tags: Option<&[&str]>) -> String {
    let cleaned_html = clean_html(html, remove_tags);

    // Check if a title exists in the original HTML.
    let document = Html::parse_document(&cleaned_html);
    let title_selector = Selector::parse("title").unwrap();
    let title_exists = document.select(&title_selector).next().is_some();

    let markdown = html2md::parse_html(&cleaned_html);
    let cleaned_markdown = clean_markdown_content(&markdown);

    // If a title existed, format the first line of the output as a Markdown H1 header.
    if title_exists {
        if let Some((first_line, rest)) = cleaned_markdown.split_once('\n') {
            if !first_line.trim().is_empty() {
                // Prepend '#' to the first line and recombine with the rest.
                return format!("# {}\n{}", first_line.trim(), rest);
            }
        } else if !cleaned_markdown.trim().is_empty() {
            // Handle case where there's only a single line of text (the title).
            return format!("# {}", cleaned_markdown.trim());
        }
    }

    cleaned_markdown
}

/// Cleans aggressively fetched markdown content by removing common navigational
/// elements, symbols, and artifacts left over from HTML conversion.
pub fn clean_markdown_content(markdown: &str) -> String {
    // This regex matches lines that contain only a combination of symbols (`*`, `|`, `-`) and whitespace.
    let symbol_line_re = Regex::new(r"^\s*([*|-]\s*)+\s*$").unwrap();

    // This regex matches common navigational keywords that often appear on their own lines,
    // potentially surrounded by asterisks for markdown bolding. It's case-insensitive `(?i)`.
    let nav_keywords_re = Regex::new(r#"(?i)^\s*\**\s*(menu|เมนู|คำถามพบบ่อย|ติดต่อเรา|เกี่ยวกับ กบข.|บริการสมาชิก|ลงทุน|ข่าวสารและกิจกรรม|รายงานผลการดำเนินงาน|การบริหารความเสี่ยง|สถิติสำคัญ|สัดส่วนการลงทุน|นโยบายการกำกับดูแลกิจการ|การลงทุนอย่างรับผิดชอบ|การดำเนินการต่อต้านการทุจริต|มาตรการภายในเพื่อส่งเสริมความโปร่งใสและป้องกันการทุจริต|การประเมิน ITA|ตำแหน่งที่เปิดรับ|กรอกใบสมัคร|ประกาศจัดซื้อ-จัดจ้าง|สรุปผลการจัดซื้อ-จัดจ้าง|วิเคราะห์ผลการจัดซื้อจัดจ้าง|ความก้าวหน้าการจัดซื้อจัดจ้าง|การขึ้นทะเบียนคู่ค้า|ประกาศจำหน่ายทรัพย์สิน|จัดซื้อ-จัดจ้างอาคารอับดุลราฮิม เพลส|MCS Web|แบบฟอร์ม|งาน กบข.|กิจกรรมต่าง ๆ|My GPF & My GPF Twins|﻿)\s*\**\s*$"#).unwrap();

    // This regex matches the copyright footer.
    let footer_re = Regex::new(r"(?i)^\s*© สงวนลิขสิทธิ์.*").unwrap();

    // This regex is for collapsing more than two consecutive newlines into just two.
    let multi_newline_re = Regex::new(r"\n{3,}").unwrap();

    let cleaned_content = markdown
        .lines()
        .filter(|line| !symbol_line_re.is_match(line.trim()))
        .filter(|line| !nav_keywords_re.is_match(line.trim()))
        .filter(|line| !footer_re.is_match(line.trim()))
        .collect::<Vec<&str>>()
        .join("\n");

    // After joining, collapse any large gaps of whitespace.
    // Also, trim the final output to remove leading/trailing newlines.
    multi_newline_re
        .replace_all(&cleaned_content, "\n\n")
        .trim()
        .to_string()
}

/// Fetches HTML from a URL, cleans specified tags, converts it to Markdown,
/// and saves it to a file named after the MD5 hash of the URL.
///
/// # Arguments
///
/// * `url` - The URL to fetch the HTML from.
/// * `remove_tags` - An optional slice of HTML tags to remove before conversion.
///   If `None`, a default set of tags will be used.
///
/// # Returns
///
/// A `Result` containing the name of the created Markdown file, or an error.
pub async fn url_to_md(
    url: &str,
    remove_tags: Option<&[&str]>,
) -> Result<String, Box<dyn std::error::Error>> {
    let html_raw = reqwest::get(url).await?.text().await?;
    let cleaned_html = clean_html(&html_raw, remove_tags);
    let md = html2md::parse_html(&cleaned_html);
    let cleaned_md = clean_markdown_content(&md);

    let digest = md5::compute(url.as_bytes());
    let file_name = format!("{digest:x}.md");
    fs::write(&file_name, cleaned_md)?;
    Ok(file_name)
}

/// Fetches a URL and converts its HTML content to cleaned Markdown.
///
/// This function handles the HTTP request, checks for success, and then uses the
/// `html_to_clean_markdown` function to process the response body.
///
/// # Arguments
///
/// * `url` - The URL to fetch.
/// * `remove_tags` - An optional slice of HTML tags to remove during cleaning.
///
/// # Returns
///
/// A `Result` containing the cleaned Markdown `String`, or a `FetchError`.

#[derive(Debug)]
pub enum FetchError {
    Status { status: u16, body: String },
    Request(reqwest::Error),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::Status { status, body } => {
                write!(f, "Request failed with status {status}: {body}")
            }
            FetchError::Request(e) => write!(f, "Request failed: {e}"),
        }
    }
}

impl Error for FetchError {}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> FetchError {
        FetchError::Request(err)
    }
}

pub async fn url_to_clean_markdown(
    url: &str,
    remove_tags: Option<&[&str]>,
) -> Result<String, FetchError> {
    if url.ends_with(".md") {
        let response = reqwest::get(url).await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(FetchError::Status { status, body });
        }
        let markdown = response.text().await?;
        return Ok(clean_markdown_content(&markdown));
    }

    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(FetchError::Status { status, body });
    }
    let html_raw = response.text().await?;
    Ok(html_to_clean_markdown(&html_raw, remove_tags))
}
