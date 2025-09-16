pub fn add(left: usize, right: usize) -> usize {
    left + right
}

use std::fs;

pub async fn url_to_md(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let resp = reqwest::get(url).await?.text().await?;
    let md = html2md::parse_html(&resp);
    let digest = md5::compute(url.as_bytes());
    let file_name = format!("{digest:x}.md");
    fs::write(&file_name, md)?;
    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_url_to_md() {
        let url = "https://www.gpf.or.th/thai2019/10contact/main.php?page=7&menu=askfreq&lang=th&size=n&pattern=n";
        let result = url_to_md(url).await;
        assert!(result.is_ok(), "url_to_md failed: {:?}", result.err());
        let file_name = result.unwrap();
        assert!(
            std::path::Path::new(&file_name).exists(),
            "File '{file_name}' was not created"
        );
        // cleanup
        // std::fs::remove_file(&file_name).unwrap();
    }
}
